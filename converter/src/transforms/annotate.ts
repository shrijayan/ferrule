import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import { DEFAULT_EDITION } from '../editions/index.ts';
import { buildClient, type FerruleClient } from '../extensions/client.ts';
import { CLIENT_GROUP, collidesWithClientGroup, forgeGroupPath } from '../extensions/groups.ts';
import { at, deref, isObject, type Json, obj, str } from '../extensions/json.ts';
import {
  bestMatch,
  type FerrulePagination,
  operationShape,
  readSchemes,
  type Scheme,
} from '../extensions/pagination.ts';
import { buildStreaming, type FerruleOnEvent, readOnEvent } from '../extensions/streaming.ts';
import type { Config, Method, Resource } from '../schema/config.ts';

export type { FerrulePagination } from '../extensions/pagination.ts';

/** `x-ferrule-models` entry (ADR 0003, gap G3): Python class name and dotted module under `types`. */
export interface FerruleModel {
  readonly name: string;
  readonly module: string;
}

interface Ctx {
  readonly spec: Json;
  readonly config: Json;
  readonly diagnostics: Diagnostic[];
  readonly models: Record<string, FerruleModel>;
  readonly operations: Json[];
  readonly schemes: readonly Scheme[];
  readonly onEvent: readonly FerruleOnEvent[];
}

function resourcePointer(path: readonly string[]): (string | number)[] {
  return ['resources', ...path.flatMap((seg, i) => (i === 0 ? [seg] : ['subresources', seg]))];
}

function methodPointer(path: readonly string[], name: string): string {
  return toJsonPointer([...resourcePointer(path), 'methods', name]);
}

function findOperation(spec: Json, endpoint: string): { pathItem: Json; op: Json } | undefined {
  const m = /^\s*(\w+)\s+(\S+)\s*$/.exec(endpoint);
  if (m === null) return undefined;
  const [, verb = '', route = ''] = m;
  const pathItem = deref(spec, obj(spec, 'paths')[route]);
  const op = isObject(pathItem) ? pathItem[verb.toLowerCase()] : undefined;
  return isObject(pathItem) && isObject(op) ? { pathItem, op } : undefined;
}

interface Position {
  /** Index of each path segment among its siblings, in stainless.yml order. */
  readonly resourceOrder: readonly number[];
  /** Index of the method within its resource, in stainless.yml order. */
  readonly methodOrder: number;
}

function annotateMethod(
  ctx: Ctx,
  path: readonly string[],
  name: string,
  method: Method,
  position: Position,
): void {
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
  op['x-fern-sdk-group-name'] = forgeGroupPath(path);
  op['x-fern-sdk-method-name'] = name;
  op['x-ferrule-resource'] = { path: [...path], order: [...position.resourceOrder] };
  op['x-ferrule-method-name'] = name;
  op['x-ferrule-method-order'] = position.methodOrder;
  ctx.operations.push(op);
  const pagination = resolvePagination(ctx, pointer, name, method, pathItem, op);
  if (pagination !== undefined) op['x-ferrule-pagination'] = pagination;
  if (typeof method !== 'string') {
    const { streaming, diagnostics } = buildStreaming({
      spec: ctx.spec,
      config: ctx.config,
      op,
      streaming: at(method, 'streaming'),
      pointer,
      onEvent: ctx.onEvent,
    });
    ctx.diagnostics.push(...diagnostics);
    if (streaming !== undefined) op['x-ferrule-streaming'] = streaming;
  }
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
  const { schemes } = ctx;
  if (typeof paginated === 'string') {
    const named = schemes.find((p) => p.extension.page === paginated);
    if (named !== undefined) return named.extension;
    ctx.diagnostics.push({
      code: 'FER1002',
      severity: 'error',
      message: `Pagination "${paginated}" is not in the top-level pagination list.`,
      pointer: `${pointer}/paginated`,
      suggestion: `Add a pagination entry named "${paginated}", or use one of: ${
        schemes.map((p) => p.extension.page).join(', ') || '(none defined)'
      }.`,
    });
    return undefined;
  }
  if (paginated !== true && name !== 'list' && !name.startsWith('list_')) return undefined;
  const scheme = bestMatch(ctx.spec, schemes, operationShape(ctx.spec, pathItem, op));
  if (scheme !== undefined) return scheme.extension;
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

const pascal = (snake: string): string =>
  snake
    .split('_')
    .filter((w) => w !== '')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');

/** Models of a resource: `$shared` ones live in `types/shared/`, others under their resource path. */
function collectModels(ctx: Ctx, path: readonly string[], resource: Resource): void {
  for (const [key, model] of Object.entries(resource.models ?? {})) {
    const pointer = toJsonPointer([...resourcePointer(path), 'models', key]);
    if (typeof model !== 'string') {
      ctx.diagnostics.push({
        code: 'FER1004',
        severity: 'warning',
        message: `Model ${[...path, key].join('.')} uses the object form, which is not supported yet.`,
        pointer,
        suggestion: 'Write the model as a schema name, e.g. `references: Reference`.',
      });
      continue;
    }
    const schema = model.replace(/^#\/components\/schemas\//, '');
    if (!Object.hasOwn(obj(obj(ctx.spec, 'components'), 'schemas'), schema)) {
      ctx.diagnostics.push({
        code: 'FER1005',
        severity: 'error',
        message: `Model ${[...path, key].join('.')} names schema "${schema}", which is not in components.schemas.`,
        pointer,
        suggestion: 'Name a schema under components.schemas in the OpenAPI spec.',
      });
      continue;
    }
    // types/<parent dirs>/<key>.py: a resource's models sit in its parent's directory.
    const dirs = path[0] === '$shared' ? ['shared'] : path.slice(0, -1);
    ctx.models[schema] = { name: pascal(key), module: [...dirs, key].join('.') };
  }
  for (const [name, sub] of Object.entries(resource.subresources ?? {})) {
    collectModels(ctx, [...path, name], sub);
  }
}

function walk(
  ctx: Ctx,
  path: readonly string[],
  order: readonly number[],
  resource: Resource,
): void {
  Object.entries(resource.methods ?? {}).forEach(([name, method], i) => {
    annotateMethod(ctx, path, name, method, { resourceOrder: order, methodOrder: i });
  });
  Object.entries(resource.subresources ?? {}).forEach(([name, sub], i) => {
    walk(ctx, [...path, name], [...order, i], sub);
  });
}

/** `x-fern-type-name` on every schema `models:` names, so Forge's model names match (gap G3). */
function nameSchemas(ctx: Ctx): void {
  const schemas = obj(obj(ctx.spec, 'components'), 'schemas');
  for (const [schema, model] of Object.entries(ctx.models)) {
    const target = schemas[schema];
    if (isObject(target)) target['x-fern-type-name'] = model.name;
  }
}

function checkClientGroup(ctx: Ctx, resources: Readonly<Record<string, Resource>>): void {
  for (const name of Object.keys(resources)) {
    if (!collidesWithClientGroup(name)) continue;
    ctx.diagnostics.push({
      code: 'FER1006',
      severity: 'error',
      message: `Resource ${name} lands in the same Forge group as $client methods (${CLIENT_GROUP}).`,
      pointer: toJsonPointer(['resources', name]),
      suggestion: 'Rename the resource; Forge splits names into words, so this one is reserved.',
    });
  }
}

/** Annotate a deep copy of `spec` from `config`. Pure: inputs are never mutated. */
export function annotate(
  spec: Readonly<Record<string, unknown>>,
  config: Config,
): { spec: Json; diagnostics: Diagnostic[] } {
  const out = structuredClone(spec) as Json;
  const raw = config as Json;
  const { schemes, diagnostics } = readSchemes(raw);
  const ctx: Ctx = {
    spec: out,
    config: raw,
    diagnostics,
    models: {},
    operations: [],
    schemes,
    onEvent: readOnEvent(raw),
  };
  const resources = config.resources ?? {};
  checkClientGroup(ctx, resources);
  for (const [name, resource] of Object.entries(resources)) {
    collectModels(ctx, [name], resource);
  }
  Object.entries(resources).forEach(([name, resource], i) => {
    walk(ctx, [name], [i], resource);
  });
  nameSchemas(ctx);
  const python = config.targets?.python;
  // Root settings are written for a Python target only; without one the spec is left as is.
  const pkg =
    python?.package_name === undefined
      ? undefined
      : { name: python.package_name, repo: python.production_repo ?? '' };
  let client: FerruleClient | undefined;
  if (pkg !== undefined) {
    const built = buildClient(out, raw);
    ctx.diagnostics.push(...built.diagnostics);
    client = built.client;
    out['x-ferrule-config'] = {
      version: 1,
      edition: str(raw, 'edition') ?? DEFAULT_EDITION,
      packageName: pkg.name,
      projectName: python?.project_name ?? pkg.name,
    };
    out['x-ferrule-client'] = client;
  }
  // Forge drops root and schema extensions (gaps G2, G3), so mirror them onto every operation.
  for (const op of ctx.operations) {
    op['x-ferrule-models'] = ctx.models;
    if (pkg !== undefined) op['x-ferrule-package'] = pkg;
    if (client !== undefined) op['x-ferrule-client'] = client;
  }
  return { spec: out, diagnostics: ctx.diagnostics };
}
