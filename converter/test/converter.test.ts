import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import {
  ConfigSchema,
  convert,
  DEFAULT_EDITION,
  isEdition,
  parseConfig,
  pipeline,
  toJsonPointer,
} from '../src/index.ts';

describe('convert', () => {
  it('returns the spec unchanged with no diagnostics', () => {
    fc.assert(
      fc.property(fc.dictionary(fc.string(), fc.jsonValue()), (spec) => {
        const result = convert(spec, {});
        expect(result.spec).toEqual(spec);
        expect(result.diagnostics).toEqual([]);
      }),
    );
  });
});

describe('pipeline', () => {
  it('applies transforms in order', () => {
    const t = pipeline([(s) => ({ ...s, a: 1 }), ({ a, ...rest }) => ({ ...rest, a: 2, b: a })]);
    expect(t({}, {})).toEqual({ a: 2, b: 1 });
  });
});

describe('toJsonPointer', () => {
  it('escapes ~ and /', () => {
    expect(toJsonPointer(['paths', '/users/{id}', 0, 'a~b'])).toBe('/paths/~1users~1{id}/0/a~0b');
    expect(toJsonPointer([])).toBe('');
  });
});

describe('editions and schema', () => {
  it('recognises editions', () => {
    expect(isEdition(DEFAULT_EDITION)).toBe(true);
    expect(isEdition('1999-01-01')).toBe(false);
    expect(isEdition(1)).toBe(false);
  });

  it('parses a minimal config and keeps unknown keys', () => {
    const parsed = ConfigSchema.parse({ organization: { name: 'acme' }, extra: true });
    expect(parsed).toHaveProperty('extra', true);
    expect(() => ConfigSchema.parse({ organization: { name: 1 } })).toThrow();
  });

  it('parses YAML text', () => {
    expect(parseConfig('organization:\n  name: acme\n').organization?.name).toBe('acme');
    expect(parseConfig('')).toEqual({});
  });
});
