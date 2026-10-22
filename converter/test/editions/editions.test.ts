import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import {
  applyEdition,
  DEFAULT_EDITION,
  EDITION_NAMES,
  EDITIONS,
  IMPLICIT_EDITION,
  isEdition,
  isPythonEdition,
  PYTHON_EDITIONS,
} from '../../src/editions/index.ts';
import type { Config } from '../../src/schema/config.ts';
import { STAINLESS_CONFIG_SCHEMA } from '../../src/schema/stainless-schema.ts';

/** Walk plain object keys. */
function at(node: unknown, ...keys: string[]): unknown {
  return keys.reduce<unknown>((n, k) => (n as Record<string, unknown>)[k], node);
}

/** The `const` values the public schema lists for an `anyOf` edition field. */
function schemaConsts(node: unknown): string[] {
  const anyOf = (node as { anyOf: { const?: string }[] }).anyOf;
  return anyOf.flatMap((b) => (b.const === undefined ? [] : [b.const])).sort();
}

describe('edition tables', () => {
  it('list exactly the editions the public schema names', () => {
    const top = at(STAINLESS_CONFIG_SCHEMA, 'properties', 'edition');
    expect([...EDITION_NAMES].sort()).toEqual(schemaConsts(top));
    const py = at(STAINLESS_CONFIG_SCHEMA, 'properties', 'targets', 'properties', 'python');
    expect(Object.keys(PYTHON_EDITIONS).sort()).toEqual(
      schemaConsts(at(py, 'properties', 'edition')),
    );
  });

  it('are ordered oldest first', () => {
    expect(EDITION_NAMES).toEqual([...EDITION_NAMES].sort());
    expect(Object.keys(PYTHON_EDITIONS)).toEqual(Object.keys(PYTHON_EDITIONS).sort());
  });

  it('switch the normalizer at 2026-05-06 and uv at python.2025-11-20', () => {
    expect(EDITION_NAMES.map((e) => EDITIONS[e].normalizer)).toEqual([
      'v1',
      'v1',
      'v1',
      'v1',
      'v2',
    ]);
    expect(PYTHON_EDITIONS['python.2025-10-08'].useUv).toBe(false);
    expect(PYTHON_EDITIONS['python.2025-11-20'].useUv).toBe(true);
  });

  it('recognise edition names', () => {
    expect(isEdition(DEFAULT_EDITION)).toBe(true);
    expect(isEdition('toString')).toBe(false);
    expect(isPythonEdition('python.2025-11-20')).toBe(true);
    expect(isPythonEdition(2025)).toBe(false);
  });
});

describe('applyEdition', () => {
  it('gives a config without editions the first editions’ defaults', () => {
    const r = applyEdition({});
    expect(r.edition).toBe(IMPLICIT_EDITION);
    expect(r.pythonEdition).toBe('python.2025-10-08');
    expect(r.defaults).toEqual({ normalizer: 'v1', useUv: false });
    expect(r.config).toEqual({ targets: { python: { options: { use_uv: false } } } });
    expect(r.diagnostics).toEqual([]);
  });

  it('applies the newest editions', () => {
    const config: Config = {
      edition: '2026-05-06',
      targets: { python: { package_name: 'acme', edition: 'python.2025-11-20' } },
    };
    const r = applyEdition(config);
    expect(r.defaults).toEqual({ normalizer: 'v2', useUv: true });
    expect(r.config.targets?.python).toEqual({
      package_name: 'acme',
      edition: 'python.2025-11-20',
      options: { use_uv: true },
    });
    expect(config.targets?.python).not.toHaveProperty('options');
  });

  it('never overrides an explicit setting', () => {
    const r = applyEdition({
      targets: { python: { edition: 'python.2025-11-20', options: { use_uv: false, x: 1 } } },
    });
    expect(r.config.targets?.python).toMatchObject({ options: { use_uv: false, x: 1 } });
  });

  it('reports an unknown edition as FER1101 with a did-you-mean', () => {
    const r = applyEdition({ edition: '2026-05-07' });
    expect(r.edition).toBe(IMPLICIT_EDITION);
    expect(r.diagnostics).toEqual([
      {
        code: 'FER1101',
        severity: 'error',
        message:
          'unknown edition "2026-05-07"; Ferrule knows 2025-10-08, 2025-10-10, 2026-01-30, 2026-02-23, 2026-05-06',
        pointer: '/edition',
        suggestion: 'Did you mean "2026-05-06"?',
      },
    ]);
  });

  it('suggests upgrading for a far-off edition and reports python editions as FER1102', () => {
    const r = applyEdition({
      edition: '2030-01-01-preview',
      targets: { python: { edition: 7 } },
    });
    expect(r.diagnostics.map((d) => [d.code, d.pointer, d.suggestion])).toEqual([
      [
        'FER1101',
        '/edition',
        'Use one of 2025-10-08, 2025-10-10, 2026-01-30, 2026-02-23, 2026-05-06, or upgrade Ferrule if this edition is newer.',
      ],
      [
        'FER1102',
        '/targets/python/edition',
        'Use one of python.2025-10-08, python.2025-11-20, or upgrade Ferrule if this edition is newer.',
      ],
    ]);
  });

  it('is pure and idempotent', () => {
    const edition = fc.option(fc.constantFrom(...EDITION_NAMES, 'nope'), { nil: undefined });
    const pyEdition = fc.option(fc.constantFrom(...Object.keys(PYTHON_EDITIONS), 'nope'), {
      nil: undefined,
    });
    const useUv = fc.option(fc.boolean(), { nil: undefined });
    fc.assert(
      fc.property(edition, pyEdition, useUv, (e, pe, uv) => {
        const config: Config = {
          ...(e === undefined ? {} : { edition: e }),
          targets: {
            python: {
              ...(pe === undefined ? {} : { edition: pe }),
              ...(uv === undefined ? {} : { options: { use_uv: uv } }),
            },
          },
        };
        const before = structuredClone(config);
        const once = applyEdition(config);
        expect(config).toEqual(before);
        expect(applyEdition(once.config).config).toEqual(once.config);
        expect(at(once.config, 'targets', 'python', 'options', 'use_uv')).toBe(
          uv ?? once.defaults.useUv,
        );
      }),
    );
  });
});
