import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { stableStringify } from '../../src/serialize.ts';
import {
  applyConfigTransforms,
  applyTransforms,
  TRANSFORM_COMMANDS,
} from '../../src/transforms/openapi-transforms.ts';
import type { OpenApiSpec } from '../../src/transforms/pipeline.ts';

const spec: OpenApiSpec = {
  info: { title: 'T' },
  components: { schemas: { A: { type: 'object' }, B: { type: 'string' } } },
};

describe('applyTransforms', () => {
  it('supports exactly the commands the schema defines', () => {
    expect(TRANSFORM_COMMANDS).toEqual(['update', 'append', 'merge', 'remove', 'copy', 'move']);
  });

  it('applies transforms in order, each seeing the previous result', () => {
    const r = applyTransforms(spec, [
      {
        command: 'copy',
        reason: 'r',
        args: { from: '$.components.schemas.A', to: '$.components.schemas.C' },
      },
      {
        command: 'merge',
        reason: 'r',
        args: { target: '$.components.schemas.C', value: { title: 'C' } },
      },
      {
        command: 'move',
        reason: 'r',
        args: { from: '$.components.schemas.C', to: '$.components.schemas.D' },
      },
      { command: 'remove', reason: 'r', args: { target: '$.components.schemas.A' } },
    ]);
    expect(r.diagnostics).toEqual([]);
    expect(r.spec).toEqual({
      info: { title: 'T' },
      components: { schemas: { B: { type: 'string' }, D: { type: 'object', title: 'C' } } },
    });
  });

  it('order matters: removing before copying fails the copy', () => {
    const r = applyTransforms(spec, [
      { command: 'remove', reason: 'r', args: { target: '$.components.schemas.A' } },
      {
        command: 'copy',
        reason: 'r',
        args: { from: '$.components.schemas.A', to: '$.components.schemas.C' },
      },
    ]);
    expect(r.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1201', '/openapi/transforms/1/args/from'],
    ]);
    expect(r.spec).toEqual({
      info: { title: 'T' },
      components: { schemas: { B: { type: 'string' } } },
    });
  });

  it('skips a failing transform as a whole and carries on', () => {
    const r = applyTransforms(spec, [
      { command: 'update', reason: 'r', args: { target: ['$.info.title', '$.nope'], value: 'X' } },
      { command: 'update', reason: 'r', args: { target: '$.info.title', value: 'Y' } },
    ]);
    expect(r.diagnostics).toHaveLength(1);
    expect(r.spec).toEqual({ ...spec, info: { title: 'Y' } });
  });

  it('uses a custom pointer base', () => {
    const r = applyTransforms(spec, [{ command: 'remove', reason: 'r', args: { target: '$.x' } }], {
      pointer: '/openapi/transformations',
    });
    expect(r.diagnostics[0]?.pointer).toBe('/openapi/transformations/0/args/target');
  });

  it('validates entries without crashing', () => {
    const r = applyTransforms(spec, [
      'nope',
      { command: 'rename', reason: 'r', args: {} },
      { command: 'update', reason: 'r' },
      { command: 'update', reason: 'r', args: { target: '$.info.title' } },
      { command: 'merge', reason: 'r', args: { target: 3, value: [] } },
      { command: 'copy', args: { from: '$.info', to: '$.components.info' }, extra: 1 },
      { command: 'remove', reason: ' ', args: { target: '$.info', keys: ['title'], bogus: true } },
    ]);
    expect(r.diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1203', 'error', '/openapi/transforms/0'],
      ['FER1203', 'error', '/openapi/transforms/1/command'],
      ['FER1203', 'error', '/openapi/transforms/2/args'],
      ['FER1203', 'error', '/openapi/transforms/3/args/value'],
      ['FER1203', 'error', '/openapi/transforms/4/args/target'],
      ['FER1203', 'error', '/openapi/transforms/4/args/value'],
      ['FER1204', 'warning', '/openapi/transforms/5/extra'],
      ['FER1204', 'warning', '/openapi/transforms/5/reason'],
      ['FER1204', 'warning', '/openapi/transforms/6/reason'],
      ['FER1204', 'warning', '/openapi/transforms/6/args/bogus'],
    ]);
    expect(r.diagnostics.every((d) => d.suggestion.length > 0)).toBe(true);
    // Warnings do not stop a transform.
    expect(r.spec).toEqual({
      info: {},
      components: {
        schemas: { A: { type: 'object' }, B: { type: 'string' } },
        info: { title: 'T' },
      },
    });
  });

  it('is deterministic', () => {
    const transforms = [
      { command: 'merge', reason: 'r', args: { target: '$..[?(@.type)]', value: { 'x-a': [1] } } },
      { command: 'remove', reason: 'r', args: { target: '$..type' } },
    ];
    const a = applyTransforms(spec, transforms);
    const b = applyTransforms(structuredClone(spec), structuredClone(transforms));
    expect(stableStringify(a.spec)).toBe(stableStringify(b.spec));
    expect(JSON.stringify(a)).toBe(JSON.stringify(b));
  });
});

const json = fc.letrec((tie) => ({
  value: fc.oneof(
    { depthSize: 'small' },
    fc.constantFrom(null, true, false, 0, 1, 'a', 'b'),
    fc.array(tie('value'), { maxLength: 3 }),
    fc.dictionary(fc.constantFrom('a', 'b', 'type', 'name'), tie('value'), { maxKeys: 3 }),
  ),
})).value;

const paths = fc.constantFrom(
  '$',
  '$.a',
  '$.b',
  '$..a',
  '$..type',
  '$.*',
  '$..*',
  '$..[0]',
  '$..[?(@.name == "a")]',
  '$.a.b',
  '$.c',
);

const transform = fc.oneof(
  fc.record({
    command: fc.constant('update'),
    reason: fc.constant('p'),
    args: fc.record({ target: paths, value: json }),
  }),
  fc.record({
    command: fc.constantFrom('append', 'merge'),
    reason: fc.constant('p'),
    args: fc.record({
      target: paths,
      value: fc.dictionary(fc.constantFrom('a', 'z'), json, { maxKeys: 2 }),
    }),
  }),
  fc.record({
    command: fc.constant('remove'),
    reason: fc.constant('p'),
    args: fc.record({ target: paths }),
  }),
  fc.record({
    command: fc.constantFrom('copy', 'move'),
    reason: fc.constant('p'),
    args: fc.record({ from: paths, to: paths }),
  }),
);

describe('properties', () => {
  it('never mutates the input spec or the transforms, and is deterministic', () => {
    fc.assert(
      fc.property(
        fc.dictionary(fc.constantFrom('a', 'b', 'c', 'type'), json, { maxKeys: 4 }),
        fc.array(transform, { maxLength: 4 }),
        (input, transforms) => {
          const before = structuredClone(input);
          const tBefore = structuredClone(transforms);
          const first = applyTransforms(input, transforms);
          expect(input).toEqual(before);
          expect(transforms).toEqual(tBefore);
          const second = applyTransforms(input, transforms);
          expect(JSON.stringify(second)).toBe(JSON.stringify(first));
          // Output never aliases input.
          if (typeof first.spec === 'object' && first.spec !== null)
            expect(first.spec).not.toBe(input);
        },
      ),
      { numRuns: 300 },
    );
  });

  it('diagnostics always carry a FER12xx code, a transforms pointer and a suggestion', () => {
    fc.assert(
      fc.property(
        fc.dictionary(fc.constantFrom('a', 'b'), json, { maxKeys: 3 }),
        fc.array(transform, { maxLength: 3 }),
        (input, transforms) => {
          for (const d of applyTransforms(input, transforms).diagnostics) {
            expect(d.code).toMatch(/^FER12\d\d$/);
            expect(d.pointer).toMatch(/^\/openapi\/transforms\/\d+/);
            expect(d.suggestion).not.toBe('');
          }
        },
      ),
      { numRuns: 200 },
    );
  });
});

describe('applyConfigTransforms', () => {
  const t = [{ command: 'remove', reason: 'r', args: { target: '$.info' } }];

  it('reads openapi.transforms', () => {
    const r = applyConfigTransforms(spec, { openapi: { transforms: t } });
    expect(r.diagnostics).toEqual([]);
    expect(r.spec).not.toHaveProperty('info');
  });

  it('accepts the deprecated openapi.transformations with a warning', () => {
    const r = applyConfigTransforms(spec, { openapi: { transformations: t } });
    expect(r.diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1212', 'warning', '/openapi/transformations'],
    ]);
    expect(r.spec).not.toHaveProperty('info');
  });

  it('prefers transforms when both are set', () => {
    const r = applyConfigTransforms(spec, {
      openapi: {
        transforms: t,
        transformations: [{ command: 'remove', reason: 'r', args: { target: '$.x' } }],
      },
    });
    expect(r.diagnostics.map((d) => [d.code, d.severity])).toEqual([['FER1212', 'error']]);
    expect(r.spec).not.toHaveProperty('info');
  });

  it('passes the spec through when there is nothing to do, and rejects a non-list', () => {
    expect(applyConfigTransforms(spec, {})).toEqual({ spec, diagnostics: [] });
    expect(applyConfigTransforms(spec, { openapi: { code_samples: 'x' } })).toEqual({
      spec,
      diagnostics: [],
    });
    expect(applyConfigTransforms(spec, { openapi: { transforms: null } }).spec).toEqual(spec);
    const r = applyConfigTransforms(spec, { openapi: { transforms: {} } });
    expect(r.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1203', '/openapi/transforms'],
    ]);
    expect(r.spec).toEqual(spec);
  });
});
