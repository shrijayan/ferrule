// File generated from our OpenAPI spec by Stainless. See CONTRIBUTING.md for details.

import type { __FerruleClient__ } from './index';

export abstract class APIResource {
  protected _client: __FerruleClient__;

  constructor(client: __FerruleClient__) {
    this._client = client;
  }
}
