// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import { APIResource } from '../../resource';
import * as Core from '../../core';

/**
 * Pets in the store.
 */
export class Photos extends APIResource {
  /**
   * Deletes one photo of a pet.
   */
  delete(petId: string, photoId: string, options?: Core.RequestOptions): Core.APIPromise<void> {
    return this._client.delete(`/pets/${petId}/photos/${photoId}`, {
      ...options,
      headers: { Accept: '*/*', ...options?.headers },
    });
  }
}
