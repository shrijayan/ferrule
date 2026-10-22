/**
 * A pure, deterministic evaluator for the JSONPath Plus subset that `openapi.transforms` uses.
 *
 * Supported: `$`, `.name`, `..name`, `.*`, `..*`, `['name']`, `["name"]`, `[0]`, `[-1]`,
 * `[*]`, unions (`['a','b']`, `[0,2]`), slices (`[0:2]`, `[::2]`) and filters
 * (`[?(@.name == "id")]`). Filters accept `@`, `@property`, `@parentProperty`, `@parent`,
 * `@root`/`$`, member access, the literals JSON has plus `undefined` and `/regex/flags`,
 * `! && || == === != !== < <= > >=`, parentheses, and the string/array methods `includes`,
 * `startsWith`, `endsWith`, `match`, `test`, `toLowerCase`, `toUpperCase` and `trim`.
 * Dot names run to the next `.` or `[`, so `$.paths./users/{id}.get` works as in JSONPath Plus.
 *
 * Anything else (script expressions, `^` parent, `~` property names, type selectors) throws a
 * `JsonPathError` so callers can turn it into a diagnostic instead of guessing.
 */

export type PathSegment = string | number;

/** A node selected by a query: its location (from the root) and its value. */
export interface Match {
  readonly path: readonly PathSegment[];
  readonly value: unknown;
}

export class JsonPathError extends Error {
  override readonly name = 'JsonPathError';
}

type Selector =
  | { readonly kind: 'name'; readonly name: string }
  | { readonly kind: 'index'; readonly index: number }
  | { readonly kind: 'wildcard' }
  | {
      readonly kind: 'slice';
      readonly start: number | undefined;
      readonly end: number | undefined;
      readonly step: number;
    }
  | { readonly kind: 'filter'; readonly expr: Expr };

export interface Segment {
  readonly descendant: boolean;
  readonly selectors: readonly Selector[];
}

type ContextName = 'current' | 'property' | 'parentProperty' | 'parent' | 'root';
const VARIABLES: Readonly<Record<string, ContextName>> = {
  '@': 'current',
  $: 'root',
  '@root': 'root',
  '@parent': 'parent',
  '@property': 'property',
  '@parentProperty': 'parentProperty',
};
type BinaryOp = '||' | '&&' | '==' | '===' | '!=' | '!==' | '<' | '<=' | '>' | '>=';
const METHODS = [
  'includes',
  'startsWith',
  'endsWith',
  'match',
  'test',
  'toLowerCase',
  'toUpperCase',
  'trim',
] as const;
type Method = (typeof METHODS)[number];

type Expr =
  | { readonly t: 'lit'; readonly v: unknown }
  | { readonly t: 'ctx'; readonly which: ContextName }
  | { readonly t: 'get'; readonly obj: Expr; readonly key: PathSegment }
  | { readonly t: 'call'; readonly obj: Expr; readonly method: Method; readonly args: Expr[] }
  | { readonly t: 'not'; readonly e: Expr }
  | { readonly t: 'bin'; readonly op: BinaryOp; readonly l: Expr; readonly r: Expr };

// ---------------------------------------------------------------------------------------------
// Parsing

class Parser {
  pos = 0;
  // A plain field, not a parameter property: node's type stripping (the bin) rejects those.
  readonly src: string;
  constructor(src: string) {
    this.src = src;
  }

  fail(message: string): never {
    throw new JsonPathError(`${message} at offset ${this.pos} in "${this.src}"`);
  }
  peek(n = 0): string {
    return this.src[this.pos + n] ?? '';
  }
  startsWith(s: string): boolean {
    return this.src.startsWith(s, this.pos);
  }
  eat(s: string): boolean {
    if (!this.startsWith(s)) return false;
    this.pos += s.length;
    return true;
  }
  expect(s: string): void {
    if (!this.eat(s)) this.fail(`Expected "${s}"`);
  }
  ws(): void {
    while (/\s/.test(this.peek())) this.pos++;
  }
  done(): boolean {
    return this.pos >= this.src.length;
  }

  path(): Segment[] {
    this.ws();
    if (!this.eat('$')) this.fail('A JSONPath must start with "$"');
    const segments: Segment[] = [];
    for (this.ws(); !this.done(); this.ws()) {
      if (this.eat('..')) {
        segments.push({ descendant: true, selectors: this.afterDot() });
      } else if (this.eat('.')) {
        segments.push({ descendant: false, selectors: this.afterDot() });
      } else if (this.peek() === '[') {
        segments.push({ descendant: false, selectors: this.bracket() });
      } else {
        this.fail(`Unexpected "${this.peek()}"`);
      }
    }
    return segments;
  }

  afterDot(): Selector[] {
    if (this.peek() === '[') return this.bracket();
    const start = this.pos;
    while (!this.done() && this.peek() !== '.' && this.peek() !== '[') this.pos++;
    const name = this.src.slice(start, this.pos).trimEnd();
    if (name === '') this.fail('Expected a property name');
    if (name === '*') return [{ kind: 'wildcard' }];
    if (name.endsWith('^') || name.endsWith('~') || name.startsWith('@') || name.startsWith('(')) {
      this.pos = start;
      this.fail(`Unsupported JSONPath Plus syntax "${name}"`);
    }
    return [{ kind: 'name', name }];
  }

  bracket(): Selector[] {
    this.expect('[');
    this.ws();
    if (this.eat('?')) {
      this.ws();
      const paren = this.eat('(');
      const expr = this.expr();
      this.ws();
      if (paren) this.expect(')');
      this.ws();
      this.expect(']');
      return [{ kind: 'filter', expr }];
    }
    if (this.peek() === '(') this.fail('Script expressions "[(...)]" are not supported');
    const selectors: Selector[] = [];
    do {
      this.ws();
      selectors.push(this.unionItem());
      this.ws();
    } while (this.eat(','));
    this.expect(']');
    return selectors;
  }

  unionItem(): Selector {
    const c = this.peek();
    if (c === "'" || c === '"') return { kind: 'name', name: this.string() };
    if (this.eat('*')) return { kind: 'wildcard' };
    if (/[-\d:]/.test(c)) {
      const start = this.optInt();
      if (!this.eat(':')) {
        if (start === undefined) this.fail('Expected an index');
        return { kind: 'index', index: start };
      }
      const end = this.optInt();
      const step = this.eat(':') ? (this.optInt() ?? 1) : 1;
      if (step === 0) this.fail('Slice step must not be 0');
      return { kind: 'slice', start, end, step };
    }
    const m = /^[A-Za-z_$][\w$-]*/.exec(this.src.slice(this.pos));
    if (m === null) this.fail('Expected a name, index, slice or filter');
    this.pos += m[0].length;
    return { kind: 'name', name: m[0] };
  }

  optInt(): number | undefined {
    this.ws();
    const m = /^-?\d+/.exec(this.src.slice(this.pos));
    if (m === null) return undefined;
    this.pos += m[0].length;
    this.ws();
    return Number(m[0]);
  }

  string(): string {
    const quote = this.peek();
    this.pos++;
    let out = '';
    for (;;) {
      if (this.done()) this.fail('Unterminated string');
      const c = this.peek();
      this.pos++;
      if (c === quote) return out;
      if (c !== '\\') {
        out += c;
        continue;
      }
      const e = this.peek();
      this.pos++;
      const simple: Record<string, string> = { n: '\n', t: '\t', r: '\r', b: '\b', f: '\f' };
      if (e === 'u') {
        const hex = this.src.slice(this.pos, this.pos + 4);
        if (!/^[0-9a-fA-F]{4}$/.test(hex)) this.fail('Invalid \\u escape');
        out += String.fromCharCode(Number.parseInt(hex, 16));
        this.pos += 4;
      } else {
        out += simple[e] ?? e;
      }
    }
  }

  // Filter expressions: precedence climbing over || && equality relational unary postfix.
  expr(): Expr {
    return this.binary(0);
  }

  binary(level: number): Expr {
    const levels: readonly (readonly BinaryOp[])[] = [
      ['||'],
      ['&&'],
      ['===', '!==', '==', '!='],
      ['<=', '>=', '<', '>'],
    ];
    const ops = levels[level];
    if (ops === undefined) return this.unary();
    let left = this.binary(level + 1);
    for (;;) {
      this.ws();
      const op = ops.find((o) => this.startsWith(o));
      if (op === undefined) return left;
      this.pos += op.length;
      left = { t: 'bin', op, l: left, r: this.binary(level + 1) };
    }
  }

  unary(): Expr {
    this.ws();
    if (this.peek() === '!' && this.peek(1) !== '=') {
      this.pos++;
      return { t: 'not', e: this.unary() };
    }
    return this.postfix(this.primary());
  }

  primary(): Expr {
    this.ws();
    const c = this.peek();
    if (this.eat('(')) {
      const e = this.expr();
      this.ws();
      this.expect(')');
      return e;
    }
    if (c === "'" || c === '"') return { t: 'lit', v: this.string() };
    if (c === '/') return { t: 'lit', v: this.regex() };
    const num = /^-?\d+(\.\d+)?([eE][-+]?\d+)?/.exec(this.src.slice(this.pos));
    if (num !== null) {
      this.pos += num[0].length;
      return { t: 'lit', v: Number(num[0]) };
    }
    const variable = /^[@$][A-Za-z]*/.exec(this.src.slice(this.pos))?.[0];
    if (variable !== undefined) {
      const which = VARIABLES[variable];
      if (which === undefined) this.fail(`Unsupported variable "${variable}"`);
      this.pos += variable.length;
      return { t: 'ctx', which };
    }
    const words: readonly [string, unknown][] = [
      ['true', true],
      ['false', false],
      ['null', null],
      ['undefined', undefined],
    ];
    for (const [word, v] of words) {
      if (this.startsWith(word) && !/\w/.test(this.peek(word.length) || ' ')) {
        this.pos += word.length;
        return { t: 'lit', v };
      }
    }
    return this.fail('Expected a filter operand');
  }

  regex(): RegExp {
    this.expect('/');
    let body = '';
    let inClass = false;
    for (;;) {
      if (this.done()) this.fail('Unterminated regular expression');
      const c = this.peek();
      this.pos++;
      if (c === '\\') {
        body += c + this.peek();
        this.pos++;
        continue;
      }
      if (c === '[') inClass = true;
      if (c === ']') inClass = false;
      if (c === '/' && !inClass) break;
      body += c;
    }
    const flags = /^[dgimsuvy]*/.exec(this.src.slice(this.pos))?.[0] ?? '';
    this.pos += flags.length;
    try {
      return new RegExp(body, flags);
    } catch (e) {
      return this.fail(`Invalid regular expression: ${(e as Error).message}`);
    }
  }

  postfix(base: Expr): Expr {
    let e = base;
    for (;;) {
      if (this.peek() === '.' && this.peek(1) !== '.') {
        this.pos++;
        const m = /^[A-Za-z_$][\w$-]*/.exec(this.src.slice(this.pos));
        if (m === null) this.fail('Expected a property name');
        this.pos += m[0].length;
        if (this.peek() === '(') {
          const method = METHODS.find((x) => x === m[0]);
          if (method === undefined) this.fail(`Unsupported method "${m[0]}"`);
          this.expect('(');
          const args: Expr[] = [];
          this.ws();
          if (!this.eat(')')) {
            do args.push(this.expr());
            while (this.eat(','));
            this.ws();
            this.expect(')');
          }
          e = { t: 'call', obj: e, method, args };
        } else {
          e = { t: 'get', obj: e, key: m[0] };
        }
      } else if (this.peek() === '[') {
        this.pos++;
        this.ws();
        const c = this.peek();
        let key: PathSegment;
        if (c === "'" || c === '"') key = this.string();
        else {
          const n = this.optInt();
          if (n === undefined) this.fail('Expected a string or index');
          key = n;
        }
        this.ws();
        this.expect(']');
        e = { t: 'get', obj: e, key };
      } else {
        return e;
      }
    }
  }
}

/** Parse a JSONPath. Throws `JsonPathError` on invalid or unsupported syntax. */
export function parseJsonPath(path: string): Segment[] {
  return new Parser(path).path();
}

// ---------------------------------------------------------------------------------------------
// Evaluation

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

interface Node {
  readonly path: readonly PathSegment[];
  readonly value: unknown;
}

interface Scope {
  readonly current: unknown;
  readonly property: PathSegment | undefined;
  readonly parent: unknown;
  readonly parentProperty: PathSegment | undefined;
  readonly root: unknown;
}

function children(node: Node): Node[] {
  const { value } = node;
  if (Array.isArray(value)) {
    return value.map((v, i) => ({ path: [...node.path, i], value: v }));
  }
  if (isRecord(value)) {
    return Object.keys(value).map((k) => ({ path: [...node.path, k], value: value[k] }));
  }
  return [];
}

function selfAndDescendants(node: Node, out: Node[]): void {
  out.push(node);
  for (const c of children(node)) selfAndDescendants(c, out);
}

function child(node: Node, key: PathSegment): Node | undefined {
  const { value } = node;
  if (Array.isArray(value)) {
    const raw = typeof key === 'number' ? key : /^\d+$/.test(key) ? Number(key) : undefined;
    if (raw === undefined) return undefined;
    const i = raw < 0 ? value.length + raw : raw;
    return i >= 0 && i < value.length ? { path: [...node.path, i], value: value[i] } : undefined;
  }
  if (isRecord(value) && typeof key === 'string' && Object.hasOwn(value, key)) {
    return { path: [...node.path, key], value: value[key] };
  }
  return undefined;
}

function slice(node: Node, s: Extract<Selector, { kind: 'slice' }>): Node[] {
  const { value } = node;
  if (!Array.isArray(value)) return [];
  const len = value.length;
  const norm = (n: number) => (n < 0 ? Math.max(len + n, 0) : Math.min(n, len));
  const out: Node[] = [];
  if (s.step > 0) {
    const end = s.end === undefined ? len : norm(s.end);
    for (let i = s.start === undefined ? 0 : norm(s.start); i < end; i += s.step) {
      out.push({ path: [...node.path, i], value: value[i] });
    }
  } else {
    const start = s.start === undefined ? len - 1 : Math.min(norm(s.start), len - 1);
    const end = s.end === undefined ? -1 : norm(s.end);
    const stride = -s.step;
    for (let i = start; i > end; i -= stride) {
      out.push({ path: [...node.path, i], value: value[i] });
    }
  }
  return out;
}

function apply(node: Node, selector: Selector, root: unknown, out: Node[]): void {
  switch (selector.kind) {
    case 'name': {
      const c = child(node, selector.name);
      if (c !== undefined) out.push(c);
      return;
    }
    case 'index': {
      const c = child(node, selector.index);
      if (c !== undefined) out.push(c);
      return;
    }
    case 'wildcard':
      out.push(...children(node));
      return;
    case 'slice':
      out.push(...slice(node, selector));
      return;
    case 'filter':
      for (const c of children(node)) {
        const scope: Scope = {
          current: c.value,
          property: c.path.at(-1),
          parent: node.value,
          parentProperty: node.path.at(-1),
          root,
        };
        if (truthy(evaluate(selector.expr, scope))) out.push(c);
      }
  }
}

function truthy(v: unknown): boolean {
  return Boolean(v);
}

function get(obj: unknown, key: PathSegment): unknown {
  if (Array.isArray(obj) || typeof obj === 'string') {
    if (key === 'length') return obj.length;
    const i = typeof key === 'number' ? key : /^\d+$/.test(key) ? Number(key) : undefined;
    return i === undefined ? undefined : obj[i < 0 ? obj.length + i : i];
  }
  if (isRecord(obj) && Object.hasOwn(obj, String(key))) return obj[String(key)];
  return undefined;
}

function call(obj: unknown, method: Method, args: unknown[]): unknown {
  const [a] = args;
  if (obj instanceof RegExp) {
    return method === 'test' && typeof a === 'string' ? obj.test(a) : undefined;
  }
  if (Array.isArray(obj)) return method === 'includes' ? obj.includes(a) : undefined;
  if (typeof obj !== 'string') return undefined;
  switch (method) {
    case 'includes':
      return typeof a === 'string' ? obj.includes(a) : undefined;
    case 'startsWith':
      return typeof a === 'string' ? obj.startsWith(a) : undefined;
    case 'endsWith':
      return typeof a === 'string' ? obj.endsWith(a) : undefined;
    case 'match':
      return a instanceof RegExp || typeof a === 'string' ? obj.match(a) : undefined;
    case 'toLowerCase':
      return obj.toLowerCase();
    case 'toUpperCase':
      return obj.toUpperCase();
    case 'trim':
      return obj.trim();
    default:
      return undefined;
  }
}

/** JavaScript `==` restricted to the primitive values JSON produces. */
function looseEquals(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return a == null && b == null;
  const prim = (v: unknown) =>
    typeof v === 'number' || typeof v === 'string' || typeof v === 'boolean';
  if (prim(a) && prim(b)) return Number(a) === Number(b);
  return false;
}

function compare(op: '<' | '<=' | '>' | '>=', a: unknown, b: unknown): boolean {
  const ok =
    (typeof a === 'number' && typeof b === 'number') ||
    (typeof a === 'string' && typeof b === 'string');
  if (!ok) return false;
  const [x, y] = [a as number | string, b as number | string];
  switch (op) {
    case '<':
      return x < y;
    case '<=':
      return x <= y;
    case '>':
      return x > y;
    default:
      return x >= y;
  }
}

function evaluate(e: Expr, scope: Scope): unknown {
  switch (e.t) {
    case 'lit':
      return e.v;
    case 'ctx':
      return scope[e.which];
    case 'get':
      return get(evaluate(e.obj, scope), e.key);
    case 'call':
      return call(
        evaluate(e.obj, scope),
        e.method,
        e.args.map((a) => evaluate(a, scope)),
      );
    case 'not':
      return !truthy(evaluate(e.e, scope));
    case 'bin': {
      if (e.op === '&&') return truthy(evaluate(e.l, scope)) && truthy(evaluate(e.r, scope));
      if (e.op === '||') return truthy(evaluate(e.l, scope)) || truthy(evaluate(e.r, scope));
      const l = evaluate(e.l, scope);
      const r = evaluate(e.r, scope);
      switch (e.op) {
        case '===':
          return l === r;
        case '!==':
          return l !== r;
        case '==':
          return looseEquals(l, r);
        case '!=':
          return !looseEquals(l, r);
        default:
          return compare(e.op, l, r);
      }
    }
  }
}

export function pathKey(path: readonly PathSegment[]): string {
  return JSON.stringify(path);
}

/** Evaluate parsed segments against `root`. Results are de-duplicated, in selection order. */
export function evaluateSegments(root: unknown, segments: readonly Segment[]): Match[] {
  let nodes: Node[] = [{ path: [], value: root }];
  for (const segment of segments) {
    const next: Node[] = [];
    for (const node of nodes) {
      const bases: Node[] = [];
      if (segment.descendant) selfAndDescendants(node, bases);
      else bases.push(node);
      for (const base of bases) {
        for (const selector of segment.selectors) apply(base, selector, root, next);
      }
    }
    nodes = next;
  }
  const seen = new Set<string>();
  const out: Match[] = [];
  for (const n of nodes) {
    const key = pathKey(n.path);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({ path: n.path, value: n.value });
  }
  return out;
}

/** Parse and evaluate `path` against `root`. Throws `JsonPathError` on bad syntax. */
export function query(root: unknown, path: string): Match[] {
  return evaluateSegments(root, parseJsonPath(path));
}
