import { parse } from 'yaml';
import type { Diagnostic } from './diagnostic.ts';
import { applyEdition } from './editions/index.ts';
import { normalize } from './normalize/index.ts';
import type { Config } from './schema/config.ts';
import { validateConfig } from './schema/validate.ts';
import { stableStringify } from './serialize.ts';
import { annotate } from './transforms/annotate.ts';
import { applyConfigTransforms } from './transforms/openapi-transforms.ts';
import type { OpenApiSpec } from './transforms/pipeline.ts';

export type { Diagnostic, FerCode, Severity } from './diagnostic.ts';
export { toJsonPointer } from './diagnostic.ts';
export {
  applyEdition,
  DEFAULT_EDITION,
  EDITION_NAMES,
  EDITIONS,
  type Edition,
  type EditionResult,
  IMPLICIT_EDITION,
  isEdition,
  isPythonEdition,
  PYTHON_EDITIONS,
  type PythonEdition,
} from './editions/index.ts';
export type {
  FerruleClient,
  FerruleClientOpt,
  FerruleSecurityScheme,
} from './extensions/client.ts';
export { CLIENT_GROUP } from './extensions/groups.ts';
export { PAGINATION_SCHEMES, type PaginationScheme } from './extensions/pagination.ts';
export type { FerruleOnEvent, FerruleStreaming } from './extensions/streaming.ts';
export {
  NORMALIZE_CODES,
  NORMALIZED_KEY,
  type NormalizeResult,
  normalize,
} from './normalize/index.ts';
export {
  type Config,
  ConfigSchema,
  type Method,
  type Pagination,
  parseConfig,
  type Resource,
} from './schema/config.ts';
export {
  CONFIG_CODES,
  type ConfigValidation,
  validateConfig,
  validateConfigValue,
} from './schema/validate.ts';
export { sortKeys, stableStringify } from './serialize.ts';
export { annotate, type FerruleModel, type FerrulePagination } from './transforms/annotate.ts';
export {
  type ApplyTransformsOptions,
  type ApplyTransformsResult,
  applyConfigTransforms,
  applyTransforms,
  TRANSFORM_COMMANDS,
} from './transforms/openapi-transforms.ts';
export { type OpenApiSpec, pipeline, type Transform } from './transforms/pipeline.ts';

export interface ConvertResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

/**
 * Apply `openapi.transforms`, normalise constructs Forge cannot read, then annotate the result with Fern and x-ferrule-* extensions from
 * `config`. Pure; the inputs are never mutated.
 */
export function convert(spec: OpenApiSpec, config: Config): ConvertResult {
  // Config's index signature defeats TypeScript's weak-type check, hence the widening.
  const transformed = applyConfigTransforms(spec, config as { readonly openapi?: unknown });
  const normalized = normalize(transformed.spec);
  const annotated = annotate(normalized.spec, config);
  return {
    spec: annotated.spec,
    diagnostics: [...transformed.diagnostics, ...normalized.diagnostics, ...annotated.diagnostics],
  };
}

/** Parse OpenAPI text (JSON or YAML) into an object. Throws if it is not an object. */
export function parseSpec(text: string): OpenApiSpec {
  const value: unknown = parse(text);
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError('OpenAPI document must be a JSON or YAML object');
  }
  return value as OpenApiSpec;
}

/**
 * Text in, deterministic JSON text out: validate the config, apply its edition, convert. Pure; the
 * CLI only adds file I/O. A config that fails validation yields no output, only diagnostics.
 */
export function convertText(
  specText: string,
  configText: string,
): { output: string; diagnostics: readonly Diagnostic[] } {
  const v = validateConfig(configText);
  if (v.config === undefined) return { output: '', diagnostics: v.diagnostics };
  const ed = applyEdition(v.config);
  const { spec, diagnostics } = convert(parseSpec(specText), ed.config);
  return {
    output: stableStringify(spec),
    diagnostics: [...v.diagnostics, ...ed.diagnostics, ...diagnostics],
  };
}
