// The public stainless.yml JSON Schema (draft 2020-12), bundled as a module so validation stays
// a pure function: no file reads at call time.
import schemaJson from '../../schema/stainless-config.schema.json' with { type: 'json' };
import type { SchemaObject } from './json-schema.ts';

export const STAINLESS_CONFIG_SCHEMA: SchemaObject = schemaJson as SchemaObject;
