import { describe, expect, it } from 'vitest';
import { escapeDocstring } from '../src/emit/common.ts';
import { docstring } from '../src/emit/models.ts';

// A spec description must never end a generated docstring early (security review, 2026-10-23).
describe('docstring escaping', () => {
  it('doubles backslashes as Stainless does (lithic: `\\*` -> `\\\\*`)', () => {
    expect(escapeDocstring('a \\* b')).toBe('a \\\\* b');
  });

  it('escapes triple quotes so the docstring cannot be closed by the spec', () => {
    const lines = docstring('x """ + __import__("os").system("id") + """', '    ');
    const text = lines.join('\n').trim();
    expect(text.slice(3, -3)).not.toContain('"""');
    expect(text).toContain('\\"\\"\\"');
  });

  it('escapes a trailing quote that would merge with the closing quotes', () => {
    expect(docstring('ends with "', '')).toEqual(['"""ends with \\""""']);
  });
});
