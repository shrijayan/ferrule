import { z } from 'zod';
import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import { append, type Command, copy, merge, move, remove, update } from './commands.ts';
import { isRecord } from './jsonpath.ts';
import type { OpenApiSpec } from './pipeline.ts';
import { type Doc, error } from './targets.ts';

/**
 * Stainless `openapi.transforms`: update, append, merge, remove, copy and move, as defined by
 * the `openapi.transforms` items in converter/schema/stainless-config.schema.json.
 */

const Path = z.string();
const TargetEntrySchema = z.union([
  Path,
  z.looseObject({
    matches_schema: Path,
    ignore: z.union([Path, z.array(Path)]).optional(),
  }),
]);
const TargetSchema = z.union([Path, z.array(TargetEntrySchema)]);

interface Spec {
  readonly args: z.ZodType;
  readonly keys: readonly string[];
  readonly required: readonly string[];
  // biome-ignore lint/suspicious/noExplicitAny: each command validates its own args shape.
  readonly run: Command<any>;
}

const COMMANDS: Readonly<Record<string, Spec>> = {
  update: {
    args: z.looseObject({ target: TargetSchema, template: z.boolean().optional() }),
    keys: ['target', 'value', 'template'],
    required: ['target', 'value'],
    run: update,
  },
  append: {
    args: z.looseObject({ target: TargetSchema }),
    keys: ['target', 'value'],
    required: ['target', 'value'],
    run: append,
  },
  merge: {
    args: z.looseObject({ target: TargetSchema, value: z.record(z.string(), z.unknown()) }),
    keys: ['target', 'value'],
    required: ['target', 'value'],
    run: merge,
  },
  remove: {
    args: z.looseObject({ target: TargetSchema, keys: z.array(z.string()).optional() }),
    keys: ['target', 'keys'],
    required: ['target'],
    run: remove,
  },
  copy: {
    args: z.looseObject({ from: Path, to: TargetSchema }),
    keys: ['from', 'to'],
    required: ['from', 'to'],
    run: copy,
  },
  move: {
    args: z.looseObject({ from: Path, to: Path }),
    keys: ['from', 'to'],
    required: ['from', 'to'],
    run: move,
  },
};

/** The command names the schema defines, in schema order. */
export const TRANSFORM_COMMANDS = Object.keys(COMMANDS);

export interface ApplyTransformsResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

export interface ApplyTransformsOptions {
  /** JSON pointer of the transforms list in the config. Default `/openapi/transforms`. */
  readonly pointer?: string;
}

function invalid(pointer: string, message: string, suggestion: string): Diagnostic {
  return error('FER1203', pointer, message, suggestion);
}

function unknownKeys(
  value: Record<string, unknown>,
  known: readonly string[],
  pointer: string,
): Diagnostic[] {
  return Object.keys(value)
    .filter((k) => !known.includes(k))
    .sort()
    .map((k) => ({
      code: 'FER1204',
      severity: 'warning',
      message: `Unknown key "${k}" is ignored.`,
      pointer: `${pointer}${toJsonPointer([k])}`,
      suggestion: `Remove it; this place accepts: ${known.join(', ')}.`,
    }));
}

/** Validate one transform entry; returns the command to run, or diagnostics only. */
function validate(
  entry: unknown,
  pointer: string,
): { spec?: Spec; args?: unknown; diagnostics: Diagnostic[] } {
  const commands = TRANSFORM_COMMANDS.join(', ');
  if (!isRecord(entry)) {
    return {
      diagnostics: [
        invalid(pointer, 'A transform must be an object.', 'Write { command, reason, args }.'),
      ],
    };
  }
  const { command, reason, args } = entry;
  const spec = typeof command === 'string' ? COMMANDS[command] : undefined;
  if (spec === undefined) {
    return {
      diagnostics: [
        invalid(
          `${pointer}/command`,
          `Unknown transform command ${JSON.stringify(command)}.`,
          `Use one of: ${commands}.`,
        ),
      ],
    };
  }
  const diagnostics = unknownKeys(entry, ['command', 'reason', 'args'], pointer);
  if (typeof reason !== 'string' || reason.trim() === '') {
    diagnostics.push({
      code: 'FER1204',
      severity: 'warning',
      message: `The ${command} transform has no reason.`,
      pointer: `${pointer}/reason`,
      suggestion:
        'Add a reason saying why the spec needs this change, so it can be removed once fixed.',
    });
  }
  const at = `${pointer}/args`;
  if (!isRecord(args)) {
    diagnostics.push(
      invalid(
        at,
        `The ${command} transform needs an args object.`,
        `Give args: ${spec.required.join(', ')}.`,
      ),
    );
    return { diagnostics };
  }
  const missing = spec.required.filter((k) => !Object.hasOwn(args, k));
  for (const k of missing) {
    diagnostics.push(
      invalid(`${at}/${k}`, `The ${command} transform is missing args.${k}.`, `Add args.${k}.`),
    );
  }
  const parsed = spec.args.safeParse(args);
  if (!parsed.success) {
    for (const issue of parsed.error.issues) {
      if (issue.code === 'invalid_type' && missing.includes(String(issue.path[0]))) continue;
      diagnostics.push(
        invalid(
          `${at}${toJsonPointer(issue.path.map((p) => (typeof p === 'number' ? p : String(p))))}`,
          `Invalid ${command} args: ${issue.message}.`,
          'Fix the argument to match the openapi.transforms schema.',
        ),
      );
    }
  }
  diagnostics.push(...unknownKeys(args, spec.keys, at));
  if (diagnostics.some((d) => d.severity === 'error')) return { diagnostics };
  return { spec, args, diagnostics };
}

/**
 * Apply Stainless `openapi.transforms` to a deep copy of `spec`, in order. Pure: the input is
 * never mutated. A transform with any error diagnostic is skipped as a whole, leaving the spec as
 * the previous transform left it; later transforms still run.
 */
export function applyTransforms(
  spec: OpenApiSpec,
  transforms: readonly unknown[],
  options: ApplyTransformsOptions = {},
): ApplyTransformsResult {
  const base = options.pointer ?? '/openapi/transforms';
  const doc: Doc = { root: structuredClone(spec) };
  const diagnostics: Diagnostic[] = [];
  transforms.forEach((entry, i) => {
    const pointer = `${base}/${i}`;
    const checked = validate(entry, pointer);
    diagnostics.push(...checked.diagnostics);
    if (checked.spec === undefined) return;
    diagnostics.push(...checked.spec.run(doc, checked.args, `${pointer}/args`));
  });
  return { spec: doc.root as OpenApiSpec, diagnostics };
}

/**
 * Read `openapi.transforms` (or its deprecated alias `openapi.transformations`) from a config
 * and apply it. Pure.
 */
export function applyConfigTransforms(
  spec: OpenApiSpec,
  config: { readonly openapi?: unknown },
): ApplyTransformsResult {
  const { openapi } = config;
  if (!isRecord(openapi)) return { spec: structuredClone(spec), diagnostics: [] };
  const diagnostics: Diagnostic[] = [];
  const hasNew = Object.hasOwn(openapi, 'transforms');
  const hasOld = Object.hasOwn(openapi, 'transformations');
  const key = hasNew ? 'transforms' : 'transformations';
  if (hasOld) {
    diagnostics.push({
      code: 'FER1212',
      severity: hasNew ? 'error' : 'warning',
      message: hasNew
        ? 'Both openapi.transforms and openapi.transformations are set; transformations is ignored.'
        : 'openapi.transformations is deprecated.',
      pointer: '/openapi/transformations',
      suggestion: hasNew
        ? 'Move every entry into openapi.transforms and delete openapi.transformations.'
        : 'Rename openapi.transformations to openapi.transforms.',
    });
  }
  const list = openapi[key];
  if (list === undefined || list === null) return { spec: structuredClone(spec), diagnostics };
  if (!Array.isArray(list)) {
    diagnostics.push(
      invalid(
        `/openapi/${key}`,
        `openapi.${key} must be a list.`,
        'Write a YAML list of { command, reason, args } entries.',
      ),
    );
    return { spec: structuredClone(spec), diagnostics };
  }
  const result = applyTransforms(spec, list, { pointer: `/openapi/${key}` });
  return { spec: result.spec, diagnostics: [...diagnostics, ...result.diagnostics] };
}
