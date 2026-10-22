import { describe, expect, it } from 'vitest';
import {
  CLIENT_GROUP,
  collidesWithClientGroup,
  forgeGroupPath,
  forgeWords,
} from '../../src/extensions/groups.ts';
import {
  deref,
  isArraySchema,
  itemsSchema,
  propertyNames,
  propertySchema,
  successResponse,
} from '../../src/extensions/json.ts';

const spec = {
  components: {
    schemas: {
      Loop: { $ref: '#/components/schemas/Loop' },
      List: { allOf: [{ description: 'x' }, { items: { type: 'string' } }] },
      Deep: { allOf: [{ allOf: [{ properties: { a: { type: 'string' } } }] }] },
      'a/b~c': { type: 'integer' },
    },
  },
};

describe('json readers', () => {
  it('derefs escaped, cyclic and external refs', () => {
    expect(deref(spec, { $ref: '#/components/schemas/a~1b~0c' })).toEqual({ type: 'integer' });
    expect(deref(spec, { $ref: '#/components/schemas/Loop' })).toBeUndefined();
    expect(deref(spec, { $ref: 'other.yml#/x' })).toBeUndefined();
    expect(deref(spec, 3)).toBe(3);
  });

  it('follows allOf for properties and items', () => {
    const deep = { $ref: '#/components/schemas/Deep' };
    expect([...propertyNames(spec, deep)]).toEqual(['a']);
    expect(propertySchema(spec, deep, 'a')).toEqual({ type: 'string' });
    expect(propertySchema(spec, deep, 'b')).toBeUndefined();
    const list = { $ref: '#/components/schemas/List' };
    expect(itemsSchema(spec, list)).toEqual({ type: 'string' });
    expect(isArraySchema(spec, list)).toBe(true);
    expect(itemsSchema(spec, { allOf: [{}] })).toBeUndefined();
    expect(isArraySchema(spec, { type: 'object' })).toBe(false);
    expect(isArraySchema(spec, 'x')).toBe(false);
    expect(itemsSchema(spec, null)).toBeUndefined();
  });

  it('stops at a depth limit on self-nesting allOf', () => {
    let s: Record<string, unknown> = { properties: { z: {} }, items: {} };
    for (let i = 0; i < 40; i++) s = { allOf: [s] };
    expect(propertyNames(spec, s).size).toBe(0);
    expect(propertySchema(spec, s, 'z')).toBeUndefined();
    expect(isArraySchema(spec, s)).toBe(false);
    expect(itemsSchema(spec, s)).toBeUndefined();
  });

  it('picks the lowest 2xx response', () => {
    const op = {
      responses: { '404': {}, '201': { description: 'b' }, '200': { description: 'a' } },
    };
    expect(successResponse(spec, op)).toEqual({ description: 'a' });
    expect(successResponse(spec, { responses: { '200': 'x' } })).toBeUndefined();
    expect(successResponse(spec, {})).toBeUndefined();
  });
});

describe('Forge groups', () => {
  it('splits names into words as Forge does', () => {
    expect(forgeWords('$ferrule_client')).toEqual(['ferrule', 'client']);
    expect(forgeWords('XMLDocument')).toEqual(['xml', 'document']);
    expect(forgeWords('ipv4Cidr')).toEqual(['ipv4', 'cidr']);
  });

  it('maps a leading $client to the reserved group only', () => {
    expect(forgeGroupPath(['$client'])).toEqual([CLIENT_GROUP]);
    expect(forgeGroupPath(['$client', 'x'])).toEqual([CLIENT_GROUP, 'x']);
    expect(forgeGroupPath(['a', '$client'])).toEqual(['a', '$client']);
  });

  it('flags resources that share the reserved group', () => {
    expect(collidesWithClientGroup('ferrule_client')).toBe(true);
    expect(collidesWithClientGroup('FerruleClient')).toBe(true);
    expect(collidesWithClientGroup('$client')).toBe(false);
    expect(collidesWithClientGroup('client')).toBe(false);
  });
});
