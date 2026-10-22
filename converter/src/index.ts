import { parse } from 'yaml';
import type { Diagnostic } from './diagnostic.ts';
import type { Config } from './schema/config.ts';
import { parseConfig } from './schema/config.ts';
import { stableStringify } from './serialize.ts';
import { annotate } from './transforms/annotate.ts';
import type { OpenApiSpec } from './transforms/pipeline.ts';

export type { Diagnostic, FerCode, Severity } from './diagnostic.ts';
export { toJsonPointer } from './diagnostic.ts';
export { DEFAULT_EDITION, EDITIONS, type Edition, isEdition } from './editions/index.ts';
export {
  type Config,
  ConfigSchema,
  type Method,
  type Pagination,
  parseConfig,
  type Resource,
} from './schema/config.ts';
export { sortKeys, stableStringify } from './serialize.ts';
export { annotate, type FerrulePagination } from './transforms/annotate.ts';
export { type OpenApiSpec, pipeline, type Transform } from './transforms/pipeline.ts';

export interface ConvertResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

/** Annotate a deep copy of `spec` with Fern and x-ferrule-* extensions from `config`. Pure. */
export function convert(spec: OpenApiSpec, config: Config): ConvertResult {
  return annotate(spec, config);
}

/** Parse OpenAPI text (JSON or YAML) into an object. Throws if it is not an object. */
export function parseSpec(text: string): OpenApiSpec {
  const value: unknown = parse(text);
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError('OpenAPI document must be a JSON or YAML object');
  }
  return value as OpenApiSpec;
}

/** Text in, deterministic JSON text out. Pure; the CLI only adds file I/O. */
export function convertText(
  specText: string,
  configText: string,
): { output: string; diagnostics: readonly Diagnostic[] } {
  const { spec, diagnostics } = convert(parseSpec(specText), parseConfig(configText));
  return { output: stableStringify(spec), diagnostics };
}
