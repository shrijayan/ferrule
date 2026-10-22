import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  compareTrees,
  fixtures,
  forgeBinary,
  goldenDir,
  readTree,
  runForge,
  wasm,
} from '../scripts/golden.ts';
import { forgeToModel } from '../src/adapter/to-model.ts';
import { emitRootFiles } from '../src/emit/project.ts';
import { fakeForge } from './helpers/fake-forge.ts';

/** The tree at `dir` after `ruff format` with the oracles' line length. */
function ruffFormatted(dir: string): Map<string, string> {
  const tmp = mkdtempSync(join(tmpdir(), 'ferrule-ruff-'));
  try {
    cpSync(dir, tmp, { recursive: true });
    spawnSync('uvx', ['ruff@0.17.1', 'format', '--quiet', '--isolated', '--line-length=120', tmp]);
    return readTree(tmp);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

/** What plugin.ts does inside Forge, run in-process on a Forge-shaped object. */
function generateInProcess(fixture: string): Map<string, string> {
  const forge = fakeForge(JSON.parse(readFileSync(fixture, 'utf8')));
  const files = new Map<string, string>();
  const root = forgeToModel(forge);
  if (root !== undefined) for (const [p, c] of emitRootFiles(root)) files.set(p, c);
  return files;
}

describe.each(fixtures().map((f) => [basename(f, '.json'), f]))('golden %s', (name, fixture) => {
  const golden = readTree(join(goldenDir, name));

  it('has a golden tree', () => {
    expect(golden.size).toBeGreaterThan(0);
  });

  // The fake Forge models what the synthetic fixtures use, not path-level parameters or every
  // nullable form of a real spec; the full OneBusAway fixture is checked through the CLI only.
  it('adapter + emitters in-process match the golden files', () => {
    const actual = generateInProcess(fixture);
    expect(compareTrees(actual, golden)).toEqual([]);
    for (const [path, content] of golden) expect(actual.get(path)).toBe(content);
  });

  const haveForge = existsSync(forgeBinary) && existsSync(wasm);
  it.skipIf(!haveForge)('forge CLI + plugin.wasm match the golden files', () => {
    expect(compareTrees(runForge(fixture), golden)).toEqual([]);
  });
});

describe('determinism', () => {
  it('generates byte-identical output twice', () => {
    for (const fixture of fixtures()) {
      expect(generateInProcess(fixture)).toEqual(generateInProcess(fixture));
    }
  });
});

describe('compareTrees', () => {
  it('reports unexpected, differing and missing files', () => {
    const a = new Map([
      ['x', '1'],
      ['y', '2'],
    ]);
    const b = new Map([
      ['y', '3'],
      ['z', '4'],
    ]);
    expect(compareTrees(a, b)).toEqual([
      'unexpected file x',
      'content differs: y',
      'missing file z',
    ]);
  });
});

describe('oracle parity (onebusaway, when oracles/cache is fetched)', () => {
  const oracle = join(goldenDir, '..', '..', '..', 'oracles', 'cache', 'onebusaway-python');
  // The full OneBusAway golden is the plugin's output for the converter's output of the pinned
  // pair. The pipeline formats generated code with ruff (phase 3 task D), so compare after
  // `ruff format`, which only re-wraps long lines the printer leaves long.
  const haveRuff = existsSync(oracle) && spawnSync('uvx', ['--version']).status === 0;
  const golden = haveRuff ? ruffFormatted(join(goldenDir, 'onebusaway')) : new Map();
  const dropHeader = (s: string): string => s.slice(s.indexOf('\n') + 1);
  it.skipIf(!haveRuff).each([...golden.keys()])('%s matches but for the header', (path) => {
    const expected = readFileSync(join(oracle, path), 'utf8');
    expect(dropHeader(golden.get(path) ?? '')).toBe(dropHeader(expected));
  });
});
