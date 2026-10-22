import { parse } from 'yaml';
import { z } from 'zod';

/** Minimal phase-0 view of a stainless.yml; the full JSON Schema lives in ../../schema. */
export const ConfigSchema = z.looseObject({
  edition: z.string().optional(),
  organization: z.looseObject({ name: z.string() }).optional(),
  resources: z.record(z.string(), z.unknown()).optional(),
});

export type Config = z.infer<typeof ConfigSchema>;

/** Parse stainless.yml text into a validated Config. Throws on invalid input. */
export function parseConfig(text: string): Config {
  return ConfigSchema.parse(parse(text) ?? {});
}
