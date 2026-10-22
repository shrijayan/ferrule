import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import {
  deepEqual,
  describeAlternatives,
  type JsonSchema,
  jsonType,
  resolveRef,
  unsupportedKeywords,
  validate,
} from '../../src/schema/json-schema.ts';
import { STAINLESS_CONFIG_SCHEMA } from '../../src/schema/stainless-schema.ts';

const keywords = (schema: JsonSchema, value: unknown) =>
  validate(schema, value).map((e) => [e.keyword, e.path.join('/')]);

describe('the bundled stainless.yml schema', () => {
  it('uses only keywords the validator implements', () => {
    expect(unsupportedKeywords(STAINLESS_CONFIG_SCHEMA)).toEqual([]);
  });

  it('has patterns that compile as unicode regexes', () => {
    const patterns: string[] = [];
    JSON.stringify(STAINLESS_CONFIG_SCHEMA, (key, value: unknown) => {
      if (key === 'pattern' && typeof value === 'string') patterns.push(value);
      return value;
    });
    expect(patterns.length).toBeGreaterThan(0);
    for (const p of patterns) expect(() => new RegExp(p, 'u')).not.toThrow();
  });
});

describe('unsupportedKeywords', () => {
  it('flags unknown keywords anywhere, ignoring x- extensions', () => {
    const schema: JsonSchema = {
      'x-anything': 1,
      if: {},
      properties: { a: { not: {} } },
      $defs: { d: { dependentSchemas: {} } },
      anyOf: [true, { else: {} }],
      items: { patternProperties: {} },
    };
    expect(unsupportedKeywords(schema)).toEqual([
      '/if',
      '/properties/a/not',
      '/$defs/d/dependentSchemas',
      '/anyOf/1/else',
      '/items/patternProperties',
    ]);
  });
});

describe('validate', () => {
  it('handles boolean schemas', () => {
    expect(validate(true, 1)).toEqual([]);
    expect(keywords(false, 1)).toEqual([['false', '']]);
  });

  it('checks types, with integer as a subtype of number', () => {
    expect(keywords({ type: 'number' }, 3)).toEqual([]);
    expect(keywords({ type: 'integer' }, 3.5)).toEqual([['type', '']]);
    expect(keywords({ type: ['string', 'null'] }, null)).toEqual([]);
    expect(keywords({ type: 'object' }, [])).toEqual([['type', '']]);
    expect(keywords({ type: 'string', minLength: 3 }, 1)).toEqual([['type', '']]);
  });

  it('checks const and enum by deep equality', () => {
    expect(keywords({ const: { a: [1] } }, { a: [1] })).toEqual([]);
    expect(keywords({ const: 'a' }, 'b')).toEqual([['const', '']]);
    expect(keywords({ enum: [1, 'x'] }, 'x')).toEqual([]);
    expect(keywords({ enum: [1, 'x'] }, 2)).toEqual([['enum', '']]);
  });

  it('checks string length in code points and patterns', () => {
    expect(keywords({ minLength: 2 }, '😀')).toEqual([['minLength', '']]);
    expect(keywords({ maxLength: 1 }, '😀')).toEqual([]);
    expect(keywords({ maxLength: 1 }, 'ab')).toEqual([['maxLength', '']]);
    expect(keywords({ pattern: '^a+$' }, 'aa')).toEqual([]);
    expect(keywords({ pattern: '^a+$' }, 'ab')).toEqual([['pattern', '']]);
  });

  it('checks numeric bounds', () => {
    const s = { minimum: 1, maximum: 5, exclusiveMinimum: 1, exclusiveMaximum: 5 };
    expect(keywords(s, 3)).toEqual([]);
    expect(keywords(s, 1)).toEqual([['exclusiveMinimum', '']]);
    expect(keywords(s, 5)).toEqual([['exclusiveMaximum', '']]);
    expect(keywords(s, 0)).toEqual([
      ['minimum', ''],
      ['exclusiveMinimum', ''],
    ]);
    expect(keywords(s, 9)).toEqual([
      ['maximum', ''],
      ['exclusiveMaximum', ''],
    ]);
  });

  it('checks objects: required, properties, additionalProperties, propertyNames', () => {
    const s: JsonSchema = {
      type: 'object',
      required: ['a'],
      properties: { a: { type: 'string' } },
      additionalProperties: { type: 'integer' },
      propertyNames: { minLength: 1 },
    };
    expect(keywords(s, { a: 'x', b: 1 })).toEqual([]);
    expect(keywords(s, { b: 'no', '': 1 })).toEqual([
      ['required', ''],
      ['type', 'b'],
      ['propertyNames', ''],
    ]);
    expect(
      keywords({ properties: { a: true }, additionalProperties: false }, { a: 1, z: 2 }),
    ).toEqual([['additionalProperties', 'z']]);
  });

  it('does not treat inherited names as declared properties', () => {
    expect(keywords({ properties: {}, additionalProperties: false }, { toString: 1 })).toEqual([
      ['additionalProperties', 'toString'],
    ]);
  });

  it('checks array items', () => {
    expect(keywords({ items: { type: 'string' } }, ['a', 2])).toEqual([['type', '1']]);
  });

  it('applies allOf and $ref', () => {
    const root: JsonSchema = {
      $defs: { S: { type: 'string' } },
      allOf: [{ $ref: '#/$defs/S' }, { minLength: 2 }],
    };
    expect(keywords(root, 'ab')).toEqual([]);
    expect(keywords(root, 'a')).toEqual([['minLength', '']]);
    expect(keywords(root, 1)).toEqual([['type', '']]);
  });

  it('passes anyOf when any branch passes and keeps that branch’s warnings', () => {
    const s: JsonSchema = {
      anyOf: [{ type: 'string' }, { properties: { old: { deprecated: true } } }],
    };
    expect(validate(s, 'x')).toEqual([]);
    expect(validate(s, { old: 1 }).map((e) => [e.keyword, e.severity])).toEqual([
      ['deprecated', 'warning'],
    ]);
  });

  it('reports the only type-compatible anyOf branch', () => {
    const s: JsonSchema = {
      anyOf: [{ type: 'string' }, { type: 'object', required: ['endpoint'] }],
    };
    expect(keywords(s, {})).toEqual([['required', '']]);
  });

  it('uses a discriminator to pick between object branches', () => {
    const branch = (cmd: string, field: string): JsonSchema => ({
      type: 'object',
      required: ['command', field],
      properties: { command: { const: cmd }, [field]: { type: 'string' } },
    });
    const s: JsonSchema = { oneOf: [branch('update', 'a'), branch('remove', 'b')] };
    expect(keywords(s, { command: 'remove', b: 1 })).toEqual([['type', 'b']]);
    expect(keywords(s, { command: 'move' })).toEqual([['oneOf', '']]);
  });

  it('prefers the branch whose errors go deepest, then the one with fewest errors', () => {
    const s: JsonSchema = {
      anyOf: [
        { type: 'object', required: ['x', 'y'] },
        { type: 'object', properties: { a: { properties: { b: { type: 'string' } } } } },
      ],
    };
    expect(keywords(s, { a: { b: 1 } })).toEqual([['type', 'a/b']]);
    const t: JsonSchema = {
      anyOf: [
        { type: 'object', required: ['x', 'y'] },
        { type: 'object', required: ['z'] },
      ],
    };
    expect(keywords(t, {})).toEqual([['required', '']]);
  });

  it('reports anyOf of constants as an enum with the full list', () => {
    const s: JsonSchema = {
      $defs: { C: { const: 'c' } },
      anyOf: [{ const: 'a' }, { enum: ['b'] }, { $ref: '#/$defs/C' }, { anyOf: [{ const: 'd' }] }],
    };
    const [e] = validate(s, 'z');
    expect(e?.keyword).toBe('enum');
    expect(e?.params.allowed).toEqual(['a', 'b', 'c', 'd']);
  });

  it('summarises unmatched alternatives', () => {
    const s: JsonSchema = {
      $defs: { O: { type: 'object' } },
      anyOf: [
        { type: 'string' },
        { anyOf: [{ $ref: '#/$defs/O' }, { type: ['array', 'integer'] }] },
      ],
    };
    const [e] = validate(s, true);
    expect(e?.message).toBe('expected a string or an object or an array or an integer, got true');
    expect(validate({ anyOf: [false, { type: 'string' }] }, 2)[0]?.keyword).toBe('anyOf');
  });

  it('rejects oneOf when several branches pass', () => {
    expect(keywords({ oneOf: [{ type: 'string' }, { minLength: 1 }] }, 'x')).toEqual([
      ['oneOf', ''],
    ]);
    expect(keywords({ oneOf: [{ type: 'string' }, { type: 'number' }] }, 'x')).toEqual([]);
  });

  it('reports deprecated properties as warnings with rename hints', () => {
    const s: JsonSchema = {
      properties: { old: { deprecated: true, 'x-deprecated-renamed-to': 'new' } },
    };
    expect(validate(s, { old: 1 })[0]).toMatchObject({
      keyword: 'deprecated',
      severity: 'warning',
      params: { renamedTo: 'new', removed: false },
    });
  });

  it('ignores deprecation warnings raised by propertyNames', () => {
    const s: JsonSchema = { propertyNames: { properties: {} }, properties: {} };
    expect(validate(s, { a: 1 })).toEqual([]);
  });
});

describe('resolveRef', () => {
  it('resolves escaped pointers and rejects remote or missing refs', () => {
    const root: JsonSchema = { $defs: { 'a/b': { const: 1 }, 'c~d': true } };
    expect(resolveRef(root, '#/$defs/a~1b')).toEqual({ const: 1 });
    expect(resolveRef(root, '#/$defs/c~0d')).toBe(true);
    expect(() => resolveRef(root, 'other.json#/x')).toThrow(/only local refs/);
    expect(() => resolveRef(root, '#/$defs/missing')).toThrow(/unresolvable/);
    expect(() => resolveRef(root, '#/$defs/a~1b/const')).toThrow(/unresolvable/);
  });
});

describe('describeAlternatives', () => {
  it('describes booleans and stops at the ref budget', () => {
    expect(describeAlternatives(true, [true, false])).toEqual(['any value']);
    const root: JsonSchema = { $defs: { L: { $ref: '#/$defs/L' } } };
    expect(describeAlternatives(root, [{ $ref: '#/$defs/L' }])).toEqual([]);
  });
});

describe('properties', () => {
  const json = fc.jsonValue();

  it('a value always matches its own const and enum', () => {
    fc.assert(
      fc.property(json, (v) => {
        expect(validate({ const: v }, v)).toEqual([]);
        expect(validate({ enum: [v] }, v)).toEqual([]);
      }),
    );
  });

  it('a value always matches a schema of its own type', () => {
    fc.assert(
      fc.property(json, (v) => {
        expect(validate({ type: jsonType(v) }, v)).toEqual([]);
      }),
    );
  });

  it('deepEqual agrees with structural JSON equality', () => {
    fc.assert(
      fc.property(json, json, (a, b) => {
        expect(deepEqual(a, structuredClone(a))).toBe(true);
        if (deepEqual(a, b)) expect(deepEqual(b, a)).toBe(true);
      }),
    );
    expect(deepEqual([1], { 0: 1 })).toBe(false);
    expect(deepEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(deepEqual({ a: 1 }, { b: 1 })).toBe(false);
  });

  it('anyOf of true and anything accepts every value', () => {
    fc.assert(
      fc.property(json, (v) => {
        expect(validate({ anyOf: [{ type: 'null' }, true] }, v)).toEqual([]);
      }),
    );
  });
});
