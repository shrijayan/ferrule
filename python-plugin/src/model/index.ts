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
  | { readonly kind: 'union'; readonly variants: readonly SdkType[] }
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
  /** The type of one item. */
  readonly item: SdkType;
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
  readonly pathParams: readonly SdkParam[];
  readonly queryParams: readonly SdkParam[];
  readonly body: SdkType | undefined;
  /** The decoded `2XX` JSON response, or undefined for none. */
  readonly response: SdkType | undefined;
  readonly pagination: SdkPagination | undefined;
}

export interface SdkResource {
  /** Python names from the client down, such as `['agencies_with_coverage']`. */
  readonly path: readonly string[];
  readonly methods: readonly SdkMethod[];
  readonly subresources: readonly SdkResource[];
}

export interface SdkPackage {
  /** Python import name, such as `onebusaway`. */
  readonly name: string;
  /** GitHub `owner/repo` used in README links, such as `OneBusAway/python-sdk`. */
  readonly repo: string;
}

export interface SdkClient {
  readonly package: SdkPackage;
  readonly baseUrl: string;
  readonly resources: readonly SdkResource[];
  /** Every named model the resources reference, sorted by module. */
  readonly models: readonly SdkModelDecl[];
}
