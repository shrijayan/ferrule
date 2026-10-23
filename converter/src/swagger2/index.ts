import type { Diagnostic } from '../diagnostic.ts';
import { NORMALIZED_KEY } from '../normalize/index.ts';
import type { OpenApiSpec } from '../transforms/pipeline.ts';

/**
 * Swagger 2.0 to OpenAPI 3.0: the pinned Forge reader knows only OpenAPI 3 (`in: body` and
 * `in: formData` are refused). Runs before `openapi.transforms`, so transforms written against the
 * OpenAPI 3 shape apply. Pure; the input is never mutated. Body and form parameters become request
 * bodies noted with `x-ferrule-normalized`; the document gets one FER1610 info diagnostic.
 * See docs/forge-gaps.md (G24).
 */
export const SWAGGER2_CODE = 'FER1610';

type Json = unknown;
/** An object node; the OpenAPI keys read here are named so they can be read with a dot. */
type Obj = Record<string, Json> & {
  $ref?: Json;
  basePath?: Json;
  components?: Json;
  consumes?: Json;
  content?: Json;
  definitions?: Json;
  description?: Json;
  headers?: Json;
  host?: Json;
  in?: Json;
  name?: Json;
  nullable?: Json;
  parameters?: Json;
  paths?: Json;
  produces?: Json;
  properties?: Json;
  requestBody?: Json;
  required?: Json;
  responses?: Json;
  schema?: Json;
  schemas?: Json;
  schemes?: Json;
  securityDefinitions?: Json;
  securitySchemes?: Json;
  servers?: Json;
  style?: Json;
  swagger?: Json;
  type?: Json;
};

function isObject(v: Json): v is Obj {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

const REFS: readonly [string, string][] = [
  ['#/definitions/', '#/components/schemas/'],
  ['#/parameters/', '#/components/parameters/'],
  ['#/responses/', '#/components/responses/'],
];

/** Keywords of a non-body Swagger 2 parameter that move into its OpenAPI 3 `schema`. */
const SCHEMA_KEYS = [
  'type',
  'format',
  'items',
  'default',
  'maximum',
  'exclusiveMaximum',
  'minimum',
  'exclusiveMinimum',
  'maxLength',
  'minLength',
  'pattern',
  'maxItems',
  'minItems',
  'uniqueItems',
  'enum',
  'multipleOf',
];

const COLLECTION_FORMATS: Record<string, Obj> = {
  csv: { style: 'form', explode: false },
  ssv: { style: 'spaceDelimited', explode: false },
  pipes: { style: 'pipeDelimited', explode: false },
  multi: { style: 'form', explode: true },
};

const METHODS = new Set(['get', 'put', 'post', 'delete', 'options', 'head', 'patch']);

/** Rewrite refs, `x-nullable`, `type: file` and string discriminators throughout a schema. */
function schema(value: Json): Json {
  if (Array.isArray(value)) return value.map(schema);
  if (!isObject(value)) return value;
  const out: Obj = {};
  for (const [k, v] of Object.entries(value)) {
    if (k === '$ref' && typeof v === 'string') {
      const hit = REFS.find(([from]) => v.startsWith(from));
      out[k] = hit === undefined ? v : hit[1] + v.slice(hit[0].length);
    } else if (k === 'x-nullable') {
      out.nullable = v;
    } else if (k === 'discriminator' && typeof v === 'string') {
      out[k] = { propertyName: v };
    } else if (k === 'example' || k === 'enum' || k === 'default' || k.startsWith('x-')) {
      out[k] = v;
    } else {
      out[k] = schema(v);
    }
  }
  if (out.type === 'file') Object.assign(out, { type: 'string', format: 'binary' });
  return out;
}

/** A non-body parameter (or a header object): its type keywords move into `schema`. */
function simpleParam(p: Obj): Obj {
  const out: Obj = {};
  const inner: Obj = {};
  for (const [k, v] of Object.entries(p)) {
    if (SCHEMA_KEYS.includes(k)) inner[k] = v;
    else if (k === 'collectionFormat') Object.assign(out, COLLECTION_FORMATS[String(v)] ?? {});
    else if (k === 'x-nullable') inner.nullable = v;
    else if (k !== 'allowEmptyValue' || p.in === 'query') out[k] = v;
  }
  if (Object.keys(inner).length > 0) out.schema = schema(inner);
  return out;
}

/** Form field schema of a `formData` parameter: its type keywords plus its description. */
function formField(p: Obj): Json {
  const { schema: s } = simpleParam(p);
  const field = isObject(s) ? { ...s } : {};
  if (typeof p.description === 'string') field.description = p.description;
  return field;
}

function content(types: readonly string[], s: Json): Obj {
  return Object.fromEntries(types.map((t) => [t, { schema: s }]));
}

export function swagger2ToOpenApi3(spec: OpenApiSpec): {
  spec: OpenApiSpec;
  diagnostics: readonly Diagnostic[];
} {
  const doc = spec as Obj;
  if (doc.swagger !== '2.0') return { spec, diagnostics: [] };
  const strings = (v: Json): string[] | undefined =>
    Array.isArray(v) && v.length > 0 ? v.map(String) : undefined;
  const rootConsumes = strings(doc.consumes) ?? ['application/json'];
  const rootProduces = strings(doc.produces) ?? ['application/json'];
  const globalParams = isObject(doc.parameters) ? doc.parameters : {};

  /** Body and formData parameters cannot be components in OpenAPI 3, so their refs are inlined. */
  function resolve(p: Json): Json {
    if (!isObject(p) || typeof p.$ref !== 'string' || !p.$ref.startsWith('#/parameters/')) return p;
    const target = globalParams[p.$ref.slice('#/parameters/'.length)];
    return isObject(target) && (target.in === 'body' || target.in === 'formData') ? target : p;
  }

  function response(r: Json, produces: readonly string[]): Json {
    if (!isObject(r) || typeof r.$ref === 'string') return schema(r);
    const out: Obj = {};
    for (const [k, v] of Object.entries(r)) {
      if (k === 'schema') out.content = content(produces, schema(v));
      else if (k === 'headers' && isObject(v)) {
        out.headers = Object.fromEntries(
          Object.entries(v).map(([n, h]) => [n, isObject(h) ? simpleParam(h) : h]),
        );
      } else if (k !== 'examples') out[k] = v;
    }
    return out;
  }

  function operation(op: Obj, pathParams: readonly Json[]): Obj {
    const consumes = strings(op.consumes) ?? rootConsumes;
    const produces = strings(op.produces) ?? rootProduces;
    const own = Array.isArray(op.parameters) ? op.parameters.map(resolve) : [];
    const ownKeys = new Set(own.filter(isObject).map((p) => `${String(p.in)}:${String(p.name)}`));
    // Path-level body and form parameters belong to each operation's request body.
    const inherited = pathParams
      .map(resolve)
      .filter((p) => isObject(p) && (p.in === 'body' || p.in === 'formData'))
      .filter((p) => !ownKeys.has(`${String((p as Obj).in)}:${String((p as Obj).name)}`));
    const all = [...inherited, ...own];
    const body = all.find((p) => isObject(p) && p.in === 'body') as Obj | undefined;
    const form = all.filter((p): p is Obj => isObject(p) && p.in === 'formData');
    const rest = own.filter((p) => !(isObject(p) && (p.in === 'body' || p.in === 'formData')));

    const out: Obj = {};
    for (const [k, v] of Object.entries(op)) {
      if (k === 'consumes' || k === 'produces') continue;
      if (k === 'parameters') {
        if (rest.length > 0) out[k] = rest.map((p) => param(p));
      } else if (k === 'responses' && isObject(v)) {
        out[k] = Object.fromEntries(Object.entries(v).map(([c, r]) => [c, response(r, produces)]));
      } else if (k.startsWith('x-')) out[k] = v;
      else out[k] = schema(v);
    }
    if (body !== undefined) {
      const types = consumes.filter((t) => !t.includes('form'));
      out.requestBody = {
        ...(typeof body.description === 'string' ? { description: body.description } : {}),
        ...(body.required === true ? { required: true } : {}),
        content: content(types.length > 0 ? types : ['application/json'], schema(body.schema)),
        [NORMALIZED_KEY]: ['swagger2-body'],
      };
    } else if (form.length > 0) {
      const multipart =
        form.some((p) => p.type === 'file') ||
        !consumes.includes('application/x-www-form-urlencoded');
      const required = form.filter((p) => p.required === true).map((p) => String(p.name));
      out.requestBody = {
        ...(required.length > 0 ? { required: true } : {}),
        content: content(
          [multipart ? 'multipart/form-data' : 'application/x-www-form-urlencoded'],
          {
            type: 'object',
            ...(required.length > 0 ? { required } : {}),
            properties: Object.fromEntries(form.map((p) => [String(p.name), formField(p)])),
          },
        ),
        [NORMALIZED_KEY]: ['swagger2-form-data'],
      };
    }
    return out;
  }

  function param(p: Json): Json {
    if (!isObject(p) || typeof p.$ref === 'string') return schema(p);
    return simpleParam(p);
  }

  const paths: Obj = {};
  for (const [path, item] of Object.entries(isObject(doc.paths) ? doc.paths : {})) {
    if (!isObject(item)) {
      paths[path] = item;
      continue;
    }
    const pathParams = Array.isArray(item.parameters) ? item.parameters : [];
    const out: Obj = {};
    for (const [k, v] of Object.entries(item)) {
      if (METHODS.has(k) && isObject(v)) out[k] = operation(v, pathParams);
      else if (k === 'parameters') {
        const kept = pathParams.filter((p) => {
          const r = resolve(p);
          return !(isObject(r) && (r.in === 'body' || r.in === 'formData'));
        });
        if (kept.length > 0) out[k] = kept.map(param);
      } else if (k.startsWith('x-')) out[k] = v;
      else out[k] = schema(v);
    }
    paths[path] = out;
  }

  const components: Obj = {};
  if (isObject(doc.definitions)) components.schemas = schema(doc.definitions);
  const params = Object.entries(globalParams).filter(
    ([, p]) => !(isObject(p) && (p.in === 'body' || p.in === 'formData')),
  );
  if (params.length > 0)
    components.parameters = Object.fromEntries(params.map(([n, p]) => [n, param(p)]));
  if (isObject(doc.responses)) {
    components.responses = Object.fromEntries(
      Object.entries(doc.responses).map(([n, r]) => [n, response(r, rootProduces)]),
    );
  }
  if (isObject(doc.securityDefinitions))
    components.securitySchemes = securitySchemes(doc.securityDefinitions);

  const out: Obj = { openapi: '3.0.3' };
  for (const [k, v] of Object.entries(doc)) {
    if (['swagger', 'host', 'basePath', 'schemes', 'consumes', 'produces'].includes(k)) continue;
    if (['definitions', 'parameters', 'responses', 'securityDefinitions'].includes(k)) continue;
    out[k] = k === 'paths' ? paths : v;
  }
  if (typeof doc.host === 'string' || typeof doc.basePath === 'string') {
    const schemes = strings(doc.schemes) ?? ['https'];
    const base = `${typeof doc.host === 'string' ? doc.host : ''}${typeof doc.basePath === 'string' ? doc.basePath : ''}`;
    out.servers = schemes.map((s) => ({
      url: typeof doc.host === 'string' ? `${s}://${base}` : base,
    }));
  }
  if (Object.keys(components).length > 0) out.components = components;
  out[NORMALIZED_KEY] = ['swagger2'];
  return {
    spec: out as OpenApiSpec,
    diagnostics: [
      {
        code: SWAGGER2_CODE,
        severity: 'info',
        message: 'Swagger 2.0 document converted to OpenAPI 3.0.3 before transforms',
        pointer: '/swagger',
        suggestion: 'convert the spec to OpenAPI 3 upstream to drop this step',
      },
    ],
  };
}

/** `basic` becomes HTTP basic; `oauth2` flows move under `flows`; `apiKey` is unchanged. */
function securitySchemes(defs: Obj): Obj {
  const out: Obj = {};
  for (const [name, d] of Object.entries(defs)) {
    if (!isObject(d)) {
      out[name] = d;
    } else if (d.type === 'basic') {
      const { type: _type, ...rest } = d;
      out[name] = { ...rest, type: 'http', scheme: 'basic' };
    } else if (d.type === 'oauth2') {
      const { flow, authorizationUrl, tokenUrl, scopes, ...rest } = d;
      const key = {
        implicit: 'implicit',
        password: 'password',
        application: 'clientCredentials',
        accessCode: 'authorizationCode',
      }[String(flow)];
      out[name] = {
        ...rest,
        flows: {
          [key ?? String(flow)]: {
            ...(authorizationUrl === undefined ? {} : { authorizationUrl }),
            ...(tokenUrl === undefined ? {} : { tokenUrl }),
            scopes: scopes ?? {},
          },
        },
      };
    } else {
      out[name] = d;
    }
  }
  return out;
}
