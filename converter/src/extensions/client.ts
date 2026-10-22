/**
 * `x-ferrule-client` (ADR 0003): client class settings from stainless.yml `organization`,
 * `client_settings`, `environments`, `security`, `security_schemes` and `query_settings`, plus
 * the security schemes and servers of the spec. Pure.
 */
import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import { arr, at, bool, isObject, type Json, num, obj, str } from './json.ts';

export type OptType = 'string' | 'number' | 'integer' | 'boolean';
export type SendLocation = 'header' | 'query' | 'body' | 'path';

/** A `client_settings.opts` entry, in config order. */
export interface FerruleClientOpt {
  readonly name: string;
  readonly type: OptType;
  /** `nullable: true` makes the option optional. */
  readonly nullable: boolean;
  readonly default?: unknown;
  readonly readEnv?: string;
  readonly description?: string;
  readonly auth?: { readonly securityScheme: string; readonly role: string };
  readonly send?: { readonly in: SendLocation; readonly name: string };
  readonly serverVariable?: string;
}

export type FerruleSecurityScheme =
  | { readonly type: 'http'; readonly scheme: 'bearer' | 'digest'; readonly keyword?: string }
  | {
      readonly type: 'http';
      readonly scheme: 'basic';
      readonly keyword?: string;
      readonly disableBase64: boolean;
    }
  | { readonly type: 'apiKey'; readonly in: 'header' | 'query' | 'cookie'; readonly name: string }
  | { readonly type: 'oauth2'; readonly flows: Json }
  | { readonly type: 'openIdConnect'; readonly url: string };

export interface FerruleClient {
  /** `client_settings.default_client_name`, else `organization.name`, as written. */
  readonly name: string;
  /** `client_settings.default_env_prefix`, else UPPER_SNAKE of `organization.name`. */
  readonly envPrefix: string;
  readonly opts: readonly FerruleClientOpt[];
  /** In config order; the first is the default. */
  readonly environments: readonly { readonly name: string; readonly url: string }[];
  readonly defaultEnvironment?: string;
  /** Spec `components.securitySchemes` overlaid with `security_schemes` from the config. */
  readonly securitySchemes: Readonly<Record<string, FerruleSecurityScheme>>;
  /** `security` from the config, else from the spec. */
  readonly security: readonly Readonly<Record<string, readonly string[]>>[];
  readonly timeoutSeconds: number;
  readonly retries: {
    readonly maxRetries: number;
    readonly initialDelaySeconds: number;
    readonly maxDelaySeconds: number;
  };
  readonly idempotencyHeader?: string;
  readonly defaultHeaders: Readonly<Record<string, string>>;
  /** `client_settings.omit_stainless_headers`. */
  readonly omitPlatformHeaders: boolean;
  readonly query: {
    readonly nestedFormat: 'brackets' | 'dots';
    readonly arrayFormat: 'comma' | 'repeat' | 'indices' | 'brackets';
  };
  readonly httpLimits?: {
    readonly maxConnections?: number;
    readonly maxKeepaliveConnections?: number;
    readonly keepaliveExpiry?: number;
  };
}

/** Stainless defaults (public schema): 60 s timeout, 2 retries, 0.5 s to 8 s backoff. */
export const DEFAULT_TIMEOUT_SECONDS = 60;
export const DEFAULT_RETRIES = { maxRetries: 2, initialDelaySeconds: 0.5, maxDelaySeconds: 8 };
/** Environment name used when the config has no `environments` and the spec has a server. */
export const DEFAULT_ENVIRONMENT_NAME = 'production';

const OPT_TYPES: readonly OptType[] = ['string', 'number', 'integer', 'boolean'];
const SENDS: readonly [string, SendLocation][] = [
  ['send_in_header', 'header'],
  ['send_as_query_param', 'query'],
  ['send_as_body_param', 'body'],
  ['send_as_path_param', 'path'],
];

/** `onebusaway-sdk` -> `ONEBUSAWAY_SDK`; `AcmeCorp` -> `ACME_CORP`. */
export function upperSnake(name: string): string {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
    .split(/[^A-Za-z0-9]+/)
    .filter((w) => w !== '')
    .join('_')
    .toUpperCase();
}

const ISO_DURATION =
  /^P(?:(\d+(?:\.\d+)?)D)?(?:T(?:(\d+(?:\.\d+)?)H)?(?:(\d+(?:\.\d+)?)M)?(?:(\d+(?:\.\d+)?)S)?)?$/;

/** `default_timeout`: milliseconds, or an ISO 8601 duration such as `PT60S`. Seconds out. */
export function timeoutSeconds(value: unknown): number | undefined {
  if (typeof value === 'number')
    return value > 0 && Number.isFinite(value) ? value / 1000 : undefined;
  if (typeof value !== 'string') return undefined;
  const m = ISO_DURATION.exec(value);
  if (m === null || value === 'P' || value.endsWith('T')) return undefined;
  const [, d = '0', h = '0', min = '0', s = '0'] = m;
  const total = Number(d) * 86400 + Number(h) * 3600 + Number(min) * 60 + Number(s);
  return total > 0 ? total : undefined;
}

function readOpts(settings: Json, diagnostics: Diagnostic[]): FerruleClientOpt[] {
  const out: FerruleClientOpt[] = [];
  for (const [name, raw] of Object.entries(obj(settings, 'opts'))) {
    const o = isObject(raw) ? raw : {};
    const pointer = toJsonPointer(['client_settings', 'opts', name]);
    const type = at(o, 'type');
    if (!OPT_TYPES.includes(type as OptType)) {
      diagnostics.push({
        code: 'FER1301',
        severity: 'error',
        message: `Client option ${name} has type ${JSON.stringify(type)}.`,
        pointer: `${pointer}/type`,
        suggestion: `Set type to one of: ${OPT_TYPES.join(', ')}.`,
      });
      continue;
    }
    const sends = SENDS.flatMap(([key, loc]) => {
      const v = str(o, key);
      return v === undefined ? [] : [{ in: loc, name: v, key }];
    });
    if (sends.length > 1) {
      diagnostics.push({
        code: 'FER1302',
        severity: 'warning',
        message: `Client option ${name} is sent in ${sends.length} places; only ${sends[0]?.key} is used.`,
        pointer,
        suggestion:
          'Keep one of send_in_header, send_as_query_param, send_as_body_param, send_as_path_param.',
      });
    }
    const auth = obj(o, 'auth');
    const scheme = str(auth, 'security_scheme');
    const readEnv = str(o, 'read_env');
    const description = str(o, 'description');
    const serverVariable = str(o, 'server_variable');
    const send = sends[0];
    out.push({
      name,
      type: type as OptType,
      nullable: bool(o, 'nullable') ?? false,
      ...(Object.hasOwn(o, 'default') ? { default: at(o, 'default') } : {}),
      ...(readEnv === undefined ? {} : { readEnv }),
      ...(description === undefined ? {} : { description }),
      ...(scheme === undefined
        ? {}
        : { auth: { securityScheme: scheme, role: str(auth, 'role') ?? 'value' } }),
      ...(send === undefined ? {} : { send: { in: send.in, name: send.name } }),
      ...(serverVariable === undefined ? {} : { serverVariable }),
    });
  }
  return out;
}

function readScheme(raw: unknown): FerruleSecurityScheme | undefined {
  const s = isObject(raw) ? raw : {};
  const type = str(s, 'type');
  const scheme = str(s, 'scheme')?.toLowerCase();
  const auth = obj(s, 'x-stainless-auth');
  const keyword = str(auth, 'scheme_keyword');
  const kw = keyword === undefined ? {} : { keyword };
  if (type === 'http' && (scheme === 'bearer' || scheme === 'digest')) {
    return { type, scheme, ...kw };
  }
  if (type === 'http' && scheme === 'basic') {
    return { type, scheme, ...kw, disableBase64: bool(auth, 'disable_base64') ?? false };
  }
  const name = str(s, 'name');
  const where = str(s, 'in') ?? 'header';
  if (type === 'apiKey' && name !== undefined && ['header', 'query', 'cookie'].includes(where)) {
    return { type, in: where as 'header' | 'query' | 'cookie', name };
  }
  if (type === 'oauth2') return { type, flows: obj(s, 'flows') };
  const url = str(s, 'openIdConnectUrl');
  if (type === 'openIdConnect' && url !== undefined) return { type, url };
  return undefined;
}

function readSchemes(
  spec: Json,
  config: Json,
  diagnostics: Diagnostic[],
): Record<string, FerruleSecurityScheme> {
  const out: Record<string, FerruleSecurityScheme> = {};
  const sources: [string[], Json][] = [
    [['components', 'securitySchemes'], obj(obj(spec, 'components'), 'securitySchemes')],
    [['security_schemes'], obj(config, 'security_schemes')],
  ];
  for (const [where, schemes] of sources) {
    for (const [name, raw] of Object.entries(schemes)) {
      const scheme = readScheme(raw);
      if (scheme === undefined) {
        diagnostics.push({
          code: 'FER1303',
          severity: 'warning',
          message: `Security scheme ${name} is not an http bearer/basic/digest, apiKey, oauth2 or openIdConnect scheme.`,
          pointer: toJsonPointer([...where, name]),
          suggestion: 'Fix the scheme, or override it under security_schemes in stainless.yml.',
        });
        continue;
      }
      out[name] = scheme;
    }
  }
  return out;
}

function readSecurity(list: unknown[]): Record<string, string[]>[] {
  return list.filter(isObject).map((req) => {
    const out: Record<string, string[]> = {};
    for (const [name, scopes] of Object.entries(req)) {
      out[name] = Array.isArray(scopes) ? scopes.filter((s) => typeof s === 'string') : [];
    }
    return out;
  });
}

function readEnvironments(spec: Json, config: Json): { name: string; url: string }[] {
  const configured = Object.entries(obj(config, 'environments')).flatMap(([name, url]) =>
    typeof url === 'string' ? [{ name, url }] : [],
  );
  if (configured.length > 0) return configured;
  const servers = arr(spec, 'servers');
  const url = isObject(servers[0]) ? str(servers[0], 'url') : undefined;
  return url === undefined ? [] : [{ name: DEFAULT_ENVIRONMENT_NAME, url }];
}

function readHttpLimits(settings: Json): FerruleClient['httpLimits'] {
  const python = obj(settings, 'python');
  if (!isObject(at(python, 'default_http_limits'))) return undefined;
  const limits = obj(python, 'default_http_limits');
  const maxConnections = num(limits, 'max_connections');
  const maxKeepaliveConnections = num(limits, 'max_keepalive_connections');
  const keepaliveExpiry = num(limits, 'keepalive_expiry');
  return {
    ...(maxConnections === undefined ? {} : { maxConnections }),
    ...(maxKeepaliveConnections === undefined ? {} : { maxKeepaliveConnections }),
    ...(keepaliveExpiry === undefined ? {} : { keepaliveExpiry }),
  };
}

const NESTED = ['brackets', 'dots'] as const;
const ARRAYS = ['comma', 'repeat', 'indices', 'brackets'] as const;

function pick<T extends string>(value: unknown, allowed: readonly T[], fallback: T): T {
  return allowed.includes(value as T) ? (value as T) : fallback;
}

/** Build `x-ferrule-client` from the (untyped) config and the spec. Pure. */
export function buildClient(
  spec: Json,
  config: Json,
): { client: FerruleClient; diagnostics: Diagnostic[] } {
  const diagnostics: Diagnostic[] = [];
  const settings = obj(config, 'client_settings');
  const org = str(obj(config, 'organization'), 'name');
  const python = obj(obj(config, 'targets'), 'python');
  const fallback = org ?? str(python, 'package_name') ?? '';
  const name = str(settings, 'default_client_name') ?? fallback;
  const envPrefix = str(settings, 'default_env_prefix') ?? upperSnake(fallback);

  const opts = readOpts(settings, diagnostics);
  const securitySchemes = readSchemes(spec, config, diagnostics);
  for (const opt of opts) {
    if (opt.auth !== undefined && !Object.hasOwn(securitySchemes, opt.auth.securityScheme)) {
      diagnostics.push({
        code: 'FER1304',
        severity: 'error',
        message: `Client option ${opt.name} authenticates with security scheme ${opt.auth.securityScheme}, which is not defined.`,
        pointer: toJsonPointer(['client_settings', 'opts', opt.name, 'auth', 'security_scheme']),
        suggestion: `Use one of: ${Object.keys(securitySchemes).join(', ') || '(none defined)'}, or add it under security_schemes.`,
      });
    }
  }

  let timeout = DEFAULT_TIMEOUT_SECONDS;
  if (Object.hasOwn(settings, 'default_timeout')) {
    const parsed = timeoutSeconds(at(settings, 'default_timeout'));
    if (parsed === undefined) {
      diagnostics.push({
        code: 'FER1305',
        severity: 'error',
        message: `default_timeout ${JSON.stringify(at(settings, 'default_timeout'))} is not a positive number of milliseconds or an ISO 8601 duration.`,
        pointer: '/client_settings/default_timeout',
        suggestion: 'Write milliseconds (60000) or an ISO 8601 duration (PT60S).',
      });
    } else timeout = parsed;
  }

  const retries = obj(settings, 'default_retries');
  const environments = readEnvironments(spec, config);
  const security = arr(config, 'security');
  const idempotencyHeader = str(obj(settings, 'idempotency'), 'header');
  const defaultHeaders: Record<string, string> = {};
  for (const [k, v] of Object.entries(obj(settings, 'default_headers'))) {
    if (typeof v === 'string') defaultHeaders[k] = v;
  }
  const query = obj(config, 'query_settings');
  const httpLimits = readHttpLimits(settings);
  const defaultEnvironment = environments[0]?.name;

  return {
    client: {
      name,
      envPrefix,
      opts,
      environments,
      ...(defaultEnvironment === undefined ? {} : { defaultEnvironment }),
      securitySchemes,
      security: readSecurity(Object.hasOwn(config, 'security') ? security : arr(spec, 'security')),
      timeoutSeconds: timeout,
      retries: {
        maxRetries: num(retries, 'max_retries') ?? DEFAULT_RETRIES.maxRetries,
        initialDelaySeconds:
          num(retries, 'initial_delay_seconds') ?? DEFAULT_RETRIES.initialDelaySeconds,
        maxDelaySeconds: num(retries, 'max_delay_seconds') ?? DEFAULT_RETRIES.maxDelaySeconds,
      },
      ...(idempotencyHeader === undefined ? {} : { idempotencyHeader }),
      defaultHeaders,
      omitPlatformHeaders: bool(settings, 'omit_stainless_headers') ?? false,
      query: {
        nestedFormat: pick(at(query, 'nested_format'), NESTED, 'brackets'),
        arrayFormat: pick(at(query, 'array_format'), ARRAYS, 'comma'),
      },
      ...(httpLimits === undefined ? {} : { httpLimits }),
    },
    diagnostics,
  };
}
