import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import type { OpenApiSpec } from '../transforms/pipeline.ts';

/**
 * Normalisation: rewrite constructs the pinned Forge OpenAPI reader rejects into equivalents it
 * accepts, without changing what a generator sees. Applied after transforms, before annotation.
 * Pure; the input is never mutated. Every rewritten node gets an `x-ferrule-normalized` note (a
 * list of rule names) and a FER16xx info diagnostic. See docs/forge-gaps.md.
 */
export const NORMALIZE_CODES = {
  /** `type` array holds a non-string entry (a YAML `null`), rewritten to the string `"null"`. */
  typeNullLiteral: 'FER1601',
  /** `discriminator` is a bare property name string, rewritten to `{ propertyName }`. */
  discriminatorString: 'FER1602',
  /** A query parameter's `style` is not `form`, the only one Forge reads; the style is dropped. */
  queryStyle: 'FER1603',
  /** Fields Forge names alike get a name extension spelling out their symbols (`+1` -> `plus_1`). */
  equalNamesSpelled: 'FER1604',
  /** Fields Forge names alike with nothing to spell out: the extra ones are numbered. */
  equalNamesNumbered: 'FER1605',
  /** A `$ref` into components.schemas points to nothing; replaced by an unknown schema. */
  danglingRef: 'FER1606',
  /** `items` is a list (a tuple, draft 4); rewritten to `prefixItems`. */
  itemsTuple: 'FER1607',
} as const;

export const NORMALIZED_KEY = 'x-ferrule-normalized';

export interface NormalizeResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

type Json = unknown;

/** An object node; the OpenAPI keys read here are named so they can be read with a dot. */
type Node = Record<string, Json> & {
  $ref?: Json;
  items?: Json;
  prefixItems?: Json;
  basePath?: Json;
  components?: Json;
  consumes?: Json;
  content?: Json;
  definitions?: Json;
  description?: Json;
  headers?: Json;
  host?: Json;
  in?: Json;
  name?: Json;
  nullable?: Json;
  parameters?: Json;
  paths?: Json;
  produces?: Json;
  properties?: Json;
  requestBody?: Json;
  required?: Json;
  responses?: Json;
  schema?: Json;
  schemas?: Json;
  schemes?: Json;
  securityDefinitions?: Json;
  securitySchemes?: Json;
  servers?: Json;
  style?: Json;
  swagger?: Json;
  type?: Json;
};

function isObject(v: Json): v is Node {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Keys under which a schema keyword appears as data rather than as a schema. */
const DATA_KEYS = new Set(['example', 'examples', 'enum', 'const', 'default']);

/** Keys whose value maps names to nodes: a name such as `default` there is not a keyword. */
const NAME_MAPS = new Set([
  'properties',
  'patternProperties',
  '$defs',
  'definitions',
  'schemas',
  'parameters',
  'responses',
  'requestBodies',
  'headers',
  'content',
  'paths',
]);

/** Walk `value`'s children with `visit`; entries of name maps are always visited. */
function children(value: Node, visit: (child: Json, keys: readonly string[]) => Json): Node {
  const out: Node = {};
  for (const [k, v] of Object.entries(value)) {
    if (NAME_MAPS.has(k) && isObject(v)) {
      out[k] = Object.fromEntries(Object.entries(v).map(([n, c]) => [n, visit(c, [k, n])]));
    } else {
      out[k] = DATA_KEYS.has(k) || k.startsWith('x-') ? v : visit(v, [k]);
    }
  }
  return out;
}

/**
 * The words Forge names a field by (`.cache/forge/src/name.rs` `Name::new`): split at characters
 * that are neither letters nor digits and at case changes, lower-cased. Fields with equal words
 * collide in Forge.
 */
export function forgeWords(s: string): string[] {
  const words: string[] = [];
  let word = '';
  const chars = [...s];
  const flush = (): void => {
    if (word !== '') words.push(word.toLowerCase());
    word = '';
  };
  chars.forEach((c, i) => {
    if (!/[\p{L}\p{N}]/u.test(c)) {
      flush();
      return;
    }
    const prev = word.at(-1);
    if (prev !== undefined && /\p{Lu}/u.test(c)) {
      const nextLower = /\p{Ll}/u.test(chars[i + 1] ?? '');
      if (/[\p{Ll}\p{N}]/u.test(prev) || (/\p{Lu}/u.test(prev) && nextLower)) flush();
    }
    word += c;
  });
  flush();
  return words;
}

const SYMBOLS: Record<string, string> = {
  '+': 'plus',
  '<': 'lt',
  '>': 'gt',
  '=': 'eq',
  '!': 'not',
};

/** `wire` with its meaningful symbols spelled out, in Forge words. */
function spelled(wire: string): string[] {
  const text = [...wire]
    .map((c, i, all) => {
      if (c === '-' && (i === 0 || /\d/.test(all[i + 1] ?? ''))) return ' minus ';
      const word = SYMBOLS[c];
      return word === undefined ? c : ` ${word} `;
    })
    .join('');
  return forgeWords(text);
}

interface NamedField {
  readonly wire: string;
  readonly words: readonly string[];
  /** Give the field the name `snake`, written as a name extension; returns its pointer. */
  readonly rename: (snake: string) => string;
}

export function normalize(spec: OpenApiSpec): NormalizeResult {
  const diagnostics: Diagnostic[] = [];
  const schemas = isObject((spec as Node).components)
    ? ((spec as { components: Node }).components.schemas ?? {})
    : {};

  function note(node: Node, rule: string): void {
    const prev = node[NORMALIZED_KEY];
    node[NORMALIZED_KEY] = Array.isArray(prev) ? [...prev, rule] : [rule];
  }

  function walk(value: Json, path: (string | number)[]): Json {
    if (Array.isArray(value)) return value.map((v, i) => walk(v, [...path, i]));
    if (!isObject(value)) return value;
    const out = children(value, (child, keys) => walk(child, [...path, ...keys]));
    const { type, discriminator, $ref, items } = out;
    if (Array.isArray(items)) {
      delete out.items;
      out.prefixItems = items;
      note(out, 'items-tuple');
      diagnostics.push({
        code: NORMALIZE_CODES.itemsTuple,
        severity: 'info',
        message:
          '`items` is a list (a draft-4 tuple); rewritten to `prefixItems`, which Forge reads as an unknown tuple',
        pointer: toJsonPointer([...path, 'items']),
        suggestion:
          'if a list of one of these was meant, write `items: { anyOf: [...] }` in the spec',
      });
    }
    if (out.in === 'query' && typeof out.style === 'string' && out.style !== 'form') {
      const style = out.style;
      delete out.style;
      note(out, `query-style-${style}`);
      diagnostics.push({
        code: NORMALIZE_CODES.queryStyle,
        severity: 'info',
        message: `query parameter style "${style}" is not read by Forge; dropped (the generated params type is the same, the runtime serialises queries)`,
        pointer: toJsonPointer([...path, 'style']),
        suggestion: 'nothing to do; the original style is kept in x-ferrule-normalized',
      });
    }
    const schemaPrefix = '#/components/schemas/';
    if (typeof $ref === 'string' && $ref.startsWith(schemaPrefix)) {
      const name = $ref.slice(schemaPrefix.length).replaceAll('~1', '/').replaceAll('~0', '~');
      if (!(isObject(schemas) && Object.hasOwn(schemas, name))) {
        delete out.$ref;
        note(out, 'dangling-ref');
        diagnostics.push({
          code: NORMALIZE_CODES.danglingRef,
          severity: 'warning',
          message: `$ref "${$ref}" points to nothing; replaced by an unknown schema`,
          pointer: toJsonPointer([...path, '$ref']),
          suggestion: `add components.schemas.${name} to the spec, or fix the $ref with an openapi.transforms update`,
        });
      }
    }
    if (Array.isArray(type) && type.includes(null)) {
      Object.assign(out, { type: type.map((t) => (t === null ? 'null' : t)) });
      note(out, 'type-null-literal');
      diagnostics.push({
        code: NORMALIZE_CODES.typeNullLiteral,
        severity: 'info',
        message: '`type` array contains a null literal; rewritten to the string "null"',
        pointer: toJsonPointer([...path, 'type']),
        suggestion: 'quote it in the spec: `type: [array, "null"]`',
      });
    }
    if (typeof discriminator === 'string') {
      Object.assign(out, { discriminator: { propertyName: discriminator } });
      note(out, 'discriminator-string');
      diagnostics.push({
        code: NORMALIZE_CODES.discriminatorString,
        severity: 'info',
        message: '`discriminator` is a string; rewritten to a Discriminator Object',
        pointer: toJsonPointer([...path, 'discriminator']),
        suggestion: 'write `discriminator: { propertyName: <name> }` in the spec',
      });
    }
    return out;
  }

  const out = walk(spec, []) as Node;

  /** Give fields Forge would name alike distinct names, as the docs/forge-gaps.md G26 row says. */
  function disambiguate(fields: readonly NamedField[]): void {
    const key = (w: readonly string[]): string => w.join(' ');
    const taken = new Set(fields.map((f) => key(f.words)));
    const groups = new Map<string, NamedField[]>();
    for (const f of fields) groups.set(key(f.words), [...(groups.get(key(f.words)) ?? []), f]);
    for (const group of groups.values()) {
      if (group.length < 2) continue;
      const left: NamedField[] = [];
      for (const f of group) {
        const words = spelled(f.wire);
        if (taken.has(key(words))) {
          left.push(f);
          continue;
        }
        taken.add(key(words));
        const pointer = f.rename(words.join('_'));
        diagnostics.push({
          code: NORMALIZE_CODES.equalNamesSpelled,
          severity: 'info',
          message: `"${f.wire}" has the same Forge name as another field; named ${words.join('_')}`,
          pointer,
          suggestion: 'set x-fern-property-name or x-fern-parameter-name to choose the name',
        });
      }
      if (left.length < 2) continue;
      const keep = left.find((f) => f.wire === f.words.join('_')) ?? (left[0] as NamedField);
      let n = 2;
      for (const f of left) {
        if (f === keep) continue;
        while (taken.has(key([...f.words, String(n)]))) n++;
        const words = [...f.words, String(n)];
        taken.add(key(words));
        const pointer = f.rename(words.join('_'));
        diagnostics.push({
          code: NORMALIZE_CODES.equalNamesNumbered,
          severity: 'warning',
          message: `"${f.wire}" and "${keep.wire}" have the same Forge name; "${f.wire}" is named ${words.join('_')}`,
          pointer,
          suggestion: 'set x-fern-property-name or x-fern-parameter-name to choose the name',
        });
      }
    }
  }

  const PROPERTY_NAME = 'x-fern-property-name';
  const PARAMETER_NAME = 'x-fern-parameter-name';

  function nameOf(node: Json, wire: string, ext: string): string[] {
    const own = isObject(node) ? node[ext] : undefined;
    return forgeWords(typeof own === 'string' ? own : wire);
  }

  function properties(value: Json, path: (string | number)[]): void {
    if (Array.isArray(value)) {
      value.forEach((v, i) => {
        properties(v, [...path, i]);
      });
      return;
    }
    if (!isObject(value)) return;
    children(value, (child, keys) => properties(child, [...path, ...keys]));
    const props = value.properties;
    if (!isObject(props)) return;
    disambiguate(
      Object.entries(props)
        .filter(([, p]) => isObject(p))
        .map(([wire, p]) => ({
          wire,
          words: nameOf(p, wire, PROPERTY_NAME),
          rename: (snake) => {
            const node = { ...(p as Node), [PROPERTY_NAME]: snake };
            note(node, 'equal-names');
            props[wire] = node;
            return toJsonPointer([...path, 'properties', wire]);
          },
        })),
    );
  }
  properties(out, []);

  const componentParams = isObject(out.components) ? out.components.parameters : undefined;
  function resolveParam(p: Json): Json {
    const prefix = '#/components/parameters/';
    if (!isObject(p) || typeof p.$ref !== 'string' || !p.$ref.startsWith(prefix)) return p;
    return isObject(componentParams) ? componentParams[p.$ref.slice(prefix.length)] : undefined;
  }
  const METHODS = ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace'];
  for (const [route, item] of Object.entries(isObject(out.paths) ? out.paths : {})) {
    if (!isObject(item)) continue;
    const shared = Array.isArray(item.parameters) ? item.parameters : [];
    for (const method of METHODS) {
      const op = item[method];
      if (!isObject(op)) continue;
      const own: Json[] = Array.isArray(op.parameters) ? [...op.parameters] : [];
      const where = (p: Json): string => {
        const r = resolveParam(p);
        return isObject(r) ? `${String(r.in)}:${String(r.name)}` : '';
      };
      const ownKeys = new Set(own.map(where));
      const declared = [
        ...shared.filter((p) => !ownKeys.has(where(p))).map((p) => ({ p, index: -1 })),
        ...own.map((p, index) => ({ p, index })),
      ];
      const field = ({ p, index }: { p: Json; index: number }): NamedField | undefined => {
        const r = resolveParam(p);
        if (!isObject(r) || typeof r.name !== 'string') return undefined;
        const wire = r.name;
        return {
          wire,
          words: nameOf(r, wire, PARAMETER_NAME),
          rename: (snake) => {
            const node = { ...r, [PARAMETER_NAME]: snake };
            note(node, 'equal-names');
            // An inline copy at the operation replaces a shared or $ref'd declaration.
            const at = index === -1 ? own.length : index;
            own[at] = node;
            op.parameters = own;
            return toJsonPointer(['paths', route, method, 'parameters', at]);
          },
        };
      };
      const at = (locations: readonly string[]): NamedField[] =>
        declared
          .filter(({ p }) => {
            const r = resolveParam(p);
            return isObject(r) && locations.includes(String(r.in));
          })
          .map(field)
          .filter((f) => f !== undefined);
      disambiguate(at(['path', 'query']));
      disambiguate(at(['header']));
    }
  }

  return { spec: out as OpenApiSpec, diagnostics };
}
