import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { pascalCase, snakeCase, splitWords } from '../src/naming.ts';

describe('naming', () => {
  it('snake_cases word lists', () => {
    expect(snakeCase(['list', 'Users'])).toBe('list_users');
    expect(snakeCase(['class'])).toBe('class_');
    expect(snakeCase(['2fa', 'code'])).toBe('_2fa_code');
    expect(snakeCase(['a-b', '', 'c'])).toBe('ab_c');
  });

  it('PascalCases word lists', () => {
    expect(pascalCase(['user', 'ID'])).toBe('UserID');
    expect(pascalCase(['none'])).toBe('None_');
  });

  it('splits identifiers', () => {
    expect(splitWords('listUsers')).toEqual(['list', 'Users']);
    expect(splitWords('HTTPServer_v2')).toEqual(['HTTP', 'Server', 'v2']);
    expect(splitWords('Acme API')).toEqual(['Acme', 'API']);
  });

  it('snake_case output is always a valid lowercase identifier', () => {
    fc.assert(
      fc.property(fc.array(fc.stringMatching(/^[A-Za-z][A-Za-z0-9]*$/), { minLength: 1 }), (ws) => {
        expect(snakeCase(ws)).toMatch(/^[a-z_][a-z0-9_]*$/);
      }),
    );
  });
});
