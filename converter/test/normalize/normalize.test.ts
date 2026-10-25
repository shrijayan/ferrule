import { describe, expect, it } from 'vitest';
import { type Config, convert, normalize } from '../../src/index.ts';
import { stableStringify } from '../../src/serialize.ts';

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

  // Reduction of the fast-check counterexample `{"":{"__proto__":{}}}` from converter.test.ts.
  it('keeps a `__proto__` key as data', () => {
    const spec = JSON.parse('{"":{"__proto__":{"a":1}}}');
    const { spec: out } = normalize(spec);
    expect(Object.keys((out as AnyJson)[''])).toEqual(['__proto__']);
    expect(stableStringify(out)).toBe(stableStringify(spec));
    expect(JSON.parse(stableStringify(spec))).toEqual(spec);
  });
});
