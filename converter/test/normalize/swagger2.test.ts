import { describe, expect, it } from 'vitest';
import { type Config, convert } from '../../src/index.ts';
import { swagger2ToOpenApi3 } from '../../src/swagger2/index.ts';

// biome-ignore lint/suspicious/noExplicitAny: test access into untyped JSON
type AnyJson = any;

// Reductions of the slack, docker, kubernetes and azure-storage corpus specs (Swagger 2.0), which
// the pinned Forge reader refused: `unknown variant "body"` / `"formData"`.
describe('swagger2ToOpenApi3', () => {
  const docker = {
    swagger: '2.0',
    host: 'localhost',
    basePath: '/v1.41',
    schemes: ['http', 'https'],
    consumes: ['application/json'],
    produces: ['application/json'],
    definitions: {
      ContainerConfig: {
        type: 'object',
        properties: { Image: { type: 'string', 'x-nullable': true } },
      },
      Container: { allOf: [{ $ref: '#/definitions/ContainerConfig' }], discriminator: 'Kind' },
    },
    parameters: {
      limit: { name: 'limit', in: 'query', type: 'integer', default: 10 },
      config: {
        name: 'config',
        in: 'body',
        required: true,
        schema: { $ref: '#/definitions/ContainerConfig' },
      },
    },
    responses: { NotFound: { description: 'no such thing', schema: { type: 'string' } } },
    securityDefinitions: { key: { type: 'apiKey', in: 'header', name: 'X-Key' } },
    paths: {
      '/containers/create': {
        parameters: [{ $ref: '#/parameters/limit' }],
        post: {
          operationId: 'ContainerCreate',
          parameters: [
            { name: 'name', in: 'query', type: 'string', pattern: '^/?[a-z]+$' },
            {
              name: 'ids',
              in: 'query',
              type: 'array',
              items: { type: 'string' },
              collectionFormat: 'csv',
            },
            { name: 'X-Trace', in: 'header', type: 'string' },
            { $ref: '#/parameters/config' },
          ],
          responses: {
            '201': {
              description: 'created',
              schema: { $ref: '#/definitions/Container' },
              headers: { 'X-Id': { type: 'string' } },
            },
            '204': { description: 'empty' },
            '404': { $ref: '#/responses/NotFound' },
          },
        },
      },
    },
  };

  it('converts the document skeleton and components (docker)', () => {
    const { spec, diagnostics } = swagger2ToOpenApi3(docker);
    const out = spec as AnyJson;
    expect(out.swagger).toBeUndefined();
    expect(out.openapi).toBe('3.0.3');
    expect(out.servers).toEqual([
      { url: 'http://localhost/v1.41' },
      { url: 'https://localhost/v1.41' },
    ]);
    expect(out.components.schemas.ContainerConfig.properties.Image).toEqual({
      type: 'string',
      nullable: true,
    });
    expect(out.components.schemas.Container).toEqual({
      allOf: [{ $ref: '#/components/schemas/ContainerConfig' }],
      discriminator: { propertyName: 'Kind' },
    });
    expect(out.components.parameters).toEqual({
      limit: { name: 'limit', in: 'query', schema: { type: 'integer', default: 10 } },
    });
    expect(out.components.responses.NotFound).toEqual({
      description: 'no such thing',
      content: { 'application/json': { schema: { type: 'string' } } },
    });
    expect(out.components.securitySchemes).toEqual({
      key: { type: 'apiKey', in: 'header', name: 'X-Key' },
    });
    expect(out['x-ferrule-normalized']).toEqual(['swagger2']);
    expect(diagnostics).toEqual([
      expect.objectContaining({ code: 'FER1610', severity: 'info', pointer: '/swagger' }),
    ]);
    expect((docker as AnyJson).definitions.ContainerConfig.properties.Image['x-nullable']).toBe(
      true,
    );
  });

  it('turns body, query and header parameters into OpenAPI 3 shapes (docker)', () => {
    const op = (swagger2ToOpenApi3(docker).spec as AnyJson).paths['/containers/create'].post;
    expect(op.parameters).toEqual([
      { name: 'name', in: 'query', schema: { type: 'string', pattern: '^/?[a-z]+$' } },
      {
        name: 'ids',
        in: 'query',
        style: 'form',
        explode: false,
        schema: { type: 'array', items: { type: 'string' } },
      },
      { name: 'X-Trace', in: 'header', schema: { type: 'string' } },
    ]);
    expect(op.requestBody).toEqual({
      required: true,
      content: { 'application/json': { schema: { $ref: '#/components/schemas/ContainerConfig' } } },
      'x-ferrule-normalized': ['swagger2-body'],
    });
    expect(op.responses['201']).toEqual({
      description: 'created',
      headers: { 'X-Id': { schema: { type: 'string' } } },
      content: { 'application/json': { schema: { $ref: '#/components/schemas/Container' } } },
    });
    expect(op.responses['204']).toEqual({ description: 'empty' });
    expect(op.responses['404']).toEqual({ $ref: '#/components/responses/NotFound' });
    expect(
      (swagger2ToOpenApi3(docker).spec as AnyJson).paths['/containers/create'].parameters,
    ).toEqual([{ $ref: '#/components/parameters/limit' }]);
  });

  it('collects formData parameters into a form request body (slack)', () => {
    const slack = {
      swagger: '2.0',
      paths: {
        '/files.upload': {
          post: {
            consumes: ['application/x-www-form-urlencoded', 'multipart/form-data'],
            parameters: [
              {
                name: 'token',
                in: 'formData',
                type: 'string',
                required: true,
                description: 'Auth token',
              },
              { name: 'file', in: 'formData', type: 'file' },
              { name: 'channels', in: 'formData', type: 'string' },
            ],
            responses: { '200': { description: 'ok', schema: { type: 'object' } } },
          },
        },
        '/chat.postMessage': {
          post: {
            consumes: ['application/x-www-form-urlencoded'],
            produces: ['application/json'],
            parameters: [{ name: 'text', in: 'formData', type: 'string' }],
            responses: { default: { description: 'ok' } },
          },
        },
      },
    };
    const paths = (swagger2ToOpenApi3(slack).spec as AnyJson).paths;
    const upload = paths['/files.upload'].post;
    expect(upload.parameters).toBeUndefined();
    expect(upload.requestBody).toEqual({
      required: true,
      content: {
        'multipart/form-data': {
          schema: {
            type: 'object',
            required: ['token'],
            properties: {
              token: { type: 'string', description: 'Auth token' },
              file: { type: 'string', format: 'binary' },
              channels: { type: 'string' },
            },
          },
        },
      },
      'x-ferrule-normalized': ['swagger2-form-data'],
    });
    const post = paths['/chat.postMessage'].post;
    expect(Object.keys(post.requestBody.content)).toEqual(['application/x-www-form-urlencoded']);
    expect(post.requestBody.required).toBeUndefined();
  });

  it('is the identity on an OpenAPI 3 document', () => {
    const spec = { openapi: '3.1.0', paths: {} };
    expect(swagger2ToOpenApi3(spec)).toEqual({ spec, diagnostics: [] });
  });

  it('runs inside convert before openapi.transforms', () => {
    const { spec } = convert(docker, {
      openapi: {
        transforms: [
          { command: 'remove', reason: 'test', args: { target: '$.components.schemas.Container' } },
        ],
      },
    } as unknown as Config);
    expect((spec as AnyJson).components.schemas.Container).toBeUndefined();
    expect((spec as AnyJson).openapi).toBe('3.0.3');
  });
});
