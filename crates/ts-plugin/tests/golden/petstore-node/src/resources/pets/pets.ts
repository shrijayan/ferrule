// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import { APIResource } from '../../resource';
import { isRequestOptions } from '../../core';
import * as Core from '../../core';
import * as Shared from '../shared';
import * as PhotosAPI from './photos';
import { Photos } from './photos';

/**
 * Pets in the store.
 */
export class Pets extends APIResource {
  photos: PhotosAPI.Photos = new PhotosAPI.Photos(this._client);

  /**
   * Adds a pet to the store.
   *
   * @example
   * ```ts
   * const pet = await client.pets.create({
   *   name: 'Rex',
   *   species: 'dog',
   * });
   * ```
   */
  create(body: PetCreateParams, options?: Core.RequestOptions): Core.APIPromise<Pet> {
    return this._client.post('/pets', { body, ...options });
  }

  /**
   * Gets one pet by its ID, with its vaccination record.
   */
  retrieve(petId: string, options?: Core.RequestOptions): Core.APIPromise<PetRetrieveResponse> {
    return this._client.get(`/pets/${petId}`, options);
  }

  /**
   * Lists the pets in the store, newest first.
   */
  list(query?: PetListParams, options?: Core.RequestOptions): Core.APIPromise<PetListResponse>;
  list(options?: Core.RequestOptions): Core.APIPromise<PetListResponse>;
  list(
    query: PetListParams | Core.RequestOptions = {},
    options?: Core.RequestOptions,
  ): Core.APIPromise<PetListResponse> {
    if (isRequestOptions(query)) {
      return this.list({}, query);
    }
    return this._client.get('/pets', { query, ...options });
  }
}

/**
 * A pet in the store.
 */
export interface Pet {
  /**
   * The pet's ID.
   */
  id: string;

  /**
   * The pet's name, which owners choose when they register the pet and which can be
   * changed later through the store's front desk.
   */
  name: string;

  species: 'dog' | 'cat';

  /**
   * A person who looks after pets.
   */
  owner?: Shared.Owner | null;
}

export interface PetRetrieveResponse {
  /**
   * A pet in the store.
   */
  pet: Pet;

  /**
   * Every vaccination the pet had.
   */
  vaccinations?: Array<PetRetrieveResponse.Vaccination>;
}

export namespace PetRetrieveResponse {
  export interface Vaccination {
    /**
     * The vaccine.
     */
    name: string;

    given_at?: string;
  }
}

export type PetListResponse = Array<Pet>;

export interface PetCreateParams {
  /**
   * The pet's name.
   */
  name: string;

  /**
   * What kind of animal it is.
   */
  species: 'dog' | 'cat';

  birthday?: string;

  /**
   * A person who looks after pets.
   */
  owner?: Shared.Owner | null;

  tags?: Array<string>;
}

export interface PetListParams {
  limit?: number;

  species?: 'dog' | 'cat';
}

Pets.Photos = Photos;

export declare namespace Pets {
  export {
    type Pet as Pet,
    type PetRetrieveResponse as PetRetrieveResponse,
    type PetListResponse as PetListResponse,
    type PetCreateParams as PetCreateParams,
    type PetListParams as PetListParams,
  };

  export { Photos as Photos };
}
