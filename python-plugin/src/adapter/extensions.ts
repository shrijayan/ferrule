/**
 * The only reader of `x-ferrule-*` extensions (ADR 0003). Validates their shapes and turns them
 * into typed values; anything malformed or unknown is a FER diagnostic.
 */
import type { Json } from '@cloudflare/forge/plugin';
import { FerruleError } from '../diagnostics.ts';
import type { PaginationScheme, SdkPackage } from '../model/index.ts';

export interface ModelName {
  readonly name: string;
  /** Dotted module under `types`; defaults to the snake_case of `name`. */
  readonly module: string | undefined;
}

export interface RawPagination {
  readonly scheme: PaginationScheme;
  readonly page: string;
  readonly request: Readonly<Record<string, string>>;
  readonly response: Readonly<Record<string, string>>;
}

/** The `x-ferrule-*` extensions an operation carries. */
export interface MethodExtensions {
  readonly resource: readonly string[];
  readonly methodName: string;
  /** Mirrored from the root (Forge gap G2). */
  readonly package: SdkPackage;
  /** Schema name -> Python model name (Forge gap G3). */
  readonly models: ReadonlyMap<string, ModelName>;
  readonly pagination: RawPagination | undefined;
}

/** Operation-level extensions this plugin reads. Root and schema ones never reach a plugin. */
const KNOWN = new Set([
  'x-ferrule-resource',
  'x-ferrule-method-name',
  'x-ferrule-package',
  'x-ferrule-models',
  'x-ferrule-pagination',
  'x-ferrule-streaming',
  // Read in Phase 3 (client class, method order); accepted now so converter output loads.
  'x-ferrule-client',
  'x-ferrule-method-order',
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
  Record<'path' | 'name' | 'repo' | 'module' | 'scheme' | 'page' | 'request' | 'response', Json>
>;

function members(v: Json | undefined): Members {
  return isObject(v) ? v : {};
}

/** A GitHub `owner/name`; it is interpolated into docstrings, so nothing else is allowed. */
const REPO_SLUG = /^(?:[\w.-]+\/[\w.-]+)?$/;
const PY_IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/;

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

  const { path } = members(found.get('x-ferrule-resource'));
  if (
    !Array.isArray(path) ||
    path.length === 0 ||
    !path.every((p) => typeof p === 'string' && PY_IDENT.test(p))
  ) {
    throw bad('x-ferrule-resource', '{ path: string[] } of Python identifiers');
  }

  const methodName = found.get('x-ferrule-method-name');
  if (typeof methodName !== 'string' || !PY_IDENT.test(methodName)) {
    throw bad('x-ferrule-method-name', 'a Python identifier');
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
    const { scheme, page, request, response } = members(rawPage);
    if (
      !SCHEMES.includes(scheme as PaginationScheme) ||
      typeof page !== 'string' ||
      !PY_IDENT.test(page) ||
      !isStringRecord(request) ||
      !isStringRecord(response)
    ) {
      throw bad(
        'x-ferrule-pagination',
        '{ scheme, page, request: Record<string,string>, response: Record<string,string> }',
      );
    }
    pagination = { scheme: scheme as PaginationScheme, page, request, response };
  }

  if (found.has('x-ferrule-streaming')) {
    throw new FerruleError(
      'FER201',
      `${pointer}/x-ferrule-streaming`,
      'streaming methods are not generated yet',
      'drop the method from the spike, or wait for the streaming phase',
    );
  }

  return {
    resource: path as string[],
    methodName,
    package: { name: pkg.name, repo: pkg.repo },
    models,
    pagination,
  };
}
