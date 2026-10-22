/**
 * Maps Forge's groups, methods, models and types to SdkModel (ADR 0002). With extensions.ts,
 * the only code that knows Forge's interface; it imports Forge's types and nothing else.
 */
import type {
  Field,
  Forge,
  Group,
  Method,
  ObjectType,
  ScalarType,
  Type,
} from '@cloudflare/forge/plugin';
import { FerruleError, operationPointer } from '../diagnostics.ts';
import type {
  HttpVerb,
  SdkClient,
  SdkClientSettings,
  SdkField,
  SdkMethod,
  SdkModelDecl,
  SdkObject,
  SdkPackage,
  SdkPagination,
  SdkParam,
  SdkRef,
  SdkResource,
  SdkStreaming,
  SdkType,
} from '../model/index.ts';
import { classSegment, pascalCase, snakeCase, splitWords } from '../naming.ts';
import { type MethodExtensions, type RawClient, readMethodExtensions } from './extensions.ts';

/** Generation state shared by every method converted into one SdkClient. */
interface Context {
  readonly decls: Map<string, SdkModelDecl>;
  /** Schema names being inlined, to reject recursive inline types. */
  readonly inlining: Set<string>;
  ext: MethodExtensions;
  pointer: string;
}

const VERBS: readonly HttpVerb[] = ['get', 'post', 'put', 'patch', 'delete'];

/** The reserved resource name of methods on the client class itself (ADR 0003). */
export const CLIENT_RESOURCE = '$client';

function unsupported(ctx: Context, what: string): FerruleError {
  return new FerruleError(
    'FER201',
    ctx.pointer,
    `${what} is not generated yet`,
    'leave the operation out, or name the schema in x-ferrule-models',
  );
}

function refFor(ctx: Context, schemaName: string): SdkRef | undefined {
  const named = ctx.ext.models.get(schemaName);
  if (named === undefined) return undefined;
  return {
    kind: 'ref',
    name: named.name,
    module: named.module ?? snakeCase(splitWords(named.name)),
  };
}

/**
 * An object schema written inline under a property carries its description on the property
 * (Forge has no object descriptions); Stainless prints it as the class docstring too.
 */
function inlineDescribed(type: SdkType, field: Field): SdkType {
  if (field.description === '') return type;
  if (field.type.kind === 'object' && type.kind === 'object') {
    return { ...type, description: field.description };
  }
  // `T | null`: the object variant of a nullable inline object.
  if (field.type.kind === 'union' && type.kind === 'union' && type.variants.length === 2) {
    const i = field.type.members.findIndex((m) => m.kind === 'object');
    const v = type.variants[i];
    if (i >= 0 && v?.kind === 'object') {
      const variants = [...type.variants];
      variants[i] = { ...v, description: field.description };
      return { ...type, variants };
    }
  }
  return type;
}

function toField(ctx: Context, field: Field): SdkField {
  return {
    wireName: field.wireName,
    pyName: snakeCase(field.name.words),
    required: field.required,
    description: field.description,
    type: inlineDescribed(toType(ctx, field.type), field),
    ...(field.deprecated ? { deprecated: true } : {}),
  };
}

function toObject(ctx: Context, type: ObjectType): SdkType {
  if (type.fields.length === 0) {
    return type.extra === undefined
      ? { kind: 'unknown' }
      : { kind: 'map', values: toType(ctx, type.extra) };
  }
  return { kind: 'object', bases: [], fields: type.fields.map((f) => toField(ctx, f)) };
}

/** `allOf`: named members become base classes, object members contribute fields. */
function toIntersection(ctx: Context, members: readonly Type[]): SdkObject {
  const bases: SdkRef[] = [];
  const fields: SdkField[] = [];
  for (const member of members) {
    const t = toType(ctx, member);
    if (t.kind === 'ref') bases.push(t);
    else if (t.kind === 'object') {
      bases.push(...t.bases);
      fields.push(...t.fields);
    } else if (t.kind !== 'unknown') throw unsupported(ctx, `an allOf member of kind ${t.kind}`);
  }
  return { kind: 'object', bases, fields };
}

function toScalar(type: ScalarType): SdkType {
  switch (type.scalar) {
    case 'unknown':
    // File uploads (`FileTypes`) need a model kind the project-file emitters know too; later.
    case 'binary':
      return { kind: 'unknown' };
    case 'null':
    case 'boolean':
    case 'date':
      return { kind: 'primitive', name: type.scalar };
    case 'date-time':
      return { kind: 'primitive', name: 'datetime' };
    case 'string':
    case 'uuid':
    case 'bytes':
      return { kind: 'primitive', name: 'string' };
    case 'number':
      return {
        kind: 'primitive',
        name: type.number === 'float' || type.number === 'double' ? 'number' : 'integer',
      };
  }
}

function toType(ctx: Context, type: Type): SdkType {
  switch (type.kind) {
    case 'scalar':
      return toScalar(type);
    case 'enum':
      return { kind: 'literal', values: type.values };
    case 'array':
      return { kind: 'array', items: toType(ctx, type.elem) };
    case 'object':
      return toObject(ctx, type);
    case 'union':
      return {
        kind: 'union',
        variants: type.members.map((m) => toType(ctx, m)),
        ...(type.discriminator === undefined ? {} : { discriminator: type.discriminator.wireName }),
      };
    case 'intersection':
      return toIntersection(ctx, type.members);
    case 'model': {
      const { model } = type;
      const ref = refFor(ctx, model.schemaName);
      if (ref !== undefined) {
        if (!ctx.decls.has(model.schemaName)) {
          // Reserve the slot first so a model that refers to itself terminates.
          ctx.decls.set(model.schemaName, {
            ref,
            description: model.description,
            type: { kind: 'object', bases: [], fields: [] },
          });
          const body = toType(ctx, model.type);
          if (body.kind !== 'object') throw unsupported(ctx, `named model of kind ${body.kind}`);
          ctx.decls.set(model.schemaName, { ref, description: model.description, type: body });
        }
        return ref;
      }
      if (ctx.inlining.has(model.schemaName)) {
        throw unsupported(ctx, `recursive schema ${model.schemaName} without x-ferrule-models`);
      }
      ctx.inlining.add(model.schemaName);
      const inlined = toType(ctx, model.type);
      ctx.inlining.delete(model.schemaName);
      if (inlined.kind !== 'object') return inlined;
      return {
        ...inlined,
        hint: model.schemaName,
        ...(model.description === '' ? {} : { description: model.description }),
      };
    }
  }
}

function toParams(ctx: Context, method: Method): { path: SdkParam[]; query: SdkParam[] } {
  const path: SdkParam[] = [];
  const query: SdkParam[] = [];
  const { request, body } = method;
  let params: Type | undefined;
  if (request?.kind === 'intersection' && body !== undefined) params = request.members[0];
  else if (body === undefined) params = request;
  if (params === undefined) return { path, query };
  if (params.kind !== 'object') throw unsupported(ctx, `parameters of kind ${params.kind}`);
  for (const field of params.fields) {
    const p = toField(ctx, field);
    const loc = method.location(field);
    if (loc?.in === 'path') path.push(p);
    else query.push(p);
  }
  // Path parameters in the order the path template names them.
  const at = (p: SdkParam): number => method.path.indexOf(`{${p.wireName}}`);
  path.sort((a, b) => at(a) - at(b));
  return { path, query };
}

function objectOf(ctx: Context, type: SdkType | undefined): SdkObject | undefined {
  if (type?.kind === 'object') return type;
  if (type?.kind === 'ref') {
    for (const decl of ctx.decls.values()) {
      if (decl.ref.module === type.module && decl.ref.name === type.name) return decl.type;
    }
  }
  return undefined;
}

function toPagination(ctx: Context, response: SdkType | undefined): SdkPagination | undefined {
  const raw = ctx.ext.pagination;
  if (raw === undefined) return undefined;
  // Maps run spec field name -> purpose (converter contract); `items` marks the page items.
  const itemsWire = Object.entries(raw.response).find(([, purpose]) => purpose === 'items')?.[0];
  const items = objectOf(ctx, response)?.fields.find((f) => f.wireName === itemsWire);
  if (items?.type.kind !== 'array') {
    throw new FerruleError(
      'FER101',
      `${ctx.pointer}/x-ferrule-pagination/response`,
      `no array response field has purpose "items" (got ${itemsWire ?? 'none'})`,
      'map the response property holding the page items to "items"',
    );
  }
  return {
    scheme: raw.scheme,
    page: pascalCase(splitWords(raw.page)),
    paramLocation: raw.paramLocation,
    request: raw.request,
    response: raw.response,
    headers: raw.headers,
    item: items.type.items,
  };
}

function toStreaming(ctx: Context, response: SdkType | undefined): SdkStreaming | undefined {
  const raw = ctx.ext.streaming;
  if (raw === undefined) return undefined;
  const event = raw.eventSchema === undefined ? response : refFor(ctx, raw.eventSchema);
  return { format: raw.format, paramDiscriminator: raw.paramDiscriminator, event };
}

function toMethod(ctx: Context, method: Method): SdkMethod {
  const verb = method.httpMethod.toLowerCase() as HttpVerb;
  if (!VERBS.includes(verb)) throw unsupported(ctx, `HTTP method ${method.httpMethod}`);
  const { path, query } = toParams(ctx, method);
  const content = method.body?.json ?? method.body?.contents[0];
  const body = content === undefined ? undefined : toType(ctx, content.type);
  const responseType = method.responses.find((r) => r.json !== undefined)?.json?.type;
  const response = responseType === undefined ? undefined : toType(ctx, responseType);
  return {
    name: ctx.ext.methodName,
    pointer: ctx.pointer,
    httpMethod: verb,
    path: method.path,
    summary: method.summary,
    description: method.description,
    order: ctx.ext.methodOrder,
    deprecated: method.deprecated,
    pathParams: path,
    queryParams: query,
    headerParams: (method.headers?.fields ?? []).map((f) => toField(ctx, f)),
    body,
    bodyParams: objectOf(ctx, body)?.fields ?? [],
    bodyMediaType: content?.mediaType,
    response,
    pagination: toPagination(ctx, response),
    streaming: toStreaming(ctx, response),
  };
}

interface MutableResource {
  path: string[];
  order: number;
  methods: SdkMethod[];
  children: Map<string, MutableResource>;
}

function byOrder<T extends { readonly order: number }>(items: Iterable<T>): T[] {
  return [...items].sort((a, b) => a.order - b.order);
}

function freeze(r: MutableResource): SdkResource {
  return {
    path: r.path,
    order: r.order,
    methods: byOrder(r.methods),
    subresources: byOrder([...r.children.values()].map(freeze)),
  };
}

/** The client class name: `onebusaway-sdk` -> `OnebusawaySDK`. */
export function clientClassName(name: string): string {
  return classSegment(splitWords(name));
}

function toSettings(raw: RawClient): SdkClientSettings {
  return {
    name: raw.name,
    className: clientClassName(raw.name),
    envPrefix: raw.envPrefix,
    opts: raw.opts,
    environments: raw.environments,
    securitySchemes: raw.securitySchemes,
    security: raw.security,
    timeoutSeconds: raw.timeoutSeconds,
    maxRetries: raw.maxRetries,
    idempotencyHeader: raw.idempotencyHeader,
    defaultHeaders: raw.defaultHeaders,
    arrayFormat: raw.arrayFormat,
    nestedFormat: raw.nestedFormat,
  };
}

/** Converts `methods` into one SdkClient; `baseUrl` is empty where Forge doesn't expose it. */
function toClient(methods: readonly Method[], baseUrl: string): SdkClient | undefined {
  const root: MutableResource = { path: [], order: 0, methods: [], children: new Map() };
  const clientMethods: SdkMethod[] = [];
  const decls = new Map<string, SdkModelDecl>();
  let pkg: SdkPackage | undefined;
  let settings: SdkClientSettings | undefined;
  for (const method of methods) {
    const pointer = operationPointer(method.path, method.httpMethod);
    const ext = readMethodExtensions(method.extensions, pointer);
    if (pkg !== undefined && (pkg.name !== ext.package.name || pkg.repo !== ext.package.repo)) {
      throw new FerruleError(
        'FER101',
        `${pointer}/x-ferrule-package`,
        'operations disagree on x-ferrule-package',
        'mirror the same root x-ferrule-config onto every operation',
      );
    }
    pkg = ext.package;
    if (settings === undefined && ext.client !== undefined) settings = toSettings(ext.client);
    const ctx: Context = { decls, inlining: new Set(), ext, pointer };
    const converted = toMethod(ctx, method);
    if (ext.resource[0] === CLIENT_RESOURCE) {
      clientMethods.push(converted);
      continue;
    }
    let node = root;
    ext.resource.forEach((written, i) => {
      const name = snakeCase(splitWords(written));
      let child = node.children.get(name);
      if (child === undefined) {
        child = {
          path: [...node.path, name],
          order: ext.resourceOrder[i] ?? 0,
          methods: [],
          children: new Map(),
        };
        node.children.set(name, child);
      }
      node = child;
    });
    node.methods.push(converted);
  }
  if (pkg === undefined) return undefined;
  const models = [...decls.values()].sort((a, b) =>
    a.ref.module < b.ref.module ? -1 : a.ref.module > b.ref.module ? 1 : 0,
  );
  return {
    package: pkg,
    baseUrl,
    settings,
    resources: freeze(root).subresources,
    clientMethods: byOrder(clientMethods),
    clientClass: settings?.className,
    envPrefix: settings?.envPrefix,
    models,
  };
}

/** The whole document; undefined when it has no operations to read the package from. */
export function forgeToModel(forge: Forge): SdkClient | undefined {
  return toClient(
    forge.groups.flatMap((g) => g.methods),
    forge.baseUrl,
  );
}

/** One group's own methods (not its children's, which get calls of their own). */
export function groupToModel(group: Group): SdkClient | undefined {
  return toClient(group.methods, '');
}
