/**
 * The only reader of `x-ferrule-*` extensions (ADR 0003). Validates their shapes and turns them
 * into typed values; anything malformed or unknown is a FER diagnostic.
 *
 * Untrusted input: extensions carry spec and config text into generated Python. Every value
 * that becomes a Python identifier (names, env vars, pagination fields) is checked here against
 * an identifier pattern (FER101); every string literal the emitters print goes through
 * `pyString`/`pyFString` (emit/common.ts), and every docstring through `escapeDocstring`.
 */
import type { Json } from '@cloudflare/forge/plugin';
import { FerruleError } from '../diagnostics.ts';
import type {
  PaginationScheme,
  SdkClientOpt,
  SdkPackage,
  SdkSecurityScheme,
} from '../model/index.ts';

export interface ModelName {
  readonly name: string;
  /** Dotted module under `types`; defaults to the snake_case of `name`. */
  readonly module: string | undefined;
}

export interface RawPagination {
  readonly scheme: PaginationScheme;
  readonly page: string;
  readonly paramLocation: 'query' | 'body';
  readonly request: Readonly<Record<string, string>>;
  readonly response: Readonly<Record<string, string>>;
  readonly headers: Readonly<Record<string, string>>;
}

export interface RawStreaming {
  readonly format: 'sse' | 'jsonl';
  readonly paramDiscriminator: string | null;
  /** The components.schemas name of the event model, when the config names one. */
  readonly eventSchema: string | undefined;
}

/** `x-ferrule-client` as the plugin reads it (ADR 0003, Phase 2 amendment). */
export interface RawClient {
  /** As written, such as `onebusaway-sdk`. */
  readonly name: string;
  readonly envPrefix: string;
  readonly opts: readonly SdkClientOpt[];
  readonly environments: readonly { readonly name: string; readonly url: string }[];
  readonly securitySchemes: Readonly<Record<string, SdkSecurityScheme>>;
  readonly security: readonly (readonly string[])[];
  readonly timeoutSeconds: number;
  readonly maxRetries: number;
  readonly idempotencyHeader: string | undefined;
  readonly defaultHeaders: Readonly<Record<string, string>>;
  readonly arrayFormat: 'comma' | 'repeat' | 'indices' | 'brackets';
  readonly nestedFormat: 'brackets' | 'dots';
}

/** The `x-ferrule-*` extensions an operation carries. */
export interface MethodExtensions {
  /** Resource path as written in stainless.yml; `['$client']` for client methods. */
  readonly resource: readonly string[];
  /** Index of each path segment among its siblings; empty when the converter wrote none. */
  readonly resourceOrder: readonly number[];
  readonly methodName: string;
  /** Index in its resource's `methods`; 0 when the converter wrote none. */
  readonly methodOrder: number;
  /** Mirrored from the root (Forge gap G2). */
  readonly package: SdkPackage;
  /** Mirrored from the root (Forge gap G2); absent in old fixtures. */
  readonly client: RawClient | undefined;
  /** Schema name -> Python model name (Forge gap G3). */
  readonly models: ReadonlyMap<string, ModelName>;
  readonly pagination: RawPagination | undefined;
  readonly streaming: RawStreaming | undefined;
}

/** Operation-level extensions this plugin reads. Root and schema ones never reach a plugin. */
const KNOWN = new Set([
  'x-ferrule-resource',
  'x-ferrule-method-name',
  'x-ferrule-method-order',
  'x-ferrule-package',
  'x-ferrule-models',
  'x-ferrule-pagination',
  'x-ferrule-streaming',
  'x-ferrule-client',
]);

const SCHEMES: readonly PaginationScheme[] = [
  'cursor',
  'cursor_id',
  'cursor_url',
  'offset',
  'page_number',
  'fake_page',
];

type JsonObject = { readonly [key: string]: Json };

function isObject(v: Json | undefined): v is JsonObject {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function isStringRecord(v: Json | undefined): v is { readonly [key: string]: string } {
  return isObject(v) && Object.values(v).every((x) => typeof x === 'string');
}

/** The members an extension object may have, typed so they read as properties. */
type Members = Partial<
  Record<
    | 'path'
    | 'order'
    | 'name'
    | 'repo'
    | 'module'
    | 'scheme'
    | 'page'
    | 'request'
    | 'response'
    | 'headers'
    | 'paramLocation'
    | 'format'
    | 'paramDiscriminator'
    | 'eventSchema'
    | 'envPrefix'
    | 'opts'
    | 'environments'
    | 'securitySchemes'
    | 'security'
    | 'timeoutSeconds'
    | 'retries'
    | 'maxRetries'
    | 'idempotencyHeader'
    | 'defaultHeaders'
    | 'query'
    | 'arrayFormat'
    | 'nestedFormat'
    | 'url'
    | 'type'
    | 'nullable'
    | 'default'
    | 'readEnv'
    | 'description'
    | 'auth'
    | 'send'
    | 'in'
    | 'securityScheme'
    | 'role'
    | 'keyword',
    Json
  >
>;

function members(v: Json | undefined): Members {
  return isObject(v) ? v : {};
}

/** A GitHub `owner/name`; it is interpolated into docstrings, so nothing else is allowed. */
const REPO_SLUG = /^(?:[\w.-]+\/[\w.-]+)?$/;
const PY_IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** `<PREFIX>_BASE_URL` and friends. */
const ENV_PREFIX = /^[A-Z][A-Z0-9_]*$/;
const ENV_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** An http(s) URL with nothing that could close or interpolate into a Python string. */
const HTTP_URL = /^https?:\/\/[^\s"'\\{}]+$/;

/** Pagination field paths: dotted identifiers, with `[]` stepping into array items. */
function fieldPaths(map: { readonly [key: string]: string }): boolean {
  return Object.keys(map).every((path) =>
    path.split('.').every((seg) => seg === '[]' || PY_IDENT.test(seg)),
  );
}
/** A stainless.yml resource key, snake_cased by the adapter (`trip-details`); `$client` is reserved. */
const RESOURCE_NAME = /^(?:\$client|[A-Za-z_][A-Za-z0-9_-]*)$/;

export function readMethodExtensions(
  extensions: readonly (readonly [string, Json])[],
  pointer: string,
): MethodExtensions {
  const found = new Map<string, Json>();
  for (const [key, value] of extensions) {
    if (!key.startsWith('x-ferrule-')) continue;
    if (!KNOWN.has(key)) {
      throw new FerruleError(
        'FER102',
        `${pointer}/${key}`,
        `unknown extension ${key}`,
        `remove it, or upgrade the plugin to one that knows ${key}`,
      );
    }
    found.set(key, value);
  }

  const bad = (key: string, shape: string): FerruleError =>
    new FerruleError(
      found.has(key) ? 'FER101' : 'FER103',
      `${pointer}/${key}`,
      found.has(key) ? `${key} is not ${shape}` : `missing ${key}`,
      `regenerate the annotated spec with ferrule-convert so ${key} is ${shape}`,
    );

  const { path, order } = members(found.get('x-ferrule-resource'));
  if (
    !Array.isArray(path) ||
    path.length === 0 ||
    !path.every((p) => typeof p === 'string' && RESOURCE_NAME.test(p))
  ) {
    throw bad('x-ferrule-resource', '{ path: string[] } of resource names');
  }
  if (
    order !== undefined &&
    (!Array.isArray(order) ||
      order.length !== path.length ||
      !order.every((n) => typeof n === 'number' && Number.isInteger(n) && n >= 0))
  ) {
    throw bad('x-ferrule-resource', '{ path: string[], order?: number[] } of equal length');
  }

  const methodName = found.get('x-ferrule-method-name');
  if (typeof methodName !== 'string' || !PY_IDENT.test(methodName)) {
    throw bad('x-ferrule-method-name', 'a Python identifier');
  }
  const methodOrder = found.get('x-ferrule-method-order') ?? 0;
  if (typeof methodOrder !== 'number' || !Number.isInteger(methodOrder) || methodOrder < 0) {
    throw bad('x-ferrule-method-order', 'a non-negative integer');
  }

  const pkg = members(found.get('x-ferrule-package'));
  if (
    typeof pkg.name !== 'string' ||
    !PY_IDENT.test(pkg.name) ||
    typeof pkg.repo !== 'string' ||
    !REPO_SLUG.test(pkg.repo)
  ) {
    throw bad('x-ferrule-package', '{ name: string, repo: "owner/name" or "" }');
  }

  const models = new Map<string, ModelName>();
  const rawModels = found.get('x-ferrule-models') ?? {};
  if (!isObject(rawModels)) throw bad('x-ferrule-models', 'Record<schema, { name, module? }>');
  for (const [schema, value] of Object.entries(rawModels)) {
    const { name, module } = members(value);
    if (
      typeof name !== 'string' ||
      !PY_IDENT.test(name) ||
      (module !== undefined && (typeof module !== 'string' || !/^[a-z_][a-z0-9_.]*$/.test(module)))
    ) {
      throw bad('x-ferrule-models', 'Record<schema, { name, module? }>');
    }
    models.set(schema, { name, module });
  }

  let pagination: RawPagination | undefined;
  const rawPage = found.get('x-ferrule-pagination');
  if (rawPage !== undefined) {
    const { scheme, page, request, response, headers, paramLocation } = members(rawPage);
    if (
      !SCHEMES.includes(scheme as PaginationScheme) ||
      typeof page !== 'string' ||
      !PY_IDENT.test(page) ||
      !isStringRecord(request) ||
      !isStringRecord(response) ||
      !fieldPaths(request) ||
      !fieldPaths(response) ||
      (headers !== undefined && !isStringRecord(headers)) ||
      (paramLocation !== undefined && paramLocation !== 'query' && paramLocation !== 'body')
    ) {
      throw bad(
        'x-ferrule-pagination',
        '{ scheme, page, paramLocation?, request: Record<string,string>, response: Record<string,string>, headers? }',
      );
    }
    pagination = {
      scheme: scheme as PaginationScheme,
      page,
      paramLocation: paramLocation ?? 'query',
      request,
      response,
      headers: headers ?? {},
    };
  }

  let streaming: RawStreaming | undefined;
  const rawStream = found.get('x-ferrule-streaming');
  if (rawStream !== undefined) {
    const { format, paramDiscriminator, eventSchema } = members(rawStream);
    if (
      (format !== 'sse' && format !== 'jsonl') ||
      (paramDiscriminator !== null &&
        paramDiscriminator !== undefined &&
        typeof paramDiscriminator !== 'string') ||
      (eventSchema !== undefined && typeof eventSchema !== 'string')
    ) {
      throw bad(
        'x-ferrule-streaming',
        '{ format: "sse" | "jsonl", paramDiscriminator, eventSchema? }',
      );
    }
    streaming = { format, paramDiscriminator: paramDiscriminator ?? null, eventSchema };
  }

  const rawClient = found.get('x-ferrule-client');
  const client =
    rawClient === undefined
      ? undefined
      : readClient(rawClient, () => bad('x-ferrule-client', 'the ADR 0003 client shape'));

  return {
    resource: path as string[],
    resourceOrder: (order as number[] | undefined) ?? [],
    methodName,
    methodOrder,
    package: { name: pkg.name, repo: pkg.repo },
    client,
    models,
    pagination,
    streaming,
  };
}

const OPT_TYPES = ['string', 'number', 'integer', 'boolean'] as const;
const SEND_IN = ['header', 'query', 'body', 'path'] as const;
const ROLES = ['value', 'username', 'password', 'client_id', 'client_secret'] as const;
const ARRAY_FORMATS = ['comma', 'repeat', 'indices', 'brackets'] as const;

function oneOf<T extends string>(values: readonly T[], v: Json | undefined): v is T {
  return typeof v === 'string' && (values as readonly string[]).includes(v);
}

function optString(v: Json | undefined): v is string | undefined {
  return v === undefined || typeof v === 'string';
}

function readOpt(v: Json, fail: () => FerruleError): SdkClientOpt {
  const m = members(v);
  const { name, type, nullable, readEnv, description, auth, send } = m;
  if (
    typeof name !== 'string' ||
    !PY_IDENT.test(name) ||
    !oneOf(OPT_TYPES, type) ||
    typeof nullable !== 'boolean' ||
    !optString(readEnv) ||
    (readEnv !== undefined && !ENV_NAME.test(readEnv)) ||
    !optString(description)
  ) {
    throw fail();
  }
  let authOf: SdkClientOpt['auth'];
  if (auth !== undefined) {
    const { securityScheme, role } = members(auth);
    if (typeof securityScheme !== 'string' || !oneOf(ROLES, role)) throw fail();
    authOf = { securityScheme, role };
  }
  let sendTo: SdkClientOpt['send'];
  if (send !== undefined) {
    const s = members(send);
    if (!oneOf(SEND_IN, s.in) || typeof s.name !== 'string') throw fail();
    sendTo = { in: s.in, name: s.name };
  }
  return {
    name,
    type,
    nullable,
    default: m.default,
    readEnv,
    description,
    auth: authOf,
    send: sendTo,
  };
}

function readScheme(v: Json, fail: () => FerruleError): SdkSecurityScheme {
  const m = members(v);
  if (m.type === 'http') {
    if (m.scheme !== 'bearer' && m.scheme !== 'digest' && m.scheme !== 'basic') throw fail();
    if (!optString(m.keyword)) throw fail();
    return m.keyword === undefined
      ? { type: 'http', scheme: m.scheme }
      : { type: 'http', scheme: m.scheme, keyword: m.keyword };
  }
  if (m.type === 'apiKey') {
    if ((m.in !== 'header' && m.in !== 'query' && m.in !== 'cookie') || typeof m.name !== 'string')
      throw fail();
    return { type: 'apiKey', in: m.in, name: m.name };
  }
  if (m.type === 'oauth2' || m.type === 'openIdConnect') return { type: m.type };
  throw fail();
}

/** Reads `x-ferrule-client`; `fail` builds the diagnostic for a malformed value. */
export function readClient(v: Json, fail: () => FerruleError): RawClient {
  const m = members(v);
  const { name, envPrefix, opts, environments, securitySchemes, security, timeoutSeconds } = m;
  if (
    typeof name !== 'string' ||
    typeof envPrefix !== 'string' ||
    !ENV_PREFIX.test(envPrefix) ||
    !Array.isArray(opts) ||
    !Array.isArray(environments) ||
    !isObject(securitySchemes) ||
    !Array.isArray(security) ||
    typeof timeoutSeconds !== 'number' ||
    !optString(m.idempotencyHeader) ||
    (m.defaultHeaders !== undefined && !isStringRecord(m.defaultHeaders))
  ) {
    throw fail();
  }
  const envs = environments.map((e) => {
    const { name: n, url } = members(e);
    if (typeof n !== 'string' || typeof url !== 'string' || !HTTP_URL.test(url)) throw fail();
    return { name: n, url };
  });
  const sec = security.map((alt) => {
    if (!isObject(alt)) throw fail();
    return Object.keys(alt);
  });
  const { maxRetries } = members(m.retries);
  const { arrayFormat, nestedFormat } = members(m.query);
  if (
    (maxRetries !== undefined && typeof maxRetries !== 'number') ||
    (arrayFormat !== undefined && !oneOf(ARRAY_FORMATS, arrayFormat)) ||
    (nestedFormat !== undefined && nestedFormat !== 'brackets' && nestedFormat !== 'dots')
  ) {
    throw fail();
  }
  return {
    name,
    envPrefix,
    opts: opts.map((o) => readOpt(o, fail)),
    environments: envs,
    securitySchemes: Object.fromEntries(
      Object.entries(securitySchemes).map(([k, s]) => [k, readScheme(s, fail)]),
    ),
    security: sec,
    timeoutSeconds,
    maxRetries: maxRetries ?? 2,
    idempotencyHeader: m.idempotencyHeader,
    defaultHeaders: m.defaultHeaders ?? {},
    arrayFormat: arrayFormat ?? 'repeat',
    nestedFormat: nestedFormat ?? 'brackets',
  };
}
