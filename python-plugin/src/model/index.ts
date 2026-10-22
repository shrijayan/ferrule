/** SdkModel: Ferrule's own, Forge-independent view of an SDK. */

export type SdkType =
  | {
      readonly kind: 'primitive';
      readonly name: 'string' | 'integer' | 'number' | 'boolean' | 'null';
    }
  | { readonly kind: 'unknown' }
  | { readonly kind: 'array'; readonly items: SdkType }
  | { readonly kind: 'map'; readonly values: SdkType }
  | { readonly kind: 'optional'; readonly inner: SdkType }
  | { readonly kind: 'union'; readonly variants: readonly SdkType[] }
  | { readonly kind: 'literal'; readonly values: readonly (string | number | boolean)[] }
  | { readonly kind: 'ref'; readonly name: readonly string[]; readonly module: string };

export interface SdkMethod {
  readonly name: readonly string[];
  readonly httpMethod: 'get' | 'post' | 'put' | 'patch' | 'delete';
  readonly path: string;
  readonly params: SdkType | undefined;
  readonly response: SdkType;
}

export interface SdkResource {
  readonly name: readonly string[];
  readonly methods: readonly SdkMethod[];
  readonly subresources: readonly SdkResource[];
}

export interface SdkClient {
  readonly name: readonly string[];
  readonly packageName: string;
  readonly baseUrl: string;
  readonly resources: readonly SdkResource[];
}
