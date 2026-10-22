/**
 * SdkModel: Ferrule's own, Forge-independent view of an SDK (ADR 0002).
 *
 * It describes the SDK, not the spec: names are already Python spellings, inline objects are
 * still anonymous (the types emitter names them by path, as Stainless does), and named models
 * are references into `SdkClient.models`.
 */

export type SdkPrimitive =
  | 'string'
  | 'integer'
  | 'number'
  | 'boolean'
  | 'null'
  | 'datetime'
  | 'date';

export type SdkType =
  | { readonly kind: 'primitive'; readonly name: SdkPrimitive }
  | { readonly kind: 'unknown' }
  | { readonly kind: 'array'; readonly items: SdkType }
  | { readonly kind: 'map'; readonly values: SdkType }
  | {
      readonly kind: 'union';
      readonly variants: readonly SdkType[];
      /** The wire name of the discriminating property, if the spec names one. */
      readonly discriminator?: string;
    }
  | { readonly kind: 'literal'; readonly values: readonly (string | number | boolean)[] }
  | SdkRef
  | SdkObject;

/** A named model: Python class `name` declared in `types/<module as path>.py`. */
export interface SdkRef {
  readonly kind: 'ref';
  readonly name: string;
  /** Dotted module under the `types` package, such as `shared.references`. */
  readonly module: string;
}

/** An object type. Inline ones are named by the emitter from their position. */
export interface SdkObject {
  readonly kind: 'object';
  /** Named models this one extends (`allOf` with a named model). */
  readonly bases: readonly SdkRef[];
  readonly fields: readonly SdkField[];
  /** The description of the schema it was inlined from, for a class docstring. */
  readonly description?: string;
  /** The components.schemas name it was inlined from, which names union variants. */
  readonly hint?: string;
}

export interface SdkField {
  /** The property name as sent. */
  readonly wireName: string;
  /** The Python attribute name, such as `lat_span` or `from_`. */
  readonly pyName: string;
  readonly required: boolean;
  readonly description: string;
  readonly type: SdkType;
}

/** A named model declared in its own module. */
export interface SdkModelDecl {
  readonly ref: SdkRef;
  readonly description: string;
  readonly type: SdkObject;
}

export interface SdkParam {
  readonly wireName: string;
  readonly pyName: string;
  readonly required: boolean;
  readonly description: string;
  readonly type: SdkType;
  /** Whether the spec marks it deprecated. */
  readonly deprecated?: boolean;
  /**
   * The spec's `example`, `minimum` and `maximum` (for README and test examples). Forge does not
   * expose them yet (docs/forge-gaps.md G18), so the adapter leaves them unset.
   */
  readonly example?: unknown;
  readonly minimum?: number;
  readonly maximum?: number;
}

export type PaginationScheme =
  | 'cursor'
  | 'cursor_id'
  | 'cursor_url'
  | 'offset'
  | 'page_number'
  | 'fake_page';

export interface SdkPagination {
  readonly scheme: PaginationScheme;
  /** Page class without the Sync/Async prefix, such as `CursorPage`. */
  readonly page: string;
  /** Wire query parameter -> purpose, such as `{ after: 'next_cursor_param' }`. */
  readonly request: Readonly<Record<string, string>>;
  /** Wire response field -> purpose; the field with purpose `items` holds the page items. */
  readonly response: Readonly<Record<string, string>>;
  /** Where the request fields go. */
  readonly paramLocation: 'query' | 'body';
  /** Response header name -> purpose, for `from_header` properties. */
  readonly headers: Readonly<Record<string, string>>;
  /** The type of one item. */
  readonly item: SdkType;
}

/** `x-ferrule-streaming`: the method returns `Stream[event]` (ADR 0003). */
export interface SdkStreaming {
  readonly format: 'sse' | 'jsonl';
  /** Body param that switches streaming on (an overload pair), or null when it always streams. */
  readonly paramDiscriminator: string | null;
  /** The type of one event: the named event model, else the 2xx response. */
  readonly event: SdkType | undefined;
}

export type HttpVerb = 'get' | 'post' | 'put' | 'patch' | 'delete';

export interface SdkMethod {
  /** Python method name, such as `list`. */
  readonly name: string;
  /** JSON pointer of the operation, for diagnostics. */
  readonly pointer: string;
  readonly httpMethod: HttpVerb;
  readonly path: string;
  readonly summary: string;
  readonly description: string;
  /** Position in its resource's `methods` (stainless.yml order). */
  readonly order: number;
  readonly deprecated: boolean;
  /** Path parameters in the order they appear in `path`. */
  readonly pathParams: readonly SdkParam[];
  readonly queryParams: readonly SdkParam[];
  readonly headerParams: readonly SdkParam[];
  /** The JSON (or multipart) request body; its object fields are `bodyParams`. */
  readonly body: SdkType | undefined;
  /** The top-level properties of an object body, as keyword arguments. */
  readonly bodyParams: readonly SdkParam[];
  /** The request body's media type, such as `application/json`. */
  readonly bodyMediaType: string | undefined;
  /** The decoded `2XX` JSON response, or undefined for none. */
  readonly response: SdkType | undefined;
  readonly pagination: SdkPagination | undefined;
  readonly streaming: SdkStreaming | undefined;
}

export interface SdkResource {
  /** Python names from the client down, such as `['agencies_with_coverage']`. */
  readonly path: readonly string[];
  /** Position among its siblings in stainless.yml. */
  readonly order: number;
  readonly methods: readonly SdkMethod[];
  readonly subresources: readonly SdkResource[];
}

export interface SdkPackage {
  /** Python import name, such as `onebusaway`. */
  readonly name: string;
  /** GitHub `owner/repo` used in README links, such as `OneBusAway/python-sdk`. */
  readonly repo: string;
}

/** One `client_settings.opts` entry: a client constructor argument. */
export interface SdkClientOpt {
  /** Python argument name, such as `api_key`. */
  readonly name: string;
  readonly type: 'string' | 'number' | 'integer' | 'boolean';
  readonly nullable: boolean;
  readonly default: unknown;
  /** Environment variable read when the argument is not given. */
  readonly readEnv: string | undefined;
  readonly description: string | undefined;
  readonly auth:
    | {
        readonly securityScheme: string;
        readonly role: 'value' | 'username' | 'password' | 'client_id' | 'client_secret';
      }
    | undefined;
  /** Where the client sends it on every request, if anywhere. */
  readonly send:
    | { readonly in: 'header' | 'query' | 'body' | 'path'; readonly name: string }
    | undefined;
}

export type SdkSecurityScheme =
  | {
      readonly type: 'http';
      readonly scheme: 'bearer' | 'digest' | 'basic';
      readonly keyword?: string;
    }
  | { readonly type: 'apiKey'; readonly in: 'header' | 'query' | 'cookie'; readonly name: string }
  | { readonly type: 'oauth2' | 'openIdConnect' };

/** Client class settings from `x-ferrule-client` (ADR 0003). */
export interface SdkClientSettings {
  /** `organization.name` (or `default_client_name`) as written, such as `onebusaway-sdk`. */
  readonly name: string;
  /** The client class, such as `OnebusawaySDK`. */
  readonly className: string;
  /** Prefix of `<PREFIX>_BASE_URL` and friends, such as `ONEBUSAWAY_SDK`. */
  readonly envPrefix: string;
  readonly opts: readonly SdkClientOpt[];
  /** In config order; the first is the default. */
  readonly environments: readonly { readonly name: string; readonly url: string }[];
  readonly securitySchemes: Readonly<Record<string, SdkSecurityScheme>>;
  /** Alternatives, each a set of scheme names that apply together. */
  readonly security: readonly (readonly string[])[];
  readonly timeoutSeconds: number;
  readonly maxRetries: number;
  readonly idempotencyHeader: string | undefined;
  readonly defaultHeaders: Readonly<Record<string, string>>;
  readonly arrayFormat: 'comma' | 'repeat' | 'indices' | 'brackets';
  readonly nestedFormat: 'brackets' | 'dots';
}

export interface SdkClient {
  readonly package: SdkPackage;
  readonly baseUrl: string;
  /** Undefined in a fixture without `x-ferrule-client`. */
  readonly settings: SdkClientSettings | undefined;
  /** Top-level resources in stainless.yml order. */
  readonly resources: readonly SdkResource[];
  /** `$client` methods, emitted on the client class. */
  readonly clientMethods: readonly SdkMethod[];
  /** The client class, such as `OnebusawaySDK` (`settings.className`, for the file emitters). */
  readonly clientClass: string | undefined;
  /** Prefix of the SDK's env vars, such as `ONEBUSAWAY_SDK` (`settings.envPrefix`). */
  readonly envPrefix: string | undefined;
  /** Every named model the resources reference, sorted by module. */
  readonly models: readonly SdkModelDecl[];
}
