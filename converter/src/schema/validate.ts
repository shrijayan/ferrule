import { type Document, isAlias, isMap, isScalar, isSeq, LineCounter, parseDocument } from 'yaml';
import type { Diagnostic, FerCode } from '../diagnostic.ts';
import { toJsonPointer } from '../diagnostic.ts';
import { type Config, ConfigSchema } from './config.ts';
import {
  type JsonSchema,
  type SchemaError,
  type SchemaObject,
  type Segment,
  validate,
} from './json-schema.ts';
import { STAINLESS_CONFIG_SCHEMA } from './stainless-schema.ts';
import { closest } from './suggest.ts';

/**
 * FER11xx: stainless.yml validation. FER1101 and FER1102 (editions) live in ../editions.
 * Codes are stable; never reuse a retired one.
 */
export const CONFIG_CODES = {
  /** The text is not valid YAML. */
  yamlSyntax: 'FER1100',
  /** A value has the wrong JSON type. */
  type: 'FER1110',
  /** A required key is missing. */
  required: 'FER1111',
  /** A key the schema does not allow (`additionalProperties: false`). */
  unknownKey: 'FER1112',
  /** A value outside the allowed `enum`/`const` set. */
  notAllowed: 'FER1113',
  /** A string fails `minLength`, `maxLength` or `pattern`. */
  stringFormat: 'FER1114',
  /** A number outside `minimum`/`maximum`. */
  range: 'FER1115',
  /** A value that matches none (or, for oneOf, several) of the alternatives. */
  alternatives: 'FER1116',
  /** A map key fails the schema's `propertyNames`. */
  keyName: 'FER1117',
  /** A deprecated key is used (warning). */
  deprecated: 'FER1118',
  /** Schema-valid, but not in the shape Ferrule's typed reader expects. */
  unsupportedShape: 'FER1119',
} as const satisfies Record<string, FerCode>;

export interface ConfigValidation {
  /** The typed config, or undefined when the YAML or Ferrule's typed reader rejects it. */
  readonly config: Config | undefined;
  /** The plain value parsed from YAML (aliases resolved), or undefined on a syntax error. */
  readonly value: unknown;
  readonly diagnostics: readonly Diagnostic[];
}

/**
 * Parse and validate stainless.yml text against the public JSON Schema. Pure. Never throws for
 * bad input: every problem is a FER11xx diagnostic whose message ends with its line and column.
 */
export function validateConfig(text: string): ConfigValidation {
  const lineCounter = new LineCounter();
  const doc = parseDocument(text, { lineCounter, prettyErrors: false });
  const at = (offset: number | undefined): string => {
    if (offset === undefined) return '';
    const { line, col } = lineCounter.linePos(offset);
    return ` (line ${line}, column ${col})`;
  };
  if (doc.errors.length > 0) {
    return {
      config: undefined,
      value: undefined,
      diagnostics: doc.errors.map((e) => ({
        code: CONFIG_CODES.yamlSyntax,
        severity: 'error',
        message: `invalid YAML: ${e.message.split('\n')[0]}${at(e.pos[0])}`,
        pointer: '',
        suggestion: 'Fix the YAML syntax at the reported position.',
      })),
    };
  }
  const value: unknown = doc.toJS() ?? {};
  const located = toDiagnostics(validate(STAINLESS_CONFIG_SCHEMA, value)).map((d) => ({
    ...d,
    message: `${d.message}${at(offsetOf(doc.contents, d.segments, d.onKey, doc))}`,
  }));
  const diagnostics: Diagnostic[] = located.map(({ segments: _s, onKey: _k, ...d }) => d);
  if (diagnostics.some((d) => d.severity === 'error')) {
    return { config: undefined, value, diagnostics };
  }
  const typed = ConfigSchema.safeParse(value);
  if (!typed.success) {
    for (const issue of typed.error.issues) {
      const segments = issue.path.filter((p): p is Segment => typeof p !== 'symbol');
      diagnostics.push({
        code: CONFIG_CODES.unsupportedShape,
        severity: 'error',
        message: `Ferrule cannot read this value yet: ${issue.message}${at(offsetOf(doc.contents, segments, false, doc))}`,
        pointer: toJsonPointer(segments),
        suggestion:
          'Open an issue with this config; the public schema accepts it but Ferrule does not.',
      });
    }
    return { config: undefined, value, diagnostics };
  }
  return { config: typed.data, value, diagnostics };
}

/** Validate an already-parsed value (no source positions). */
export function validateConfigValue(value: unknown): Diagnostic[] {
  return toDiagnostics(validate(STAINLESS_CONFIG_SCHEMA, value)).map(
    ({ segments: _s, onKey: _k, ...d }) => d,
  );
}

interface Located extends Diagnostic {
  readonly segments: readonly Segment[];
  /** Point at the key rather than the value. */
  readonly onKey: boolean;
}

function toDiagnostics(errors: readonly SchemaError[]): Located[] {
  return errors.map((e) => {
    const base = {
      segments: e.path,
      onKey: false,
      severity: e.severity,
      pointer: toJsonPointer(e.path),
    };
    const p = e.params;
    switch (e.keyword) {
      case 'type':
        return {
          ...base,
          code: CONFIG_CODES.type,
          message: `${label(e.path)}: ${e.message}`,
          suggestion: `Change the value to ${article((p.expected as string[]).join(' or '))}.`,
        };
      case 'required': {
        const name = String(p.missing);
        const hint = firstSentence(p.schema as JsonSchema | undefined);
        return {
          ...base,
          code: CONFIG_CODES.required,
          message: `${label(e.path)}: ${e.message}`,
          suggestion: `Add \`${name}\`${hint ? ` (${hint})` : ''}.`,
        };
      }
      case 'additionalProperties': {
        const name = String(p.unknown);
        const allowed = p.allowed as string[];
        const near = closest(name, allowed);
        return {
          ...base,
          onKey: true,
          code: CONFIG_CODES.unknownKey,
          message: `${label(e.path.slice(0, -1))}: ${e.message}`,
          suggestion: near
            ? `Did you mean \`${near}\`?`
            : `Remove \`${name}\`; allowed keys are ${allowed.map((k) => `\`${k}\``).join(', ')}.`,
        };
      }
      case 'const':
      case 'enum': {
        const allowed = (p.allowed as unknown[]).map((v) => JSON.stringify(v));
        const strings = (p.allowed as unknown[]).filter((v): v is string => typeof v === 'string');
        const near = typeof p.actual === 'string' ? closest(p.actual, strings) : undefined;
        return {
          ...base,
          code: CONFIG_CODES.notAllowed,
          message: `${label(e.path)}: ${e.message}`,
          suggestion: near ? `Did you mean "${near}"?` : `Use one of ${allowed.join(', ')}.`,
        };
      }
      case 'minLength':
      case 'maxLength':
      case 'pattern':
        return {
          ...base,
          code: CONFIG_CODES.stringFormat,
          message: `${label(e.path)}: ${e.message}`,
          suggestion: describe(e.schema) ?? 'Change the string to the documented format.',
        };
      case 'minimum':
      case 'maximum':
      case 'exclusiveMinimum':
      case 'exclusiveMaximum':
        return {
          ...base,
          code: CONFIG_CODES.range,
          message: `${label(e.path)}: ${e.message}`,
          suggestion: 'Use a number inside the allowed range.',
        };
      case 'propertyNames':
        return {
          ...base,
          onKey: true,
          code: CONFIG_CODES.keyName,
          message: `${label(e.path.slice(0, -1))}: ${e.message}`,
          suggestion: 'Rename the key to one the schema allows.',
        };
      case 'deprecated': {
        const to = p.renamedTo;
        return {
          ...base,
          onKey: true,
          code: CONFIG_CODES.deprecated,
          message: `${label(e.path)}: ${e.message}`,
          suggestion:
            typeof to === 'string'
              ? `Rename \`${String(e.path.at(-1))}\` to \`${to}\`.`
              : `Remove \`${String(e.path.at(-1))}\`; it no longer has any effect.`,
        };
      }
      default:
        // anyOf, oneOf, false
        return {
          ...base,
          code: CONFIG_CODES.alternatives,
          message: `${label(e.path)}: ${e.message}`,
          suggestion:
            describe(e.schema) ?? 'Change the value to one of the forms the schema documents.',
        };
    }
  });
}

/** The YAML source offset for a path: the key node when `onKey`, else the value node. */
function offsetOf(
  contents: unknown,
  segments: readonly Segment[],
  onKey: boolean,
  doc: Document,
): number | undefined {
  let node: unknown = contents;
  let last: number | undefined = rangeStart(node);
  for (const [i, seg] of segments.entries()) {
    if (isAlias(node)) node = node.resolve(doc);
    let next: unknown;
    let key: unknown;
    if (isMap(node)) {
      const pair = node.items.find(
        (it) => isScalar(it.key) && String(it.key.value) === String(seg),
      );
      next = pair?.value;
      key = pair?.key;
    } else if (isSeq(node) && typeof seg === 'number') {
      next = node.items[seg];
    }
    if (onKey && i === segments.length - 1 && key !== undefined) return rangeStart(key);
    if (next === undefined || next === null) return key !== undefined ? rangeStart(key) : last;
    node = next;
    last = rangeStart(node) ?? last;
  }
  return last;
}

function rangeStart(node: unknown): number | undefined {
  const range = (node as { range?: readonly number[] } | null | undefined)?.range;
  return range?.[0];
}

function label(path: readonly Segment[]): string {
  return path.length === 0 ? 'config' : path.join('.');
}

function article(t: string): string {
  return `a${/^[aeiou]/.test(t) ? 'n' : ''} ${t}`;
}

function describe(schema: SchemaObject): string | undefined {
  return firstSentence(schema);
}

function firstSentence(schema: JsonSchema | undefined): string | undefined {
  if (typeof schema !== 'object' || typeof schema.description !== 'string') return undefined;
  const sentence = schema.description
    .split(/(?<=\.)\s|\n/)[0]
    ?.trim()
    .replace(/\.$/, '');
  if (!sentence || sentence.length > 160) return undefined;
  return sentence;
}
