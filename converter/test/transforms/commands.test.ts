import { describe, expect, it } from 'vitest';
import { applyTransforms } from '../../src/transforms/openapi-transforms.ts';
import type { OpenApiSpec } from '../../src/transforms/pipeline.ts';

function base(): OpenApiSpec {
  return {
    openapi: '3.1.0',
    info: { title: 'Pets', version: '1' },
    paths: {
      '/pets/{id}': {
        get: {
          parameters: [
            { name: 'id', in: 'path', schema: { type: 'integer' } },
            { name: 'limit', in: 'query', deprecated: true, schema: { type: 'integer' } },
          ],
          examples: { a: 1 },
          responses: { '200': { description: 'ok' } },
        },
      },
    },
    components: {
      schemas: {
        Pet: {
          type: 'object',
          description: 'A pet.',
          required: ['name'],
          properties: { name: { type: 'string' }, role: { type: 'string', default: 'cat' } },
        },
        user_response: { type: 'object' },
        ErrorTemplate: { type: 'object', properties: { message: { type: 'string' } } },
        NotFound: { type: 'object', properties: { message: { type: 'string' } } },
        Conflict: { type: 'object', properties: { message: { type: 'string' } } },
        Error: { type: 'object' },
      },
    },
  };
}

const run = (command: string, args: unknown, spec: OpenApiSpec = base()) =>
  applyTransforms(spec, [{ command, reason: 'test', args }]);

// biome-ignore lint/suspicious/noExplicitAny: tests poke into the result freely.
const at = (spec: OpenApiSpec): any => spec;

describe('update', () => {
  it('replaces values selected by a filter', () => {
    const r = run('update', {
      target: '$.paths..parameters[?(@.name == "id")].schema.type',
      value: 'string',
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).paths['/pets/{id}'].get.parameters[0].schema.type).toBe('string');
    expect(at(r.spec).paths['/pets/{id}'].get.parameters[1].schema.type).toBe('integer');
  });

  it('templates strings with {{value}}', () => {
    const r = run('update', {
      target: '$.components.schemas.Pet.description',
      value: '[BETA] {{value}} {{value}}',
      template: true,
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Pet.description).toBe('[BETA] A pet. A pet.');
  });

  it('replaces deep-equal duplicates found with matches_schema, honouring ignore', () => {
    const r = run('update', {
      target: [
        {
          matches_schema: '$.components.schemas.ErrorTemplate',
          ignore: '$.components.schemas.ErrorTemplate',
        },
      ],
      value: { $ref: '#/components/schemas/Error' },
    });
    expect(r.diagnostics).toEqual([]);
    const s = at(r.spec).components.schemas;
    expect(s.NotFound).toEqual({ $ref: '#/components/schemas/Error' });
    expect(s.Conflict).toEqual({ $ref: '#/components/schemas/Error' });
    expect(s.ErrorTemplate.type).toBe('object');
  });

  it('accepts ignore as a list', () => {
    const r = run('update', {
      target: [
        {
          matches_schema: '$.components.schemas.ErrorTemplate',
          ignore: ['$.components.schemas.ErrorTemplate', '$.components.schemas.Conflict'],
        },
      ],
      value: { $ref: '#/components/schemas/Error' },
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Conflict.type).toBe('object');
    expect(at(r.spec).components.schemas.NotFound.$ref).toBeDefined();
  });

  it('can replace the whole document with an object', () => {
    const r = run('update', { target: '$', value: { openapi: '3.0.0' } });
    expect(r.spec).toEqual({ openapi: '3.0.0' });
  });

  it('reports a target that matches nothing and leaves the spec alone', () => {
    const r = run('update', { target: '$.components.schemas.Nope.type', value: 'x' });
    expect(r.spec).toEqual(base());
    expect(r.diagnostics).toEqual([
      expect.objectContaining({
        code: 'FER1201',
        severity: 'error',
        pointer: '/openapi/transforms/0/args/target',
        suggestion: expect.stringMatching(/delete this transform/),
      }),
    ]);
  });

  it('addresses each entry of a target list', () => {
    const r = run('update', {
      target: ['$.info.title', '$.info.nope'],
      value: 'x',
    });
    expect(r.diagnostics.map((d) => d.pointer)).toEqual(['/openapi/transforms/0/args/target/1']);
    expect(at(r.spec).info.title).toBe('Pets');
  });

  it('rejects bad templates, non-string template targets and a non-object root', () => {
    const codes = (args: unknown) => run('update', args).diagnostics.map((d) => d.code);
    expect(codes({ target: '$.info.title', value: 'no placeholder', template: true })).toEqual([
      'FER1210',
    ]);
    expect(codes({ target: '$.info.title', value: 3, template: true })).toEqual(['FER1210']);
    expect(codes({ target: '$.info', value: '{{value}}!', template: true })).toEqual(['FER1205']);
    expect(codes({ target: '$', value: 'x' })).toEqual(['FER1205']);
  });

  it('reports matches_schema templates that match zero or several nodes, and no duplicates', () => {
    const codes = (target: unknown) =>
      run('update', { target, value: 1 }).diagnostics.map((d) => [d.code, d.pointer]);
    expect(codes([{ matches_schema: '$.components.schemas.Nope' }])).toEqual([
      ['FER1207', '/openapi/transforms/0/args/target/0/matches_schema'],
    ]);
    expect(codes([{ matches_schema: '$.components.schemas.*' }])).toEqual([
      ['FER1207', '/openapi/transforms/0/args/target/0/matches_schema'],
    ]);
    expect(codes([{ matches_schema: '$.info', ignore: '$.info' }]).map(([c]) => c)).toEqual([
      'FER1201',
    ]);
    expect(codes([{ matches_schema: '$.info[' }]).map(([c]) => c)).toEqual(['FER1202']);
    expect(
      codes([{ matches_schema: '$.info', ignore: ['$.info', '$['] }]).map(([c, p]) => [c, p]),
    ).toEqual([
      ['FER1202', '/openapi/transforms/0/args/target/0/ignore/1'],
      ['FER1201', '/openapi/transforms/0/args/target/0'],
    ]);
  });

  it('reports invalid JSONPath with FER1202', () => {
    const r = run('update', { target: '$.info[', value: 1 });
    expect(r.diagnostics).toEqual([
      expect.objectContaining({ code: 'FER1202', pointer: '/openapi/transforms/0/args/target' }),
    ]);
  });
});

describe('append', () => {
  it('pushes an item, or several items, onto arrays', () => {
    const one = run('append', { target: '$.components.schemas.Pet.required', value: 'id' });
    expect(one.diagnostics).toEqual([]);
    expect(at(one.spec).components.schemas.Pet.required).toEqual(['name', 'id']);
    const many = run('append', {
      target: '$.components.schemas.Pet.required',
      value: ['id', 'role'],
    });
    expect(at(many.spec).components.schemas.Pet.required).toEqual(['name', 'id', 'role']);
  });

  it('adds new properties to objects', () => {
    const r = run('append', {
      target: '$.components.schemas.Pet.properties',
      value: { id: { type: 'string', format: 'uuid' } },
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Pet.properties.id).toEqual({
      type: 'string',
      format: 'uuid',
    });
  });

  it('fails when a property already exists, so a fixed spec is noticed', () => {
    const r = run('append', {
      target: '$.components.schemas.Pet.properties',
      value: { name: { type: 'integer' }, 'a/b': 1 },
    });
    expect(r.spec).toEqual(base());
    expect(r.diagnostics).toEqual([
      expect.objectContaining({
        code: 'FER1206',
        pointer: '/openapi/transforms/0/args/value/name',
      }),
    ]);
  });

  it('reports no match and wrong target or value types', () => {
    expect(run('append', { target: '$.nope', value: 1 }).diagnostics[0]?.code).toBe('FER1201');
    expect(run('append', { target: '$.info.title', value: 1 }).diagnostics[0]?.code).toBe(
      'FER1205',
    );
    const r = run('append', { target: '$.info', value: 'x' });
    expect(r.diagnostics).toEqual([
      expect.objectContaining({ code: 'FER1205', pointer: '/openapi/transforms/0/args/value' }),
    ]);
  });
});

describe('merge', () => {
  it('deep merges into every target, overwriting leaves', () => {
    const r = run('merge', {
      target: '$..parameters[?(@.name == "limit")]',
      value: { schema: { 'x-stainless-skip': ['terraform'], type: 'number' }, in: 'header' },
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).paths['/pets/{id}'].get.parameters[1]).toEqual({
      name: 'limit',
      in: 'header',
      deprecated: true,
      schema: { type: 'number', 'x-stainless-skip': ['terraform'] },
    });
  });

  it('reports no match and non-object targets', () => {
    expect(run('merge', { target: '$.nope', value: {} }).diagnostics[0]?.code).toBe('FER1201');
    expect(run('merge', { target: '$.info.title', value: {} }).diagnostics).toEqual([
      expect.objectContaining({ code: 'FER1205', message: expect.stringMatching(/found string/) }),
    ]);
    expect(
      run('merge', { target: '$.paths..parameters', value: {} }).diagnostics[0]?.message,
    ).toMatch(/found array/);
  });
});

describe('remove', () => {
  it('removes nodes from objects and arrays', () => {
    const r = run('remove', {
      target: ['$.paths.*.*.examples', '$..parameters[?(@.deprecated == true)]'],
    });
    expect(r.diagnostics).toEqual([]);
    const op = at(r.spec).paths['/pets/{id}'].get;
    expect(op).not.toHaveProperty('examples');
    expect(op.parameters.map((p: { name: string }) => p.name)).toEqual(['id']);
  });

  it('removes several elements of one array without shifting indices', () => {
    const r = run('remove', { target: '$..parameters[*]' });
    expect(at(r.spec).paths['/pets/{id}'].get.parameters).toEqual([]);
  });

  it('removes nested matches together with their ancestors', () => {
    const r = run('remove', { target: '$..type' });
    expect(r.diagnostics).toEqual([]);
    expect(JSON.stringify(r.spec)).not.toContain('"type"');
  });

  it('removes specific keys', () => {
    const r = run('remove', {
      target: '$.components.schemas.Pet.properties.role',
      keys: ['default'],
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Pet.properties.role).toEqual({ type: 'string' });
  });

  it('reports no match, missing keys, keyed non-objects and the root', () => {
    expect(run('remove', { target: '$.nope' }).diagnostics[0]?.code).toBe('FER1201');
    expect(
      run('remove', { target: '$.components.schemas.Pet', keys: ['type', 'nope'] }).diagnostics,
    ).toEqual([
      expect.objectContaining({ code: 'FER1209', pointer: '/openapi/transforms/0/args/keys/1' }),
    ]);
    expect(run('remove', { target: '$.info.title', keys: ['x'] }).diagnostics[0]?.code).toBe(
      'FER1205',
    );
    expect(run('remove', { target: '$' }).diagnostics[0]?.code).toBe('FER1211');
  });
});

describe('copy', () => {
  it('copies to a new key whose parent exists', () => {
    const r = run('copy', { from: '$.components.schemas.Pet', to: '$.components.schemas.Admin' });
    expect(r.diagnostics).toEqual([]);
    const s = at(r.spec).components.schemas;
    expect(s.Admin).toEqual(s.Pet);
    s.Admin.required.push('x');
    expect(s.Pet.required).toEqual(['name']);
  });

  it('copies to several destinations, replacing existing values', () => {
    const r = run('copy', {
      from: '$.components.schemas.NotFound',
      to: [
        "$.paths./pets/{id}.get.responses.200['content']",
        '$.components.schemas.Error',
        '$.paths.*.get.parameters[2]',
        '$.paths.*.get.parameters.0',
      ],
    });
    expect(r.diagnostics).toEqual([]);
    const s = at(r.spec);
    const nf = s.components.schemas.NotFound;
    expect(s.paths['/pets/{id}'].get.responses['200'].content).toEqual(nf);
    expect(s.components.schemas.Error).toEqual(nf);
    expect(s.paths['/pets/{id}'].get.parameters).toEqual([
      nf,
      at(base()).paths['/pets/{id}'].get.parameters[1],
      nf,
    ]);
  });

  it('copies onto existing nodes selected by wildcards or matches_schema', () => {
    const r = run('copy', {
      from: '$.info',
      to: [
        '$.components.schemas[?(@.type == "object" && @property == "Error")]',
        { matches_schema: '$.components.schemas.user_response' },
      ],
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Error).toEqual(at(base()).info);
    expect(at(r.spec).components.schemas.user_response).toEqual(at(base()).info);
  });

  it('can replace the root with an object', () => {
    expect(run('copy', { from: '$.info', to: '$' }).spec).toEqual(at(base()).info);
  });

  it('reports a missing source, an ambiguous source, a missing parent and a bad root copy', () => {
    const d = (args: unknown) => run('copy', args).diagnostics.map((x) => [x.code, x.pointer]);
    expect(d({ from: '$.nope', to: '$.info.x' })).toEqual([
      ['FER1201', '/openapi/transforms/0/args/from'],
    ]);
    expect(d({ from: '$.components.schemas.*', to: '$.info.x' })).toEqual([
      ['FER1207', '/openapi/transforms/0/args/from'],
    ]);
    expect(d({ from: '$.info', to: '$.nope.x' })).toEqual([
      ['FER1208', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info', to: '$.info.title.x' })).toEqual([
      ['FER1208', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info', to: '$.paths.*.get.parameters.x' })).toEqual([
      ['FER1208', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info', to: '$.paths.*.get.parameters[7]' })).toEqual([
      ['FER1208', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info.title', to: '$' })).toEqual([
      ['FER1205', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$[', to: ['$.info.x', '$['] })).toEqual([
      ['FER1202', '/openapi/transforms/0/args/from'],
      ['FER1202', '/openapi/transforms/0/args/to/1'],
    ]);
  });
});

describe('move', () => {
  it('renames a key in place, keeping its position', () => {
    const r = run('move', {
      from: '$.components.schemas.user_response',
      to: '$.components.schemas.UserResponse',
    });
    expect(r.diagnostics).toEqual([]);
    expect(Object.keys(at(r.spec).components.schemas)).toEqual([
      'Pet',
      'UserResponse',
      'ErrorTemplate',
      'NotFound',
      'Conflict',
      'Error',
    ]);
  });

  it('renames onto an existing key, replacing it', () => {
    const r = run('move', {
      from: '$.components.schemas.NotFound',
      to: '$.components.schemas.Pet',
    });
    expect(Object.keys(at(r.spec).components.schemas)).toEqual([
      'user_response',
      'ErrorTemplate',
      'Pet',
      'Conflict',
      'Error',
    ]);
    expect(at(r.spec).components.schemas.Pet).toEqual(at(base()).components.schemas.NotFound);
  });

  it('relocates between parents', () => {
    const r = run('move', {
      from: '$.components.schemas.Pet.properties.role',
      to: '$.components.schemas.Error.properties',
    });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).components.schemas.Pet.properties).not.toHaveProperty('role');
    expect(at(r.spec).components.schemas.Error.properties).toEqual({
      type: 'string',
      default: 'cat',
    });
  });

  it('reorders within an array and appends to arrays', () => {
    const r = run('move', { from: '$..parameters[0]', to: '$..parameters[1]' });
    expect(r.diagnostics).toEqual([]);
    expect(
      at(r.spec).paths['/pets/{id}'].get.parameters.map((p: { name: string }) => p.name),
    ).toEqual(['limit', 'id']);
    const s = run('move', { from: '$.info.title', to: '$.components.schemas.Pet.required[1]' });
    expect(s.diagnostics).toEqual([]);
    expect(at(s.spec).components.schemas.Pet.required).toEqual(['name', 'Pets']);
  });

  it('moves a child onto its own ancestor', () => {
    const r = run('move', { from: '$.info.title', to: '$.info' });
    expect(r.diagnostics).toEqual([]);
    expect(at(r.spec).info).toBe('Pets');
  });

  it('moves an object onto the root, and a node onto itself is a no-op', () => {
    expect(run('move', { from: '$.info', to: '$' }).spec).toEqual(at(base()).info);
    expect(run('move', { from: '$.info', to: '$.info' }).spec).toEqual(base());
  });

  it('reports bad sources and destinations', () => {
    const d = (args: unknown) => run('move', args).diagnostics.map((x) => [x.code, x.pointer]);
    expect(d({ from: '$.nope', to: '$.info.x' })).toEqual([
      ['FER1201', '/openapi/transforms/0/args/from'],
    ]);
    expect(d({ from: '$.info', to: '$.nope.x' })).toEqual([
      ['FER1208', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$', to: '$.info.x' })).toEqual([
      ['FER1211', '/openapi/transforms/0/args/from'],
    ]);
    expect(d({ from: '$.info', to: '$.info.inner' })).toEqual([
      ['FER1211', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info.title', to: '$' })).toEqual([
      ['FER1205', '/openapi/transforms/0/args/to'],
    ]);
    expect(d({ from: '$.info', to: '$.components.schemas.*' })).toEqual([
      ['FER1207', '/openapi/transforms/0/args/to'],
    ]);
  });
});
