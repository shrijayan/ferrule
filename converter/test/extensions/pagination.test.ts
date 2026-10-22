import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import {
  bestMatch,
  matches,
  operationShape,
  PAGINATION_SCHEMES,
  readSchemes,
} from '../../src/extensions/pagination.ts';

const purpose = (p: string) => ({ 'x-stainless-pagination-property': { purpose: p } });

const spec = {
  components: {
    schemas: {
      Item: { type: 'object', properties: { id: { type: 'string' } } },
      Page: {
        allOf: [
          {
            type: 'object',
            properties: {
              data: { type: 'array', items: { $ref: '#/components/schemas/Item' } },
            },
          },
          {
            properties: {
              meta: { type: 'object', properties: { next_cursor: { type: 'string' } } },
              has_more: { type: 'boolean' },
              total: { type: 'integer' },
            },
          },
        ],
      },
    },
  },
};

const op = (extra: Record<string, unknown> = {}) => ({
  parameters: [
    { name: 'cursor', in: 'query' },
    { name: 'limit', in: 'query' },
  ],
  responses: {
    '201': {
      content: { 'application/json': { schema: { $ref: '#/components/schemas/Page' } } },
    },
  },
  ...extra,
});

const nestedCursor = {
  name: 'nested',
  type: 'cursor',
  request: { cursor: purpose('next_cursor_param'), limit: { type: 'integer' } },
  response: {
    data: { type: 'array', ...purpose('items') },
    meta: {
      type: 'object',
      properties: { next_cursor: { type: 'string', ...purpose('next_cursor_field') } },
    },
  },
};

describe('readSchemes', () => {
  it('flattens nested response fields, item fields and header fields', () => {
    const { schemes, diagnostics } = readSchemes({
      pagination: [
        nestedCursor,
        {
          name: 'ids',
          type: 'cursor_id',
          param_location: 'body',
          continue_on_empty_items: true,
          request: { starting_after: purpose('cursor_id_param') },
          response: {
            data: {
              type: 'array',
              ...purpose('items'),
              items: { type: 'object', properties: { id: purpose('cursor_item_id') } },
            },
            meta: { type: 'object', properties: { plain: { type: 'string' } } },
            next: {
              'x-stainless-pagination-property': {
                purpose: 'next_cursor_url',
                from_header: 'Link',
              },
            },
            raw: { 'x-stainless-pagination-property': { from_header: 'X-Total' } },
          },
        },
      ],
    });
    expect(diagnostics).toEqual([]);
    expect(schemes.map((s) => s.extension)).toEqual([
      {
        scheme: 'cursor',
        page: 'nested',
        paramLocation: 'query',
        continueOnEmptyItems: false,
        request: { cursor: 'next_cursor_param', limit: 'limit' },
        response: { data: 'items', 'meta.next_cursor': 'next_cursor_field' },
      },
      {
        scheme: 'cursor_id',
        page: 'ids',
        paramLocation: 'body',
        continueOnEmptyItems: true,
        request: { starting_after: 'cursor_id_param' },
        response: { 'data.[].id': 'cursor_item_id', data: 'items', meta: 'meta' },
        headers: { Link: 'next_cursor_url', 'X-Total': 'raw' },
      },
    ]);
  });

  it('accepts the single-object form and rejects unknown types', () => {
    const single = readSchemes({ pagination: { type: 'offset', request: {}, response: {} } });
    expect(single.schemes[0]?.extension.page).toBe('page');
    const bad = readSchemes({ pagination: [{ name: 'a', type: 'token' }, 'junk'] });
    expect(bad.schemes).toEqual([]);
    expect(bad.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1501', '/pagination/0/type'],
      ['FER1501', '/pagination/1/type'],
    ]);
    expect(readSchemes({ pagination: { type: 1 } }).diagnostics[0]?.pointer).toBe(
      '/pagination/type',
    );
    expect(readSchemes({}).schemes).toEqual([]);
  });

  it('accepts every scheme the public schema allows', () => {
    fc.assert(
      fc.property(fc.constantFrom(...PAGINATION_SCHEMES), fc.string(), (type, name) => {
        const { schemes } = readSchemes({
          pagination: [{ name, type, request: {}, response: {} }],
        });
        expect(schemes[0]?.extension).toMatchObject({ scheme: type, page: name });
      }),
    );
  });
});

describe('matching', () => {
  const [nested] = readSchemes({ pagination: [nestedCursor] }).schemes;
  if (nested === undefined) throw new Error('fixture');

  it('matches nested response paths through $ref and allOf', () => {
    expect(matches(spec, nested, operationShape(spec, {}, op()))).toBe(true);
  });

  it('needs every request param and every response path', () => {
    const noLimit = op({ parameters: [{ name: 'cursor', in: 'query' }] });
    expect(matches(spec, nested, operationShape(spec, {}, noLimit))).toBe(false);
    const flat = op({
      responses: {
        '200': {
          content: {
            'application/json': {
              schema: { properties: { data: { type: 'array' }, next_cursor: {} } },
            },
          },
        },
      },
    });
    expect(matches(spec, nested, operationShape(spec, {}, flat))).toBe(false);
  });

  it('needs the items field to be an array', () => {
    const notArray = op({
      responses: {
        '200': {
          content: {
            'application/json': {
              schema: {
                properties: {
                  data: { type: 'object' },
                  meta: { properties: { next_cursor: {} } },
                },
              },
            },
          },
        },
      },
    });
    expect(matches(spec, nested, operationShape(spec, {}, notArray))).toBe(false);
    const typeList = op({
      responses: {
        '200': {
          content: {
            'application/json': {
              schema: {
                properties: {
                  data: { type: ['array', 'null'] },
                  meta: { properties: { next_cursor: {} } },
                },
              },
            },
          },
        },
      },
    });
    expect(matches(spec, nested, operationShape(spec, {}, typeList))).toBe(true);
  });

  it('reads body params for param_location: body', () => {
    const [body] = readSchemes({
      pagination: [
        {
          name: 'b',
          type: 'offset',
          param_location: 'body',
          request: { offset: {} },
          response: {},
        },
      ],
    }).schemes;
    if (body === undefined) throw new Error('fixture');
    const withBody = {
      requestBody: {
        content: { 'application/json': { schema: { properties: { offset: {} } } } },
      },
      responses: {},
    };
    expect(matches(spec, body, operationShape(spec, {}, withBody))).toBe(true);
    expect(
      matches(
        spec,
        body,
        operationShape(spec, {}, { parameters: [{ name: 'offset', in: 'query' }] }),
      ),
    ).toBe(false);
  });

  it('picks the most specific match, earliest on ties', () => {
    const { schemes } = readSchemes({
      pagination: [
        {
          name: 'loose',
          type: 'cursor',
          request: { cursor: {} },
          response: { data: purpose('items') },
        },
        nestedCursor,
        { ...nestedCursor, name: 'twin' },
        { name: 'miss', type: 'offset', request: { offset: {} }, response: {} },
      ],
    });
    const shape = operationShape(spec, { parameters: [{ name: 'limit', in: 'query' }] }, op());
    expect(bestMatch(spec, schemes, shape)?.extension.page).toBe('nested');
    expect(bestMatch(spec, schemes.slice(3), shape)).toBeUndefined();
  });

  it('collects path-level and operation-level query params only', () => {
    const shape = operationShape(
      spec,
      {
        parameters: [
          { name: 'a', in: 'query' },
          { name: 'h', in: 'header' },
        ],
      },
      { parameters: [{ $ref: '#/nope' }, { name: 'b', in: 'query' }], responses: { default: {} } },
    );
    expect([...shape.query]).toEqual(['a', 'b']);
    expect(shape.responseSchema).toBeUndefined();
    expect([...shape.body]).toEqual([]);
  });
});
