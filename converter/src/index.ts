import type { Diagnostic } from './diagnostic.ts';
import type { Config } from './schema/config.ts';
import { type OpenApiSpec, pipeline } from './transforms/pipeline.ts';

export type { Diagnostic, FerCode, Severity } from './diagnostic.ts';
export { toJsonPointer } from './diagnostic.ts';
export { DEFAULT_EDITION, EDITIONS, type Edition, isEdition } from './editions/index.ts';
export { type Config, ConfigSchema, parseConfig } from './schema/config.ts';
export { type OpenApiSpec, pipeline, type Transform } from './transforms/pipeline.ts';

export interface ConvertResult {
  readonly spec: OpenApiSpec;
  readonly diagnostics: readonly Diagnostic[];
}

/** Phase 0 stub: no transforms are registered yet, so the spec is returned unchanged. */
export function convert(spec: OpenApiSpec, config: Config): ConvertResult {
  return { spec: pipeline([])(spec, config), diagnostics: [] };
}
