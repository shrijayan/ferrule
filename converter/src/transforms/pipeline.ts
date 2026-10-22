import type { Config } from '../schema/config.ts';

export type OpenApiSpec = Readonly<Record<string, unknown>>;

/** A pure transform: annotated spec in, annotated spec out. */
export type Transform = (spec: OpenApiSpec, config: Config) => OpenApiSpec;

/** Compose transforms, applied left to right. */
export function pipeline(transforms: readonly Transform[]): Transform {
  return (spec, config) => transforms.reduce((acc, t) => t(acc, config), spec);
}
