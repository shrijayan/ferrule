// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import Petstore from 'petstore';

const client = new Petstore({
  apiKey: 'My API Key',
  baseURL: process.env['TEST_API_BASE_URL'] ?? 'http://127.0.0.1:4010',
});

describe('resource photos', () => {
  test('delete: only required params', async () => {
    const responsePromise = client.pets.photos.delete('photo_id', { pet_id: 'pet_id' });
    const rawResponse = await responsePromise.asResponse();
    expect(rawResponse).toBeInstanceOf(Response);
    const response = await responsePromise;
    expect(response).not.toBeInstanceOf(Response);
    const dataAndResponse = await responsePromise.withResponse();
    expect(dataAndResponse.data).toBe(response);
    expect(dataAndResponse.response).toBe(rawResponse);
  });

  test('delete: required and optional params', async () => {
    const response = await client.pets.photos.delete('photo_id', { pet_id: 'pet_id' });
  });
});
