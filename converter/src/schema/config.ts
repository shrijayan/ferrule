import { parse } from 'yaml';
import { z } from 'zod';

/** `"get /path"` shorthand or the object form. Unknown keys are kept. */
export const MethodSchema = z.union([
  z.string(),
  z.looseObject({
    endpoint: z.string(),
    /** `false` stops list-method matching; a string names a pagination entry. */
    paginated: z.union([z.boolean(), z.string()]).optional(),
    type: z.string().optional(),
  }),
]);

/** A model is a bare schema name or an object form. */
export const ModelSchema = z.union([z.string(), z.looseObject({})]);

export type Method = z.infer<typeof MethodSchema>;
export type Model = z.infer<typeof ModelSchema>;

export interface Resource {
  readonly methods?: Readonly<Record<string, Method>> | undefined;
  readonly models?: Readonly<Record<string, Model>> | undefined;
  readonly subresources?: Readonly<Record<string, Resource>> | undefined;
  readonly [key: string]: unknown;
}

export const ResourceSchema: z.ZodType<Resource> = z.lazy(() =>
  z.looseObject({
    methods: z.record(z.string(), MethodSchema).optional(),
    models: z.record(z.string(), ModelSchema).optional(),
    subresources: z.record(z.string(), ResourceSchema).optional(),
  }),
);

const PaginationFieldSchema = z
  .looseObject({
    'x-stainless-pagination-property': z.looseObject({ purpose: z.string() }).optional(),
  })
  .nullable();

export const PaginationSchema = z.looseObject({
  name: z.string(),
  type: z.string(),
  request: z.record(z.string(), PaginationFieldSchema).optional(),
  response: z.record(z.string(), PaginationFieldSchema).optional(),
});

export type Pagination = z.infer<typeof PaginationSchema>;

/** The public schema also allows one unnamed scheme instead of a list (annotate names it `page`). */
export const SinglePaginationSchema = PaginationSchema.extend({ name: z.string().optional() });

/** The subset of a stainless.yml Ferrule reads today; the full JSON Schema lives in ../../schema. */
export const ConfigSchema = z.looseObject({
  edition: z.string().optional(),
  organization: z.looseObject({ name: z.string() }).optional(),
  resources: z.record(z.string(), ResourceSchema).optional(),
  pagination: z.union([z.array(PaginationSchema), SinglePaginationSchema]).optional(),
  targets: z
    .looseObject({
      python: z
        .looseObject({
          package_name: z.string().optional(),
          project_name: z.string().optional(),
          production_repo: z.string().optional(),
        })
        .optional(),
    })
    .optional(),
});

export type Config = z.infer<typeof ConfigSchema>;

/** Parse stainless.yml text into a validated Config. Throws a ZodError on invalid input. */
export function parseConfig(text: string): Config {
  return ConfigSchema.parse(parse(text) ?? {});
}
