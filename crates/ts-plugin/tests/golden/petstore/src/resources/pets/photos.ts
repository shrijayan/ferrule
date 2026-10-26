// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import { APIResource } from '../../core/resource';
import { APIPromise } from '../../core/api-promise';
import { RequestOptions } from '../../internal/request-options';
import { path } from '../../internal/utils/path';

/**
 * Pets in the store.
 */
export class Photos extends APIResource {
  /**
   * Deletes one photo of a pet.
   *
   * @example
   * ```ts
   * await client.pets.photos.delete('photo_id', {
   *   pet_id: 'pet_id',
   * });
   * ```
   */
  delete(photoID: string, params: PhotoDeleteParams, options?: RequestOptions): APIPromise<void> {
    const { pet_id } = params;
    return this._client.delete(path`/pets/${pet_id}/photos/${photoID}`, options);
  }
}

export interface PhotoDeleteParams {
  pet_id: string;
}

export declare namespace Photos {
  export { type PhotoDeleteParams as PhotoDeleteParams };
}
