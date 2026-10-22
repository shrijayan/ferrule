import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import { DEFAULT_EDITION } from '../editions/index.ts';
import type { Config, Method, Pagination, Resource } from '../schema/config.ts';

type Json = Record<string, unknown>;

/** `x-ferrule-pagination` (ADR 0003). Field maps are spec field name -> pagination purpose. */
export interface FerrulePagination {
  readonly scheme: string;
  readonly page: string;
  readonly request: Record<string, string>;
  readonly response: Record<string, string>;
}

const isObject = (v: unknown): v is Json =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

/** Typed field readers (keep biome's useLiteralKeys and tsc's index-signature rule apart). */
const obj = (o: Json, k: string): Json => {
  const v = o[k];
  return isObject(v) ? v : {};
};
const arr = (o: Json, k: string): unknown[] => {
  const v = o[k];
  return Array.isArray(v) ? v : [];
};
const str = (o: Json, k: string): string | undefined => {
  const v = o[k];
  return typeof v === 'string' ? v : undefined;
};

/** Resolve a local `#/...` $ref chain (cycle-safe); other nodes are returned as is. */
function deref(spec: Json, node: unknown, seen: ReadonlySet<string> = new Set()): unknown {
  const ref = isObject(node) ? str(node, '$ref') : undefined;
  if (ref === undefined) return node;
  if (!ref.startsWith('#/') || seen.has(ref)) return undefined;
  let cur: unknown = spec;
  for (const raw of ref.slice(2).split('/')) {
    cur = isObject(cur) ? cur[raw.replaceAll('~1', '/').replaceAll('~0', '~')] : undefined;
  }
  return deref(spec, cur, new Set([...seen, ref]));
}

/** Property names of a schema, following $ref and allOf. */
function propertyNames(spec: Json, schema: unknown, depth = 0): Set<string> {
  const out = new Set<string>();
  const s = deref(spec, schema);
  if (!isObject(s) || depth > 32) return out;
  for (const k of Object.keys(obj(s, 'properties'))) out.add(k);
  for (const part of arr(s, 'allOf')) {
    for (const k of propertyNames(spec, part, depth + 1)) out.add(k);
  }
  return out;
}

function queryParamNames(spec: Json, pathItem: Json, op: Json): Set<string> {
  const out = new Set<string>();
  for (const p of [...arr(pathItem, 'parameters'), ...arr(op, 'parameters')]) {
    const param = deref(spec, p);
    const name = isObject(param) && str(param, 'in') === 'query' ? str(param, 'name') : undefined;
    if (name !== undefined) out.add(name);
  }
  return out;
}

function responseFieldNames(spec: Json, op: Json): Set<string> {
  const responses = obj(op, 'responses');
  const code = Object.keys(responses)
    .filter((c) => c.startsWith('2'))
    .sort()[0];
  const resp = code === undefined ? undefined : deref(spec, responses[code]);
  const { schema } = isObject(resp) ? obj(obj(resp, 'content'), 'application/json') : {};
  return propertyNames(spec, schema);
}

function fieldMap(fields: Pagination['request']): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [name, def] of Object.entries(fields ?? {})) {
    out[name] = def?.['x-stainless-pagination-property']?.purpose ?? name;
  }
  return out;
}

function toExtension(p: Pagination): FerrulePagination {
  return {
    scheme: p.type,
    page: p.name,
    request: fieldMap(p.request),
    response: fieldMap(p.response),
  };
}

/** Stainless list-method rule: every scheme request field is a query param, every response field a top-level property. */
function matches(p: Pagination, query: Set<string>, response: Set<string>): boolean {
  return (
    Object.keys(p.request ?? {}).every((k) => query.has(k)) &&
    Object.keys(p.response ?? {}).every((k) => response.has(k))
  );
}

interface Ctx {
  readonly spec: Json;
  readonly config: Config;
  readonly diagnostics: Diagnostic[];
  readonly models: Record<string, FerruleModel>;
  readonly operations: Json[];
}

function methodPointer(path: readonly string[], name: string): string {
  const segs = path.flatMap((seg, i) => (i === 0 ? [seg] : ['subresources', seg]));
  return toJsonPointer(['resources', ...segs, 'methods', name]);
}

function findOperation(spec: Json, endpoint: string): { pathItem: Json; op: Json } | undefined {
  const m = /^\s*(\w+)\s+(\S+)\s*$/.exec(endpoint);
  if (m === null) return undefined;
  const [, verb = '', route = ''] = m;
  const pathItem = deref(spec, obj(spec, 'paths')[route]);
  const op = isObject(pathItem) ? pathItem[verb.toLowerCase()] : undefined;
  return isObject(pathItem) && isObject(op) ? { pathItem, op } : undefined;
}

function annotateMethod(ctx: Ctx, path: readonly string[], name: string, method: Method): void {
  const pointer = methodPointer(path, name);
  const endpoint = typeof method === 'string' ? method : method.endpoint;
  const found = findOperation(ctx.spec, endpoint);
  if (found === undefined) {
    ctx.diagnostics.push({
      code: 'FER1001',
      severity: 'error',
      message: `Endpoint "${endpoint}" of method ${[...path, name].join('.')} is not in the OpenAPI spec.`,
      pointer,
      suggestion: 'Use "<verb> <path>" naming an operation in the spec, or remove the method.',
    });
    return;
  }
  const { pathItem, op } = found;
  op['x-fern-sdk-group-name'] = [...path];
  op['x-fern-sdk-method-name'] = name;
  op['x-ferrule-resource'] = { path: [...path] };
  op['x-ferrule-method-name'] = name;
  ctx.operations.push(op);
  const pagination = resolvePagination(ctx, pointer, name, method, pathItem, op);
  if (pagination !== undefined) op['x-ferrule-pagination'] = pagination;
}

function resolvePagination(
  ctx: Ctx,
  pointer: string,
  name: string,
  method: Method,
  pathItem: Json,
  op: Json,
): FerrulePagination | undefined {
  const paginated = typeof method === 'string' ? undefined : method.paginated;
  if (paginated === false) return undefined;
  const schemes = ctx.config.pagination ?? [];
  if (typeof paginated === 'string') {
    const named = schemes.find((p) => p.name === paginated);
    if (named !== undefined) return toExtension(named);
    ctx.diagnostics.push({
      code: 'FER1002',
      severity: 'error',
      message: `Pagination "${paginated}" is not in the top-level pagination list.`,
      pointer: `${pointer}/paginated`,
      suggestion: `Add a pagination entry named "${paginated}", or use one of: ${
        schemes.map((p) => p.name).join(', ') || '(none defined)'
      }.`,
    });
    return undefined;
  }
  if (paginated !== true && name !== 'list' && !name.startsWith('list_')) return undefined;
  const query = queryParamNames(ctx.spec, pathItem, op);
  const response = responseFieldNames(ctx.spec, op);
  const scheme = schemes.find((p) => matches(p, query, response));
  if (scheme !== undefined) return toExtension(scheme);
  if (paginated === true) {
    ctx.diagnostics.push({
      code: 'FER1003',
      severity: 'error',
      message: 'Method is marked paginated but matches no configured pagination scheme.',
      pointer: `${pointer}/paginated`,
      suggestion:
        'Name the scheme with `paginated: <name>`, or align the operation fields with a pagination entry.',
    });
  }
  return undefined;
}

/** `x-ferrule-models` entry (ADR 0003, gap G3): Python class name and dotted module under `types`. */
export interface FerruleModel {
  readonly name: string;
  readonly module: string;
}

const pascal = (snake: string): string =>
  snake
    .split('_')
    .filter((w) => w !== '')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');

/** Models of a resource: `$shared` ones live in `types/shared/`, others under their resource path. */
function collectModels(ctx: Ctx, path: readonly string[], resource: Resource): void {
  for (const [key, model] of Object.entries(resource.models ?? {})) {
    if (typeof model !== 'string') {
      ctx.diagnostics.push({
        code: 'FER1004',
        severity: 'warning',
        message: `Model ${[...path, key].join('.')} uses the object form, which is not supported yet.`,
        pointer: toJsonPointer([
          'resources',
          ...path.flatMap((p, i) => (i === 0 ? [p] : ['subresources', p])),
          'models',
          key,
        ]),
        suggestion: 'Write the model as a schema name, e.g. `references: Reference`.',
      });
      continue;
    }
    const schema = model.replace(/^#\/components\/schemas\//, '');
    // types/<parent dirs>/<key>.py: a resource's models sit in its parent's directory.
    const dirs = path[0] === '$shared' ? ['shared'] : path.slice(0, -1);
    ctx.models[schema] = { name: pascal(key), module: [...dirs, key].join('.') };
  }
  for (const [name, sub] of Object.entries(resource.subresources ?? {})) {
    collectModels(ctx, [...path, name], sub);
  }
}

function walk(ctx: Ctx, path: readonly string[], resource: Resource): void {
  for (const [name, method] of Object.entries(resource.methods ?? {})) {
    annotateMethod(ctx, path, name, method);
  }
  for (const [name, sub] of Object.entries(resource.subresources ?? {})) {
    walk(ctx, [...path, name], sub);
  }
}

/** Annotate a deep copy of `spec` from `config`. Pure: inputs are never mutated. */
export function annotate(
  spec: Readonly<Record<string, unknown>>,
  config: Config,
): { spec: Json; diagnostics: Diagnostic[] } {
  const out = structuredClone(spec) as Json;
  const ctx: Ctx = { spec: out, config, diagnostics: [], models: {}, operations: [] };
  for (const [name, resource] of Object.entries(config.resources ?? {})) {
    collectModels(ctx, [name], resource);
  }
  for (const [name, resource] of Object.entries(config.resources ?? {})) {
    walk(ctx, [name], resource);
  }
  const python = config.targets?.python;
  // Forge drops root and schema extensions (gaps G2, G3), so mirror them onto every operation.
  const pkg =
    python?.package_name === undefined
      ? undefined
      : { name: python.package_name, repo: python.production_repo ?? '' };
  for (const op of ctx.operations) {
    if (pkg !== undefined) op['x-ferrule-package'] = pkg;
    op['x-ferrule-models'] = ctx.models;
  }
  if (python?.package_name !== undefined) {
    out['x-ferrule-config'] = {
      version: 1,
      edition: config.edition ?? DEFAULT_EDITION,
      packageName: python.package_name,
      projectName: python.project_name ?? python.package_name,
    };
  }
  return { spec: out, diagnostics: ctx.diagnostics };
}
