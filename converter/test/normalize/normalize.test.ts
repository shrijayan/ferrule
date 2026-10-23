import { describe, expect, it } from 'vitest';
import { type Config, convert, normalize } from '../../src/index.ts';

// biome-ignore lint/suspicious/noExplicitAny: test access into untyped JSON
type AnyJson = any;

// Reductions of imagekit-config and knock-config specs that the pinned Forge CLI refused to read.
describe('normalize', () => {
  it('rewrites a null literal in a type array to "null" (imagekit)', () => {
    const spec = {
      components: {
        schemas: { F: { properties: { tags: { type: ['array', null], items: {} } } } },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const tags = (out as AnyJson).components.schemas.F.properties.tags;
    expect(tags.type).toEqual(['array', 'null']);
    expect(tags['x-ferrule-normalized']).toEqual(['type-null-literal']);
    expect(diagnostics).toEqual([
      expect.objectContaining({
        code: 'FER1601',
        severity: 'info',
        pointer: '/components/schemas/F/properties/tags/type',
      }),
    ]);
    expect(spec.components.schemas.F.properties.tags.type).toEqual(['array', null]);
  });

  it('rewrites a string discriminator to a Discriminator Object (knock)', () => {
    const spec = { components: { schemas: { R: { oneOf: [], discriminator: '__typename' } } } };
    const { spec: out, diagnostics } = normalize(spec);
    const r = (out as AnyJson).components.schemas.R;
    expect(r.discriminator).toEqual({ propertyName: '__typename' });
    expect(r['x-ferrule-normalized']).toEqual(['discriminator-string']);
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1602', '/components/schemas/R/discriminator'],
    ]);
  });

  it('records several rules on one node and leaves data and extensions alone', () => {
    const node = {
      type: [null, 'object'],
      discriminator: 'kind',
      example: { type: ['x', null], discriminator: 'y' },
      enum: [{ type: [null] }],
      'x-ext': { discriminator: 'z' },
      properties: { type: { type: 'string' }, discriminator: { type: 'string' } },
    };
    const { spec: out, diagnostics } = normalize({ components: { schemas: { N: node } } });
    const n = (out as AnyJson).components.schemas.N;
    expect(n['x-ferrule-normalized']).toEqual(['type-null-literal', 'discriminator-string']);
    expect(n.example).toBe(node.example);
    expect(n.enum).toBe(node.enum);
    expect(n['x-ext']).toBe(node['x-ext']);
    expect(n.properties).toEqual(node.properties);
    expect(diagnostics).toHaveLength(2);
  });

  it('is the identity on a spec with nothing to rewrite', () => {
    const spec = { openapi: '3.1.0', paths: { '/a': { get: { tags: ['x'] } } }, list: [1, 'a'] };
    expect(normalize(spec)).toEqual({ spec, diagnostics: [] });
  });

  it('runs inside convert before annotation', () => {
    const spec = { openapi: '3.1.0', components: { schemas: { R: { discriminator: 'k' } } } };
    const { spec: out, diagnostics } = convert(spec, {} as Config);
    expect((out as AnyJson).components.schemas.R.discriminator).toEqual({ propertyName: 'k' });
    expect(diagnostics.some((d) => d.code === 'FER1602')).toBe(true);
  });
});

// Reductions of the stripe, openai, github, twilio, cloudflare and square corpus specs.
describe('normalize: corpus Tier 1', () => {
  it('drops a query parameter style Forge does not cover (stripe, openai)', () => {
    const spec = {
      paths: {
        '/v1/webhook_endpoints/{id}': {
          get: {
            parameters: [
              { name: 'id', in: 'path', required: true, schema: { type: 'string' } },
              {
                name: 'expand',
                in: 'query',
                style: 'deepObject',
                explode: true,
                schema: { type: 'array' },
              },
              { name: 'tags', in: 'query', style: 'form', schema: { type: 'array' } },
            ],
          },
        },
      },
      components: {
        parameters: { metadata: { name: 'metadata', in: 'query', style: 'deepObject' } },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const params = (out as AnyJson).paths['/v1/webhook_endpoints/{id}'].get.parameters;
    expect(params[1]).toEqual({
      name: 'expand',
      in: 'query',
      explode: true,
      schema: { type: 'array' },
      'x-ferrule-normalized': ['query-style-deepObject'],
    });
    expect(params[2]).toEqual(spec.paths['/v1/webhook_endpoints/{id}'].get.parameters[2]);
    expect((out as AnyJson).components.parameters.metadata.style).toBeUndefined();
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1603', '/paths/~1v1~1webhook_endpoints~1{id}/get/parameters/1/style'],
      ['FER1603', '/components/parameters/metadata/style'],
    ]);
  });

  it('spells out symbols in properties whose names Forge finds equal (github)', () => {
    const spec = {
      components: {
        schemas: {
          reactions: {
            properties: { '+1': { type: 'integer' }, '-1': { type: 'integer' }, laugh: {} },
          },
        },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const props = (out as AnyJson).components.schemas.reactions.properties;
    expect(props['+1']).toEqual({
      type: 'integer',
      'x-fern-property-name': 'plus_1',
      'x-ferrule-normalized': ['equal-names'],
    });
    expect(props['-1']['x-fern-property-name']).toBe('minus_1');
    expect(props.laugh).toEqual({});
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1604', 'info', '/components/schemas/reactions/properties/+1'],
      ['FER1604', 'info', '/components/schemas/reactions/properties/-1'],
    ]);
  });

  it('spells out comparison suffixes on parameters Forge finds equal (twilio)', () => {
    const spec = {
      paths: {
        '/Recordings.json': {
          parameters: [{ name: 'DateCreated', in: 'query', schema: { type: 'string' } }],
          get: {
            parameters: [
              { $ref: '#/components/parameters/lt' },
              { name: 'DateCreated>', in: 'query', schema: { type: 'string' } },
              { name: 'date-created', in: 'header', schema: { type: 'string' } },
            ],
          },
        },
      },
      components: {
        parameters: { lt: { name: 'DateCreated<', in: 'query', schema: { type: 'string' } } },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const item = (out as AnyJson).paths['/Recordings.json'];
    expect(item.parameters[0]['x-fern-parameter-name']).toBeUndefined();
    expect(item.get.parameters[0]).toEqual({
      name: 'DateCreated<',
      in: 'query',
      schema: { type: 'string' },
      'x-fern-parameter-name': 'date_created_lt',
      'x-ferrule-normalized': ['equal-names'],
    });
    expect(item.get.parameters[1]['x-fern-parameter-name']).toBe('date_created_gt');
    expect(item.get.parameters[2]['x-fern-parameter-name']).toBeUndefined();
    expect((out as AnyJson).components.parameters.lt['x-fern-parameter-name']).toBeUndefined();
    expect(diagnostics.map((d) => d.pointer)).toEqual([
      '/paths/~1Recordings.json/get/parameters/0',
      '/paths/~1Recordings.json/get/parameters/1',
    ]);
  });

  it('numbers a field that differs from another only by case, with a warning (cloudflare)', () => {
    const spec = {
      components: {
        schemas: {
          hook: {
            properties: {
              notificationUrl: { type: 'string' },
              notification_url: { type: 'string' },
            },
          },
        },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const props = (out as AnyJson).components.schemas.hook.properties;
    expect(props.notification_url['x-fern-property-name']).toBeUndefined();
    expect(props.notificationUrl['x-fern-property-name']).toBe('notification_url_2');
    expect(diagnostics.map((d) => [d.code, d.severity])).toEqual([['FER1605', 'warning']]);
  });

  it('replaces a $ref to a missing schema with an unknown schema, with a warning (square)', () => {
    const spec = {
      components: {
        schemas: {
          Refund: {
            properties: {
              app_fee: { type: 'array', items: { $ref: '#/components/schemas/AppFeeAllocation' } },
              money: { $ref: '#/components/schemas/Money' },
            },
          },
          Money: { type: 'object' },
        },
      },
    };
    const { spec: out, diagnostics } = normalize(spec);
    const props = (out as AnyJson).components.schemas.Refund.properties;
    expect(props.app_fee.items).toEqual({ 'x-ferrule-normalized': ['dangling-ref'] });
    expect(props.money).toEqual({ $ref: '#/components/schemas/Money' });
    expect(diagnostics).toEqual([
      expect.objectContaining({
        code: 'FER1606',
        severity: 'warning',
        pointer: '/components/schemas/Refund/properties/app_fee/items/$ref',
      }),
    ]);
  });

  it('rewrites a list-valued items to prefixItems, which Forge reads (slack)', () => {
    const latest = { items: [{ $ref: '#/components/schemas/M' }, { type: 'null' }] };
    const spec = { components: { schemas: { M: {}, C: { properties: { latest } } } } };
    const { spec: out, diagnostics } = normalize(spec);
    expect((out as AnyJson).components.schemas.C.properties.latest).toEqual({
      prefixItems: latest.items,
      'x-ferrule-normalized': ['items-tuple'],
    });
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1607', '/components/schemas/C/properties/latest/items'],
    ]);
  });

  it('treats property names that are keywords as names, not data (kubernetes)', () => {
    const spec = {
      components: {
        schemas: {
          L: {
            properties: {
              default: { additionalProperties: { $ref: '#/components/schemas/Gone' } },
            },
          },
        },
      },
    };
    const { diagnostics } = normalize(spec);
    expect(diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1606', '/components/schemas/L/properties/default/additionalProperties/$ref'],
    ]);
  });
});
