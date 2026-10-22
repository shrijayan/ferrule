import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { type Config, convert, convertText, parseConfig, parseSpec } from '../src/index.ts';

const cursorPage = {
  name: 'cursor_page',
  type: 'cursor',
  request: {
    after: { type: 'string', 'x-stainless-pagination-property': { purpose: 'next_cursor_param' } },
    limit: { type: 'integer' },
  },
  response: {
    data: { type: 'array', 'x-stainless-pagination-property': { purpose: 'items' } },
    next_cursor: {
      type: 'string',
      'x-stainless-pagination-property': { purpose: 'next_cursor_field' },
    },
  },
};

const expectedPagination = {
  scheme: 'cursor',
  page: 'cursor_page',
  paramLocation: 'query',
  continueOnEmptyItems: false,
  request: { after: 'next_cursor_param', limit: 'limit' },
  response: { data: 'items', next_cursor: 'next_cursor_field' },
};

function makeSpec(): Record<string, unknown> {
  return {
    openapi: '3.1.0',
    paths: {
      '/users': {
        parameters: [{ $ref: '#/components/parameters/Limit' }],
        get: {
          parameters: [
            { name: 'after', in: 'query', schema: { type: 'string' } },
            { name: 'X-Trace', in: 'header' },
          ],
          responses: { '200': { $ref: '#/components/responses/UserPage' } },
        },
        post: { responses: { '201': { description: 'ok' } } },
      },
      '/users/{id}': { get: { responses: {} } },
      '/users/{id}/posts': {
        get: {
          responses: {
            '200': {
              content: {
                'application/json': {
                  schema: {
                    allOf: [
                      { $ref: '#/components/schemas/Page' },
                      { properties: { extra: { type: 'string' } } },
                    ],
                  },
                },
              },
            },
          },
        },
      },
      '/aliased': { $ref: '#/paths/~1users~1{id}' },
      '/unconfigured': { get: { responses: {} } },
    },
    components: {
      parameters: { Limit: { name: 'limit', in: 'query', schema: { type: 'integer' } } },
      responses: {
        UserPage: {
          content: { 'application/json': { schema: { $ref: '#/components/schemas/Page' } } },
        },
      },
      schemas: {
        Page: {
          type: 'object',
          properties: { data: { type: 'array' }, next_cursor: { type: 'string' } },
        },
        Loop: { $ref: '#/components/schemas/Loop' },
      },
    },
  };
}

type Op = Record<string, unknown>;
type Paths = Record<string, Record<string, Op>>;
function op(spec: unknown, path: string, verb: string): Op {
  const { paths } = spec as { paths: Paths };
  return paths[path]?.[verb] as Op;
}

describe('convert: resources and methods', () => {
  const config: Config = {
    resources: {
      users: {
        methods: { retrieve: 'get /users/{id}', create: { endpoint: 'post /users' } },
        subresources: { posts: { methods: { list_posts: 'get /users/{id}/posts' } } },
      },
      $client: { methods: { ping: 'GET /users/{id}' } },
    },
  };

  it('writes Fern and x-ferrule names from the resource tree', () => {
    const { spec, diagnostics } = convert(makeSpec(), config);
    expect(diagnostics).toEqual([]);
    expect(op(spec, '/users', 'post')).toMatchObject({
      'x-fern-sdk-group-name': ['users'],
      'x-fern-sdk-method-name': 'create',
      'x-ferrule-resource': { path: ['users'], order: [0] },
      'x-ferrule-method-name': 'create',
      'x-ferrule-method-order': 1,
    });
    expect(op(spec, '/users/{id}/posts', 'get')).toMatchObject({
      'x-fern-sdk-group-name': ['users', 'posts'],
      'x-ferrule-resource': { path: ['users', 'posts'], order: [0, 0] },
      'x-ferrule-method-name': 'list_posts',
      'x-ferrule-method-order': 0,
    });
    // `$client` is kept verbatim; the verb is case-insensitive; later methods win on shared operations.
    // Forge needs a non-empty group, so `$client` methods go to a reserved one.
    expect(op(spec, '/users/{id}', 'get')).toMatchObject({
      'x-fern-sdk-group-name': ['$ferrule_client'],
      'x-ferrule-resource': { path: ['$client'], order: [1] },
      'x-ferrule-method-name': 'ping',
      'x-ferrule-method-order': 0,
    });
  });

  it('leaves operations not in the config untouched', () => {
    const input = makeSpec();
    const { spec } = convert(input, config);
    expect(op(spec, '/unconfigured', 'get')).toEqual(op(input, '/unconfigured', 'get'));
  });

  it('reports FER1001 with a pointer for missing operations', () => {
    const { diagnostics } = convert(makeSpec(), {
      resources: {
        a: {
          methods: { bad: 'get /nope', verb: 'put /users', junk: 'no-space' },
          subresources: { 'b/c': { methods: { x: { endpoint: 'get /missing' } } } },
        },
      },
    });
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1001', 'error', '/resources/a/methods/bad'],
      ['FER1001', 'error', '/resources/a/methods/verb'],
      ['FER1001', 'error', '/resources/a/methods/junk'],
      ['FER1001', 'error', '/resources/a/subresources/b~1c/methods/x'],
    ]);
    expect(diagnostics[0]?.message).toContain('get /nope');
    expect(diagnostics[0]?.suggestion).not.toBe('');
  });

  it('follows $ref path items', () => {
    const { spec } = convert(makeSpec(), { resources: { a: { methods: { r: 'get /aliased' } } } });
    expect(op(spec, '/users/{id}', 'get')).toHaveProperty('x-ferrule-method-name', 'r');
  });

  it('handles a spec without paths', () => {
    const { diagnostics } = convert({}, { resources: { a: { methods: { r: 'get /x' } } } });
    expect(diagnostics).toHaveLength(1);
  });
});

describe('convert: pagination', () => {
  const withPagination = (
    methods: NonNullable<Config['resources']>[string]['methods'],
  ): Config => ({
    pagination: [cursorPage],
    resources: { users: { methods } },
  });

  it('matches list methods against configured schemes', () => {
    const { spec, diagnostics } = convert(makeSpec(), withPagination({ list: 'get /users' }));
    expect(diagnostics).toEqual([]);
    expect(op(spec, '/users', 'get')['x-ferrule-pagination']).toEqual(expectedPagination);
  });

  it('matches list_* methods and allOf responses', () => {
    const { spec } = convert(makeSpec(), {
      pagination: [{ name: 'p', type: 'cursor', response: { data: null } }],
      resources: { users: { methods: { list_posts: 'get /users/{id}/posts' } } },
    });
    expect(op(spec, '/users/{id}/posts', 'get')['x-ferrule-pagination']).toEqual({
      scheme: 'cursor',
      page: 'p',
      paramLocation: 'query',
      continueOnEmptyItems: false,
      request: {},
      response: { data: 'data' },
    });
  });

  it('paginated: false suppresses matching', () => {
    const { spec } = convert(
      makeSpec(),
      withPagination({ list: { endpoint: 'get /users', paginated: false } }),
    );
    expect(op(spec, '/users', 'get')).not.toHaveProperty('x-ferrule-pagination');
  });

  it('does not match non-list methods or non-matching operations', () => {
    const { spec, diagnostics } = convert(
      makeSpec(),
      withPagination({ retrieve: 'get /users', list: 'get /users/{id}', list_x: 'post /users' }),
    );
    expect(diagnostics).toEqual([]);
    expect(op(spec, '/users/{id}', 'get')).not.toHaveProperty('x-ferrule-pagination');
    expect(op(spec, '/users', 'post')).not.toHaveProperty('x-ferrule-pagination');
  });

  it('paginated: true uses the matching scheme or reports FER1003', () => {
    const ok = convert(
      makeSpec(),
      withPagination({ fetch: { endpoint: 'get /users', paginated: true } }),
    );
    expect(op(ok.spec, '/users', 'get')['x-ferrule-pagination']).toEqual(expectedPagination);
    const bad = convert(
      makeSpec(),
      withPagination({ get: { endpoint: 'get /users/{id}', paginated: true } }),
    );
    expect(bad.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1003', '/resources/users/methods/get/paginated'],
    ]);
  });

  it('paginated: <name> picks a scheme or reports FER1002', () => {
    const ok = convert(
      makeSpec(),
      withPagination({ get: { endpoint: 'get /users/{id}', paginated: 'cursor_page' } }),
    );
    expect(op(ok.spec, '/users/{id}', 'get')['x-ferrule-pagination']).toEqual(expectedPagination);
    const bad = convert(makeSpec(), {
      resources: { u: { methods: { get: { endpoint: 'get /users/{id}', paginated: 'nope' } } } },
    });
    expect(bad.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1002', '/resources/u/methods/get/paginated'],
    ]);
    expect(bad.diagnostics[0]?.suggestion).toContain('(none defined)');
  });

  it('survives cyclic and external refs', () => {
    const spec = makeSpec();
    const { paths } = spec as { paths: Paths };
    Object.assign(paths['/users'] ?? {}, {
      get: {
        parameters: [{ $ref: 'other.yaml#/p' }],
        responses: {
          '200': {
            content: { 'application/json': { schema: { $ref: '#/components/schemas/Loop' } } },
          },
        },
      },
    });
    const { spec: out } = convert(spec, withPagination({ list: 'get /users' }));
    expect(op(out, '/users', 'get')).not.toHaveProperty('x-ferrule-pagination');
  });
});

describe('convert: x-ferrule-config', () => {
  it('is written from targets.python', () => {
    const { spec } = convert({}, { targets: { python: { package_name: 'acme' } } });
    expect(spec['x-ferrule-config']).toEqual({
      version: 1,
      edition: '2025-10-10',
      packageName: 'acme',
      projectName: 'acme',
    });
    const named = convert(
      {},
      { edition: 'e', targets: { python: { package_name: 'acme', project_name: 'acme-sdk' } } },
    );
    expect(named.spec['x-ferrule-config']).toMatchObject({ edition: 'e', projectName: 'acme-sdk' });
  });

  it('is omitted without a python package name', () => {
    expect(convert({}, { organization: { name: 'acme' } }).spec).not.toHaveProperty(
      'x-ferrule-config',
    );
  });
});

describe('parseConfig', () => {
  it('accepts the method forms, models, $shared and unknown keys', () => {
    const config = parseConfig(`
resources:
  a:
    methods:
      list: get /a
      create: { endpoint: post /a, paginated: false, type: http }
    models:
      thing: Thing
    subresources:
      b:
        methods:
          list: { endpoint: get /a/b, paginated: page }
  $shared:
    models: { ref: Reference }
pagination:
  - name: page
    type: offset
    request: { offset: { type: integer } }
    response: { items: { type: array } }
client_settings: { anything: true }
`);
    expect(config).toMatchObject({
      resources: {
        a: {
          subresources: { b: { methods: { list: { endpoint: 'get /a/b', paginated: 'page' } } } },
        },
      },
    });
    expect(config).toHaveProperty('client_settings');
  });

  it('rejects malformed methods and pagination', () => {
    expect(() => parseConfig('resources: { a: { methods: { x: 3 } } }')).toThrow();
    expect(() => parseConfig('pagination: [{ name: p }]')).toThrow();
  });
});

describe('parseSpec and convertText', () => {
  it('parses JSON and YAML objects and rejects others', () => {
    expect(parseSpec('{"openapi":"3.1.0"}')).toEqual({ openapi: '3.1.0' });
    expect(parseSpec('openapi: 3.1.0')).toEqual({ openapi: '3.1.0' });
    expect(() => parseSpec('- 1')).toThrow(TypeError);
    expect(() => parseSpec('')).toThrow(TypeError);
  });

  it('emits sorted, indented JSON with a trailing newline', () => {
    const { output } = convertText('{"b":1,"a":{"d":[{"z":1,"y":2}],"c":null}}', '');
    expect(output).toBe(
      '{\n  "a": {\n    "c": null,\n    "d": [\n      {\n        "y": 2,\n        "z": 1\n      }\n    ]\n  },\n  "b": 1\n}\n',
    );
  });
});

describe('properties', () => {
  const configArb = fc.record({
    resources: fc.dictionary(
      fc.string(),
      fc.record({
        methods: fc.dictionary(
          fc.string(),
          fc.oneof(
            fc.constantFrom('get /users', 'post /users', 'get /users/{id}', 'get /nope'),
            fc.record({
              endpoint: fc.constantFrom('get /users', 'get /users/{id}/posts'),
              paginated: fc.oneof(fc.boolean(), fc.constant('cursor_page')),
            }),
          ),
        ),
      }),
    ),
    pagination: fc.constant([cursorPage] as Config['pagination']),
  });

  it('never mutates its inputs', () => {
    fc.assert(
      fc.property(configArb, (config) => {
        const spec = makeSpec();
        const specBefore = structuredClone(spec);
        const configBefore = structuredClone(config);
        convert(spec, config);
        expect(spec).toEqual(specBefore);
        expect(config).toEqual(configBefore);
      }),
    );
  });

  it('is deterministic', () => {
    fc.assert(
      fc.property(configArb, (config) => {
        const text = JSON.stringify(makeSpec());
        const yaml = JSON.stringify(config);
        expect(convertText(text, yaml)).toEqual(convertText(text, yaml));
      }),
    );
  });
});

describe('convert: client, order, streaming and $client', () => {
  it('records stainless.yml order of resources and methods (gap G11)', () => {
    const { spec } = convert(makeSpec(), {
      resources: {
        z: { methods: { second: 'get /users/{id}/posts', first: 'post /users' } },
        a: { subresources: { x: {}, y: { methods: { only: 'get /users' } } } },
      },
    });
    expect(op(spec, '/users', 'post')).toMatchObject({
      'x-ferrule-resource': { path: ['z'], order: [0] },
      'x-ferrule-method-order': 1,
    });
    expect(op(spec, '/users/{id}/posts', 'get')).toMatchObject({ 'x-ferrule-method-order': 0 });
    expect(op(spec, '/users', 'get')).toMatchObject({
      'x-fern-sdk-group-name': ['a', 'y'],
      'x-ferrule-resource': { path: ['a', 'y'], order: [1, 1] },
      'x-ferrule-method-order': 0,
    });
  });

  it('writes x-ferrule-client at the root and on every operation for a python target', () => {
    const { spec, diagnostics } = convert(makeSpec(), {
      organization: { name: 'acme' },
      targets: { python: { package_name: 'acme' } },
      client_settings: { idempotency: { header: 'Idempotency-Key' } },
      resources: { users: { methods: { create: 'post /users', retrieve: 'get /users/{id}' } } },
    });
    expect(diagnostics).toEqual([]);
    const root = spec['x-ferrule-client'];
    expect(root).toMatchObject({
      name: 'acme',
      envPrefix: 'ACME',
      idempotencyHeader: 'Idempotency-Key',
    });
    expect(op(spec, '/users', 'post')['x-ferrule-client']).toEqual(root);
    expect(op(spec, '/users/{id}', 'get')['x-ferrule-client']).toEqual(root);
    const none = convert(makeSpec(), {
      resources: { users: { methods: { create: 'post /users' } } },
    });
    expect(none.spec).not.toHaveProperty('x-ferrule-client');
    expect(op(none.spec, '/users', 'post')).not.toHaveProperty('x-ferrule-client');
  });

  it('passes client diagnostics through', () => {
    const { diagnostics } = convert(makeSpec(), {
      targets: { python: { package_name: 'acme' } },
      client_settings: { default_timeout: 'never' },
    });
    expect(diagnostics.map((d) => d.code)).toEqual(['FER1305']);
  });

  it('writes x-ferrule-streaming with the top-level on_event handlers', () => {
    const { spec, diagnostics } = convert(makeSpec(), {
      streaming: { on_event: [{ data_starts_with: '[DONE]', handle: 'done' }] },
      resources: {
        users: {
          methods: {
            create: { endpoint: 'post /users', streaming: { param_discriminator: null } },
            retrieve: { endpoint: 'get /users/{id}', streaming: { type: 'jsonl' } },
          },
        },
      },
    });
    expect(op(spec, '/users', 'post')['x-ferrule-streaming']).toEqual({
      format: 'sse',
      paramDiscriminator: null,
      synthesizeEventAndData: false,
      onEvent: [{ kind: 'data', handle: 'done', dataStartsWith: '[DONE]' }],
    });
    expect(op(spec, '/users/{id}', 'get')['x-ferrule-streaming']).toMatchObject({
      format: 'jsonl',
    });
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1401', '/resources/users/methods/retrieve/streaming'],
    ]);
  });

  it('reports FER1006 for a resource sharing the $client Forge group', () => {
    const { diagnostics } = convert(makeSpec(), {
      resources: { ferrule_client: {}, $client: { methods: { ping: 'get /users' } } },
    });
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1006', 'error', '/resources/ferrule_client'],
    ]);
  });

  it('reports pagination config errors and names the defined schemes', () => {
    const { diagnostics } = convert(makeSpec(), {
      pagination: [cursorPage, { name: 'bad', type: 'token', request: {}, response: {} }],
      resources: { u: { methods: { get: { endpoint: 'get /users/{id}', paginated: 'nope' } } } },
    } as unknown as Config);
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1501', '/pagination/1/type'],
      ['FER1002', '/resources/u/methods/get/paginated'],
    ]);
    expect(diagnostics[1]?.suggestion).toContain('cursor_page');
  });
});
