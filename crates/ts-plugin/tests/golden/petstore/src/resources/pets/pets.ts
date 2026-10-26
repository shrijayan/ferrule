// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import { APIResource } from '../../core/resource';
import * as Shared from '../shared';
import * as PhotosAPI from './photos';
import { PhotoDeleteParams, Photos } from './photos';
import { APIPromise } from '../../core/api-promise';
import { RequestOptions } from '../../internal/request-options';
import { path } from '../../internal/utils/path';

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
  create(body: PetCreateParams, options?: RequestOptions): APIPromise<Pet> {
    return this._client.post('/pets', { body, ...options });
  }

  /**
   * Gets one pet by its ID, with its vaccination record.
   *
   * @example
   * ```ts
   * const pet = await client.pets.retrieve('pet_id');
   * ```
   */
  retrieve(petID: string, options?: RequestOptions): APIPromise<PetRetrieveResponse> {
    return this._client.get(path`/pets/${petID}`, options);
  }

  /**
   * Lists the pets in the store, newest first.
   *
   * @example
   * ```ts
   * const pets = await client.pets.list();
   * ```
   */
  list(query: PetListParams | null | undefined = {}, options?: RequestOptions): APIPromise<PetListResponse> {
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

  export { Photos as Photos, type PhotoDeleteParams as PhotoDeleteParams };
}
