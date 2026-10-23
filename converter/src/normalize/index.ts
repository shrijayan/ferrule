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
} as const;

export const NORMALIZED_KEY = 'x-ferrule-normalized';

export interface NormalizeResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

type Json = unknown;

function isObject(v: Json): v is Record<string, Json> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Keys under which a schema keyword appears as data rather than as a schema. */
const DATA_KEYS = new Set(['example', 'examples', 'enum', 'const', 'default']);

export function normalize(spec: OpenApiSpec): NormalizeResult {
  const diagnostics: Diagnostic[] = [];

  function note(node: Record<string, Json>, rule: string): void {
    const prev = node[NORMALIZED_KEY];
    node[NORMALIZED_KEY] = Array.isArray(prev) ? [...prev, rule] : [rule];
  }

  function walk(value: Json, path: (string | number)[]): Json {
    if (Array.isArray(value)) return value.map((v, i) => walk(v, [...path, i]));
    if (!isObject(value)) return value;
    const out: Record<string, Json> = {};
    for (const [k, v] of Object.entries(value)) {
      out[k] = DATA_KEYS.has(k) || k.startsWith('x-') ? v : walk(v, [...path, k]);
    }
    const { type, discriminator } = out;
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

  return { spec: walk(spec, []) as OpenApiSpec, diagnostics };
}
