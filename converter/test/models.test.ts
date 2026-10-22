import { describe, expect, it } from 'vitest';
import { convert, parseConfig } from '../src/index.ts';

const spec = {
  openapi: '3.1.0',
  paths: { '/a': { get: { responses: {} } }, '/b': { get: { responses: {} } } },
  components: { schemas: { Ref: { type: 'object' }, Thing: { type: 'object' } } },
};

const ext = (out: Record<string, unknown>, path: string): Record<string, unknown> =>
  (out as { paths: Record<string, { get?: Record<string, unknown> }> }).paths[path]?.get ?? {};

describe('x-ferrule-models and x-ferrule-package (gaps G2, G3)', () => {
  it('mirrors models and package onto every annotated operation', () => {
    const config = parseConfig(`
targets: { python: { package_name: acme, production_repo: acme/acme-python } }
resources:
  $shared: { models: { references: Ref } }
  things:
    models: { thing: "#/components/schemas/Thing" }
    methods: { list: get /a }
    subresources:
      parts: { models: { part_ref: Ref }, methods: { retrieve: get /b } }
`);
    const { spec: out, diagnostics } = convert(spec, config);
    expect(diagnostics).toEqual([]);
    const models = {
      Ref: { name: 'PartRef', module: 'things.part_ref' },
      Thing: { name: 'Thing', module: 'thing' },
    };
    for (const path of ['/a', '/b']) {
      expect(ext(out, path)['x-ferrule-package']).toEqual({
        name: 'acme',
        repo: 'acme/acme-python',
      });
      expect(ext(out, path)['x-ferrule-models']).toEqual(models);
    }
  });

  it('puts $shared models in shared and omits the package without a python target', () => {
    const config = parseConfig(
      'resources: { $shared: { models: { references: Ref } }, a: { methods: { list: get /a } } }',
    );
    const { spec: out } = convert(spec, config);
    expect(ext(out, '/a')['x-ferrule-models']).toEqual({
      Ref: { name: 'References', module: 'shared.references' },
    });
    expect(ext(out, '/a')['x-ferrule-package']).toBeUndefined();
  });

  it('defaults repo to empty and warns on the object model form', () => {
    const config = parseConfig(`
targets: { python: { package_name: acme } }
resources: { a: { models: { thing: { type: { $ref: "#/components/schemas/Thing" } } }, methods: { list: get /a } } }
`);
    const { spec: out, diagnostics } = convert(spec, config);
    expect(ext(out, '/a')['x-ferrule-package']).toEqual({ name: 'acme', repo: '' });
    expect(diagnostics).toEqual([
      expect.objectContaining({
        code: 'FER1004',
        severity: 'warning',
        pointer: '/resources/a/models/thing',
      }),
    ]);
  });
});

describe('x-fern-type-name (gap G3)', () => {
  it('names every schema models: points at, with the last name winning', () => {
    const config = parseConfig(`
resources:
  $shared: { models: { references: Ref } }
  a: { models: { thing: Thing, other_ref: Ref }, methods: { list: get /a } }
`);
    const { spec: out, diagnostics } = convert(spec, config);
    expect(diagnostics).toEqual([]);
    const schemas = (out as { components: { schemas: Record<string, Record<string, unknown>> } })
      .components.schemas;
    expect(schemas).toMatchObject({
      Thing: { 'x-fern-type-name': 'Thing' },
      Ref: { 'x-fern-type-name': 'OtherRef' },
    });
    expect(spec.components.schemas.Ref).not.toHaveProperty('x-fern-type-name');
  });

  it('reports FER1005 for a model whose schema is not in the spec', () => {
    const config = parseConfig('resources: { a: { models: { ghost: Ghost } } }');
    const { spec: out, diagnostics } = convert(spec, config);
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1005', 'error', '/resources/a/models/ghost'],
    ]);
    expect(out).toEqual(spec);
  });
});
