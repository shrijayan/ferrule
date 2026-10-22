/**
 * A small JSON Schema (draft 2020-12) validator for exactly the keywords the public
 * stainless.yml schema uses. Pure: no I/O, no global state besides a regex cache.
 *
 * Anything outside `ASSERTION_KEYWORDS` and `ANNOTATION_KEYWORDS` is rejected by
 * `unsupportedKeywords`, and a test runs it over the bundled schema, so a schema update
 * that needs a new keyword fails loudly instead of being silently ignored.
 */

export type JsonSchema = boolean | SchemaObject;
export interface SchemaObject {
  readonly $ref?: unknown;
  readonly type?: unknown;
  readonly const?: unknown;
  readonly enum?: unknown;
  readonly minLength?: unknown;
  readonly maxLength?: unknown;
  readonly pattern?: unknown;
  readonly minimum?: unknown;
  readonly maximum?: unknown;
  readonly exclusiveMinimum?: unknown;
  readonly exclusiveMaximum?: unknown;
  readonly required?: unknown;
  readonly properties?: unknown;
  readonly additionalProperties?: unknown;
  readonly propertyNames?: unknown;
  readonly items?: unknown;
  readonly allOf?: unknown;
  readonly anyOf?: unknown;
  readonly oneOf?: unknown;
  readonly description?: unknown;
  readonly deprecated?: unknown;
  readonly [keyword: string]: unknown;
}

export type Segment = string | number;

/** Keywords this validator enforces. */
export const ASSERTION_KEYWORDS = [
  '$ref',
  'type',
  'const',
  'enum',
  'minLength',
  'maxLength',
  'pattern',
  'minimum',
  'maximum',
  'exclusiveMinimum',
  'exclusiveMaximum',
  'required',
  'properties',
  'additionalProperties',
  'propertyNames',
  'items',
  'allOf',
  'anyOf',
  'oneOf',
] as const;

/** Keywords that carry no assertion (2020-12 treats `format` as an annotation by default). */
export const ANNOTATION_KEYWORDS = [
  '$schema',
  '$defs',
  'id',
  'title',
  'description',
  'markdownDescription',
  'default',
  'examples',
  'deprecated',
  'format',
  'contentEncoding',
] as const;

export type Keyword = (typeof ASSERTION_KEYWORDS)[number] | 'false' | 'deprecated';

export interface SchemaError {
  /** Path into the validated value. */
  readonly path: readonly Segment[];
  readonly keyword: Keyword;
  readonly severity: 'error' | 'warning';
  readonly message: string;
  /** The (sub)schema whose keyword failed. */
  readonly schema: SchemaObject;
  /** Keyword-specific detail, e.g. the missing property or the allowed values. */
  readonly params: ErrorParams;
}

export interface ErrorParams {
  readonly expected?: readonly string[];
  readonly actual?: unknown;
  readonly allowed?: readonly unknown[];
  readonly missing?: string;
  readonly unknown?: string;
  readonly limit?: number;
  readonly pattern?: string;
  readonly schema?: JsonSchema | undefined;
  readonly cause?: SchemaError;
  readonly renamedTo?: unknown;
  readonly removed?: boolean;
  /** Set on the summary error an anyOf/oneOf reports when no branch was close. */
  readonly fallback?: boolean;
}

/** Return every keyword path in `schema` this validator does not understand. */
export function unsupportedKeywords(schema: JsonSchema): string[] {
  const known = new Set<string>([...ASSERTION_KEYWORDS, ...ANNOTATION_KEYWORDS]);
  const out: string[] = [];
  const visit = (node: JsonSchema, at: string): void => {
    if (typeof node === 'boolean') return;
    for (const [key, value] of Object.entries(node)) {
      const here = `${at}/${key}`;
      if (!known.has(key) && !key.startsWith('x-')) out.push(here);
      if (key === 'properties' || key === '$defs') {
        for (const [name, sub] of Object.entries(value as Record<string, JsonSchema>)) {
          visit(sub, `${here}/${name}`);
        }
      } else if (key === 'allOf' || key === 'anyOf' || key === 'oneOf') {
        (value as JsonSchema[]).forEach((sub, i) => {
          visit(sub, `${here}/${i}`);
        });
      } else if (key === 'items' || key === 'additionalProperties' || key === 'propertyNames') {
        visit(value as JsonSchema, here);
      }
    }
  };
  visit(schema, '');
  return out;
}

/** Validate `value` against `root`. Returns errors and deprecation warnings, in document order. */
export function validate(root: JsonSchema, value: unknown): SchemaError[] {
  const out: SchemaError[] = [];
  check(root, root, value, [], out);
  return out;
}

const regexCache = new Map<string, RegExp>();
function regex(source: string): RegExp {
  let re = regexCache.get(source);
  if (re === undefined) {
    re = new RegExp(source, 'u');
    regexCache.set(source, re);
  }
  return re;
}

/** JSON type of a value as JSON Schema names it (`integer` for whole numbers). */
export function jsonType(value: unknown): string {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  if (typeof value === 'number') return Number.isInteger(value) ? 'integer' : 'number';
  return typeof value;
}

function typeMatches(expected: string, value: unknown): boolean {
  const actual = jsonType(value);
  return actual === expected || (expected === 'number' && actual === 'integer');
}

export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  return ka.every(
    (k) =>
      Object.hasOwn(b, k) &&
      deepEqual((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]),
  );
}

/** Resolve a local `$ref` such as `#/$defs/ResourceConfig`. */
export function resolveRef(root: JsonSchema, ref: string): JsonSchema {
  if (!ref.startsWith('#')) throw new Error(`unsupported $ref ${ref}: only local refs are allowed`);
  let node: unknown = root;
  for (const raw of ref.slice(1).split('/').slice(1)) {
    const seg = decodeURIComponent(raw).replaceAll('~1', '/').replaceAll('~0', '~');
    node = typeof node === 'object' && node !== null ? (node as SchemaObject)[seg] : undefined;
  }
  if (typeof node !== 'boolean' && (typeof node !== 'object' || node === null)) {
    throw new Error(`unresolvable $ref ${ref}`);
  }
  return node as JsonSchema;
}

const show = (v: unknown): string => JSON.stringify(v);

function check(
  root: JsonSchema,
  schema: JsonSchema,
  value: unknown,
  path: readonly Segment[],
  out: SchemaError[],
): void {
  if (schema === true) return;
  if (schema === false) {
    out.push(err(path, 'false', {}, 'no value is allowed here', {}));
    return;
  }
  const s = schema;
  const fail = (keyword: Keyword, message: string, params: ErrorParams = {}): void => {
    out.push(err(path, keyword, s, message, params));
  };

  if (typeof s.$ref === 'string') check(root, resolveRef(root, s.$ref), value, path, out);

  if (s.type !== undefined) {
    const types = (Array.isArray(s.type) ? s.type : [s.type]) as string[];
    if (!types.some((t) => typeMatches(t, value))) {
      // Further keywords would only repeat the type mismatch.
      fail('type', `expected ${types.join(' or ')}, got ${jsonType(value)}`, {
        expected: types,
        actual: jsonType(value),
      });
      return;
    }
  }
  if ('const' in s && !deepEqual(s.const, value)) {
    fail('const', `expected ${show(s.const)}, got ${show(value)}`, {
      allowed: [s.const],
      actual: value,
    });
  }
  if (Array.isArray(s.enum) && !s.enum.some((e) => deepEqual(e, value))) {
    fail('enum', `expected one of ${s.enum.map(show).join(', ')}, got ${show(value)}`, {
      allowed: s.enum,
      actual: value,
    });
  }

  if (typeof value === 'string') checkString(s, value, fail);
  if (typeof value === 'number') checkNumber(s, value, fail);
  if (Array.isArray(value) && s.items !== undefined) {
    value.forEach((item, i) => {
      check(root, s.items as JsonSchema, item, [...path, i], out);
    });
  }
  if (jsonType(value) === 'object') {
    checkObject(root, s, value as Record<string, unknown>, path, out, fail);
  }

  if (Array.isArray(s.allOf)) {
    for (const sub of s.allOf as JsonSchema[]) check(root, sub, value, path, out);
  }
  if (Array.isArray(s.anyOf)) checkAnyOf(root, s, 'anyOf', value, path, out);
  if (Array.isArray(s.oneOf)) checkAnyOf(root, s, 'oneOf', value, path, out);
}

type Fail = (keyword: Keyword, message: string, params?: ErrorParams) => void;

function checkString(s: SchemaObject, value: string, fail: Fail): void {
  const length = [...value].length;
  if (typeof s.minLength === 'number' && length < s.minLength) {
    fail('minLength', `must be at least ${s.minLength} characters long`, { limit: s.minLength });
  }
  if (typeof s.maxLength === 'number' && length > s.maxLength) {
    fail('maxLength', `must be at most ${s.maxLength} characters long`, { limit: s.maxLength });
  }
  if (typeof s.pattern === 'string' && !regex(s.pattern).test(value)) {
    fail('pattern', `${show(value)} does not match /${s.pattern}/`, { pattern: s.pattern });
  }
}

function checkNumber(s: SchemaObject, value: number, fail: Fail): void {
  if (typeof s.minimum === 'number' && value < s.minimum) {
    fail('minimum', `must be >= ${s.minimum}`, { limit: s.minimum });
  }
  if (typeof s.maximum === 'number' && value > s.maximum) {
    fail('maximum', `must be <= ${s.maximum}`, { limit: s.maximum });
  }
  if (typeof s.exclusiveMinimum === 'number' && value <= s.exclusiveMinimum) {
    fail('exclusiveMinimum', `must be > ${s.exclusiveMinimum}`, { limit: s.exclusiveMinimum });
  }
  if (typeof s.exclusiveMaximum === 'number' && value >= s.exclusiveMaximum) {
    fail('exclusiveMaximum', `must be < ${s.exclusiveMaximum}`, { limit: s.exclusiveMaximum });
  }
}

function checkObject(
  root: JsonSchema,
  s: SchemaObject,
  value: Record<string, unknown>,
  path: readonly Segment[],
  out: SchemaError[],
  fail: Fail,
): void {
  const properties = (s.properties ?? {}) as Record<string, JsonSchema>;
  if (Array.isArray(s.required)) {
    for (const name of s.required as string[]) {
      if (!Object.hasOwn(value, name)) {
        fail('required', `missing required key ${show(name)}`, {
          missing: name,
          schema: properties[name],
        });
      }
    }
  }
  for (const [key, child] of Object.entries(value)) {
    const childPath = [...path, key];
    if (s.propertyNames !== undefined) {
      const nameErrors: SchemaError[] = [];
      check(root, s.propertyNames as JsonSchema, key, childPath, nameErrors);
      for (const e of nameErrors) {
        if (e.severity === 'warning') continue;
        out.push(
          err(childPath, 'propertyNames', s, `invalid key ${show(key)}: ${e.message}`, {
            cause: e,
          }),
        );
      }
    }
    const sub = Object.hasOwn(properties, key) ? properties[key] : undefined;
    if (sub !== undefined) {
      if (typeof sub === 'object' && sub.deprecated === true) {
        out.push({
          path: childPath,
          keyword: 'deprecated',
          severity: 'warning',
          schema: sub,
          message: `${show(key)} is deprecated`,
          params: {
            renamedTo: sub['x-deprecated-renamed-to'],
            removed: sub['x-deprecated-removed'] === true,
          },
        });
      }
      check(root, sub, child, childPath, out);
    } else if (s.additionalProperties === false) {
      out.push(
        err(childPath, 'additionalProperties', s, `unknown key ${show(key)}`, {
          unknown: key,
          allowed: Object.keys(properties),
        }),
      );
    } else if (s.additionalProperties !== undefined) {
      check(root, s.additionalProperties as JsonSchema, child, childPath, out);
    }
  }
}

const isError = (e: SchemaError): boolean => e.severity === 'error';

/**
 * anyOf/oneOf: pass if a branch passes (exactly one for oneOf). On failure, report the errors
 * of the branch the value most plausibly meant, so users see "missing key `endpoint`" rather
 * than "matched none of 4 alternatives":
 *   1. if exactly one branch accepts the value's JSON type, use it;
 *   2. otherwise drop branches that fail a type/const/enum check on the value itself or on one
 *      of its direct keys (a discriminator such as `command: update`), and use the branch whose
 *      errors go deepest (ties: fewest errors);
 *   3. if nothing is left, report one error listing what was expected.
 */
function checkAnyOf(
  root: JsonSchema,
  s: SchemaObject,
  keyword: 'anyOf' | 'oneOf',
  value: unknown,
  path: readonly Segment[],
  out: SchemaError[],
): void {
  const branches = (s[keyword] as JsonSchema[]).map((sub) => {
    const errors: SchemaError[] = [];
    check(root, sub, value, path, errors);
    return errors;
  });
  const passing = branches.filter((errors) => !errors.some(isError));
  if (keyword === 'oneOf' && passing.length > 1) {
    out.push(
      err(path, 'oneOf', s, `matches ${passing.length} alternatives; exactly one is allowed`, {}),
    );
    return;
  }
  const [first] = passing;
  if (first !== undefined) {
    out.push(...first);
    return;
  }
  const depth = path.length;
  const shallowType = (e: SchemaError) =>
    e.path.length === depth &&
    (e.keyword === 'type' || e.keyword === 'false' || e.params.fallback === true);
  const typeOk = branches.filter((errors) => !errors.some(shallowType));
  if (typeOk.length === 1 && typeOk[0] !== undefined) {
    out.push(...typeOk[0]);
    return;
  }
  const discriminatorMiss = (e: SchemaError) =>
    e.path.length <= depth + 1 && (e.keyword === 'const' || e.keyword === 'enum');
  const candidates = typeOk.filter((errors) => !errors.some(discriminatorMiss));
  const score = (errors: SchemaError[]) =>
    Math.max(...errors.filter(isError).map((e) => e.path.length));
  const best = candidates.sort(
    (a, b) => score(b) - score(a) || a.filter(isError).length - b.filter(isError).length,
  )[0];
  if (best !== undefined) {
    out.push(...best);
    return;
  }
  const alternatives = s[keyword] as JsonSchema[];
  const allowed = constantsOf(root, alternatives);
  if (allowed !== undefined) {
    const list = allowed.map(show).join(', ');
    out.push(
      err(path, 'enum', s, `expected one of ${list}, got ${describeValue(value)}`, {
        allowed,
        actual: value,
        fallback: true,
      }),
    );
    return;
  }
  const expected = describeAlternatives(root, alternatives);
  out.push(
    err(path, keyword, s, `expected ${expected.join(' or ')}, got ${describeValue(value)}`, {
      expected,
      fallback: true,
    }),
  );
}

/** The allowed values when every alternative is a `const` or `enum`, else undefined. */
function constantsOf(root: JsonSchema, alternatives: readonly JsonSchema[]): unknown[] | undefined {
  const out: unknown[] = [];
  for (const alt of alternatives) {
    if (typeof alt === 'boolean') return undefined;
    if ('const' in alt) out.push(alt.const);
    else if (Array.isArray(alt.enum)) out.push(...alt.enum);
    else if (typeof alt.$ref === 'string') {
      const inner = constantsOf(root, [resolveRef(root, alt.$ref)]);
      if (inner === undefined) return undefined;
      out.push(...inner);
    } else if (Array.isArray(alt.anyOf) || Array.isArray(alt.oneOf)) {
      const inner = constantsOf(root, (alt.anyOf ?? alt.oneOf) as JsonSchema[]);
      if (inner === undefined) return undefined;
      out.push(...inner);
    } else return undefined;
  }
  return out;
}

function describeValue(value: unknown): string {
  return typeof value === 'object' && value !== null ? jsonType(value) : show(value);
}

/** Short human list of what a set of alternatives accepts: consts, enums, then types. */
export function describeAlternatives(
  root: JsonSchema,
  alternatives: readonly JsonSchema[],
): string[] {
  const out = new Set<string>();
  const visit = (sub: JsonSchema, budget: number): void => {
    if (typeof sub === 'boolean') {
      if (sub) out.add('any value');
      return;
    }
    if (typeof sub.$ref === 'string' && budget > 0) visit(resolveRef(root, sub.$ref), budget - 1);
    if ('const' in sub) out.add(show(sub.const));
    else if (Array.isArray(sub.enum)) for (const e of sub.enum) out.add(show(e));
    else if (sub.type !== undefined) {
      for (const t of (Array.isArray(sub.type) ? sub.type : [sub.type]) as string[]) {
        out.add(`a${/^[aeiou]/.test(t) ? 'n' : ''} ${t}`);
      }
    }
    for (const k of ['anyOf', 'oneOf'] as const) {
      if (Array.isArray(sub[k]) && budget > 0) {
        for (const inner of sub[k] as JsonSchema[]) visit(inner, budget - 1);
      }
    }
  };
  for (const sub of alternatives) visit(sub, 3);
  return [...out];
}

function err(
  path: readonly Segment[],
  keyword: Keyword,
  schema: SchemaObject,
  message: string,
  params: ErrorParams,
): SchemaError {
  return { path, keyword, severity: 'error', schema, message, params };
}
