/**
 * Golden comparison of the project-file emitters against the pinned OneBusAway oracle, byte for
 * byte after the branding substitutions of BRANDING.md. Skipped when the oracle isn't fetched
 * (`python3 oracles/fetch.py onebusaway`).
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { emitProjectFiles } from '../../src/emit/files/index.ts';
import type { ProjectFilesInput } from '../../src/emit/files/input.ts';
import { brand, DROPPED_FILES, NOT_GENERATED, TEMPLATE_FILES } from './oracle-files.ts';

const here = dirname(fileURLToPath(import.meta.url));
const oracle = join(here, '../../../oracles/cache/onebusaway-python');
const input = JSON.parse(
  readFileSync(join(here, 'onebusaway-input.json'), 'utf8'),
) as ProjectFilesInput;

/**
 * Known, unexplained diffs. The oracle parenthesizes some one-name imports in generated tests
 * (here 73 columns; lithic has one at 37) while keeping longer ones on one line: most likely the
 * generator's import list held names that ruff's F401 fix removed afterwards, which no input
 * reveals. Compared with parenthesized imports collapsed; ruff treats both the same.
 */
const KNOWN_DIFFS: readonly string[] = [
  'tests/api_resources/test_arrivals_and_departures_for_location.py',
];

const collapseImports = (text: string): string =>
  text.replace(
    /^from (\S+) import \(\n((?: {4}\w+,\n)+)\)$/gm,
    (_, mod: string, names: string) =>
      `from ${mod} import ${names
        .trim()
        .replace(/,\n {4}/g, ', ')
        .replace(/,$/, '')}`,
  );

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    if (e.isDirectory()) return e.name === '.git' || e.name === 'src' ? [] : walk(p);
    return [relative(oracle, p)];
  });
}

describe.skipIf(!existsSync(oracle))('OneBusAway oracle project files', () => {
  const files = emitProjectFiles(input);
  const oracleFiles = walk(oracle);
  const ownerFiles = (p: string): boolean =>
    NOT_GENERATED.includes(p) || (p.startsWith('examples/') && p !== 'examples/.keep');

  it.each([...files.keys()])('%s matches the oracle', (path) => {
    const expected = brand(path, readFileSync(join(oracle, path), 'utf8'));
    if (KNOWN_DIFFS.includes(path)) {
      expect(files.get(path)).not.toBe(expected);
      expect(files.get(path)).toBe(collapseImports(expected));
    } else {
      expect(files.get(path)).toBe(expected);
    }
  });

  it('emits every generated non-src file of the oracle', () => {
    const missing = oracleFiles.filter(
      (p) => !files.has(p) && !ownerFiles(p) && !DROPPED_FILES.includes(p),
    );
    expect(missing).toEqual([]);
  });

  it('classifies every template file', () => {
    for (const p of TEMPLATE_FILES) expect(files.has(p) || DROPPED_FILES.includes(p)).toBe(true);
  });
});
