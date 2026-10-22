import type { Json } from '@cloudflare/forge/plugin';
import { describe, expect, it } from 'vitest';
import { readMethodExtensions } from '../src/adapter/extensions.ts';
import { forgeToModel, groupToModel } from '../src/adapter/to-model.ts';
import { FerruleError, operationPointer } from '../src/diagnostics.ts';
import type { SdkObject } from '../src/model/index.ts';
import { fakeForge } from './helpers/fake-forge.ts';

const BASE: Record<string, Json> = {
  'x-ferrule-resource': { path: ['things'] },
  'x-ferrule-method-name': 'list',
  'x-ferrule-package': { name: 'acme', repo: 'acme/acme-python' },
};

function ext(over: Record<string, Json | undefined>): [string, Json][] {
  return Object.entries({ ...BASE, ...over }).filter(
    (e): e is [string, Json] => e[1] !== undefined,
  );
}

function code(f: () => unknown): string {
  try {
    f();
  } catch (e) {
    if (e instanceof FerruleError) return `${e.code} ${e.pointer}`;
    throw e;
  }
  return 'none';
}

describe('readMethodExtensions', () => {
  it('reads every extension', () => {
    const e = readMethodExtensions(
      ext({
        'x-other': 1,
        'x-ferrule-models': { Thing: { name: 'Thing', module: 'shared.thing' }, B: { name: 'B' } },
        'x-ferrule-pagination': {
          scheme: 'cursor',
          page: 'cursor_page',
          request: { after: 'next_cursor_param' },
          response: { data: 'items' },
        },
      }),
      '#/p',
    );
    expect(e.resource).toEqual(['things']);
    expect(e.methodName).toBe('list');
    expect(e.package).toEqual({ name: 'acme', repo: 'acme/acme-python' });
    expect(e.models.get('Thing')).toEqual({ name: 'Thing', module: 'shared.thing' });
    expect(e.models.get('B')).toEqual({ name: 'B', module: undefined });
    expect(e.pagination?.scheme).toBe('cursor');
  });

  it.each([
    ['x-ferrule-bogus', 1, 'FER102 #/p/x-ferrule-bogus'],
    ['x-ferrule-resource', undefined, 'FER103 #/p/x-ferrule-resource'],
    ['x-ferrule-resource', { path: [] }, 'FER101 #/p/x-ferrule-resource'],
    ['x-ferrule-resource', { path: ['a b'] }, 'FER101 #/p/x-ferrule-resource'],
    ['x-ferrule-method-name', 3, 'FER101 #/p/x-ferrule-method-name'],
    ['x-ferrule-package', { name: 'a' }, 'FER101 #/p/x-ferrule-package'],
    ['x-ferrule-package', 'acme', 'FER101 #/p/x-ferrule-package'],
    ['x-ferrule-package', { name: 'a', repo: 'a/b""" x' }, 'FER101 #/p/x-ferrule-package'],
    ['x-ferrule-models', [], 'FER101 #/p/x-ferrule-models'],
    ['x-ferrule-models', { A: { name: 'A', module: 'Bad-Mod' } }, 'FER101 #/p/x-ferrule-models'],
    ['x-ferrule-models', { A: { name: 1 } }, 'FER101 #/p/x-ferrule-models'],
    ['x-ferrule-models', { A: 'A' }, 'FER101 #/p/x-ferrule-models'],
    ['x-ferrule-pagination', { scheme: 'nope' }, 'FER101 #/p/x-ferrule-pagination'],
    [
      'x-ferrule-pagination',
      { scheme: 'cursor', page: 'p', request: { a: 1 }, response: {} },
      'FER101 #/p/x-ferrule-pagination',
    ],
    ['x-ferrule-streaming', { format: 'xml' }, 'FER101 #/p/x-ferrule-streaming'],
  ])('rejects %s = %j', (key, value, expected) => {
    expect(code(() => readMethodExtensions(ext({ [key]: value as Json }), '#/p'))).toBe(expected);
  });

  it('error messages carry code, pointer and fix', () => {
    expect(() => readMethodExtensions([], '#/p')).toThrow(
      /FER103 at #\/p\/x-ferrule-resource: .*Fix: /,
    );
  });
});

describe('operationPointer', () => {
  it('escapes the path', () => {
    expect(operationPointer('/a/{b}~c', 'GET')).toBe('#/paths/~1a~1{b}~0c/get');
  });
});

/** A one-operation document; `op` overrides the operation. */
function doc(op: Record<string, unknown>, schemas: Record<string, unknown> = {}): never {
  return {
    info: { version: '1', title: 'T' },
    servers: [{ url: 'https://x' }],
    paths: {
      '/things/{id}': {
        get: {
          'x-fern-sdk-group-name': ['things'],
          ...BASE,
          responses: {
            '200': {
              description: 'ok',
              content: { 'application/json': { schema: { type: 'string' } } },
            },
          },
          ...op,
        },
      },
    },
    components: { schemas },
  } as never;
}

function only(d: never) {
  const client = forgeToModel(fakeForge(d));
  const method = client?.resources[0]?.methods[0];
  if (client === undefined || method === undefined) throw new Error('no method');
  return { client, method };
}

describe('toModel', () => {
  it('maps scalars, enums, maps, unions and nullability', () => {
    const { method } = only(
      doc({
        responses: {
          '200': {
            description: 'ok',
            content: {
              'application/json': {
                schema: {
                  type: 'object',
                  properties: {
                    a: { type: 'string', format: 'date-time' },
                    b: { type: 'string', format: 'date' },
                    c: { type: 'string', format: 'uuid' },
                    d: { type: 'string', format: 'binary' },
                    e: { type: 'number', format: 'float' },
                    f: { type: 'integer', format: 'int32' },
                    g: { enum: ['x', 1] },
                    h: { type: 'object', additionalProperties: { type: 'boolean' } },
                    i: { type: 'object' },
                    j: { oneOf: [{ type: 'string' }, { type: 'integer' }] },
                    k: { type: 'string', nullable: true },
                    l: {},
                  },
                },
              },
            },
          },
        },
      }),
    );
    const fields = (method.response as SdkObject).fields.map((f) => [f.wireName, f.type]);
    expect(Object.fromEntries(fields)).toEqual({
      a: { kind: 'primitive', name: 'datetime' },
      b: { kind: 'primitive', name: 'date' },
      c: { kind: 'primitive', name: 'string' },
      d: { kind: 'unknown' },
      e: { kind: 'primitive', name: 'number' },
      f: { kind: 'primitive', name: 'integer' },
      g: { kind: 'literal', values: ['x', 1] },
      h: { kind: 'map', values: { kind: 'primitive', name: 'boolean' } },
      i: { kind: 'unknown' },
      j: {
        kind: 'union',
        variants: [
          { kind: 'primitive', name: 'string' },
          { kind: 'primitive', name: 'integer' },
        ],
      },
      k: {
        kind: 'union',
        variants: [
          { kind: 'primitive', name: 'string' },
          { kind: 'primitive', name: 'null' },
        ],
      },
      l: { kind: 'unknown' },
    });
  });

  it('splits path and query parameters and carries the body', () => {
    const { method, client } = only(
      doc({
        parameters: [
          { name: 'id', in: 'path', required: true, schema: { type: 'string' } },
          { name: 'q', in: 'query', schema: { type: 'string' } },
        ],
        requestBody: { content: { 'application/json': { schema: { type: 'string' } } } },
      }),
    );
    expect(method.pathParams.map((p) => p.pyName)).toEqual(['id']);
    expect(method.queryParams.map((p) => p.pyName)).toEqual(['q']);
    expect(method.body).toEqual({ kind: 'primitive', name: 'string' });
    expect(client.baseUrl).toBe('https://x');
    expect(method.pointer).toBe('#/paths/~1things~1{id}/get');
  });

  it('handles a body-only request and no JSON response', () => {
    const { method } = only(
      doc({
        requestBody: { content: { 'application/json': { schema: { type: 'string' } } } },
        responses: { '204': { description: 'none' } },
      }),
    );
    expect(method.queryParams).toEqual([]);
    expect(method.response).toBeUndefined();
  });

  it('names models, flattens allOf and keeps self-references finite', () => {
    const { client, method } = only(
      doc(
        {
          'x-ferrule-models': {
            Node: { name: 'Node' },
            Base: { name: 'Base', module: 'shared.base' },
          },
          responses: {
            '200': {
              description: 'ok',
              content: {
                'application/json': {
                  schema: {
                    allOf: [
                      { $ref: '#/components/schemas/Base' },
                      { $ref: '#/components/schemas/Mixin' },
                      {
                        type: 'object',
                        properties: { node: { $ref: '#/components/schemas/Node' } },
                      },
                    ],
                  },
                },
              },
            },
          },
        },
        {
          Base: { type: 'object', properties: { code: { type: 'integer' } } },
          Mixin: {
            allOf: [
              { $ref: '#/components/schemas/Base' },
              { type: 'object', properties: { m: { type: 'string' } } },
            ],
          },
          Node: { type: 'object', properties: { next: { $ref: '#/components/schemas/Node' } } },
        },
      ),
    );
    const resp = method.response as SdkObject;
    expect(resp.bases.map((b) => b.name)).toEqual(['Base', 'Base']);
    expect(resp.fields.map((f) => f.wireName)).toEqual(['m', 'node']);
    expect(client.models.map((m) => m.ref.module)).toEqual(['node', 'shared.base']);
    const node = client.models.find((m) => m.ref.name === 'Node');
    expect(node?.type.fields[0]?.type).toEqual({ kind: 'ref', name: 'Node', module: 'node' });
  });

  it.each([
    [
      'a recursive inline schema',
      doc(
        {
          responses: {
            '200': {
              description: 'ok',
              content: { 'application/json': { schema: { $ref: '#/components/schemas/R' } } },
            },
          },
        },
        { R: { type: 'object', properties: { r: { $ref: '#/components/schemas/R' } } } },
      ),
    ],
    [
      'an allOf of a scalar',
      doc({
        responses: {
          '200': {
            description: 'ok',
            content: { 'application/json': { schema: { allOf: [{ type: 'string' }] } } },
          },
        },
      }),
    ],
    [
      'a named model that is not an object',
      doc(
        {
          'x-ferrule-models': { S: { name: 'S' } },
          responses: {
            '200': {
              description: 'ok',
              content: { 'application/json': { schema: { $ref: '#/components/schemas/S' } } },
            },
          },
        },
        { S: { type: 'string' } },
      ),
    ],
    [
      'a pagination without an items field',
      doc({
        'x-ferrule-pagination': {
          scheme: 'cursor',
          page: 'cursor_page',
          request: {},
          response: { data: 'items' },
        },
      }),
    ],
  ])('rejects %s', (_, d) => {
    expect(code(() => forgeToModel(fakeForge(d)))).toMatch(/^FER(101|201) /);
  });

  it('rejects unsupported HTTP methods and disagreeing packages', () => {
    const forge = fakeForge(doc({}));
    const m = forge.groups[0]?.methods[0];
    if (m === undefined) throw new Error('no method');
    const other = { ...m, httpMethod: 'TRACE' };
    expect(code(() => groupToModel({ ...forge.groups[0], methods: [other] } as never))).toMatch(
      /FER201/,
    );
    const ext2 = m.extensions.map(([k, v]) =>
      k === 'x-ferrule-package'
        ? ([k, { name: 'other', repo: 'o/o' }] as const)
        : ([k, v] as const),
    );
    const g = { ...forge.groups[0], methods: [m, { ...m, extensions: ext2 }] };
    expect(code(() => groupToModel(g as never))).toMatch(/FER101/);
  });

  it('builds nested resources and returns undefined without operations', () => {
    const client = forgeToModel(fakeForge(doc({ 'x-ferrule-resource': { path: ['a', 'b'] } })));
    expect(client?.resources[0]?.path).toEqual(['a']);
    expect(client?.resources[0]?.subresources[0]?.path).toEqual(['a', 'b']);
    const forge = fakeForge(doc({}));
    expect(groupToModel({ ...forge.groups[0], methods: [] } as never)).toBeUndefined();
    expect(forgeToModel({ ...forge, groups: [] })).toBeUndefined();
  });
});
