import { existsSync, readFileSync } from 'node:fs';
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
import { forgeToModel, groupToModel } from '../src/adapter/to-model.ts';
import { emitGroupFiles, emitRootFiles } from '../src/emit/project.ts';
import { fakeForge } from './helpers/fake-forge.ts';

/** What plugin.ts does inside Forge, run in-process on a Forge-shaped object. */
function generateInProcess(fixture: string): Map<string, string> {
  const forge = fakeForge(JSON.parse(readFileSync(fixture, 'utf8')));
  const files = new Map<string, string>();
  const root = forgeToModel(forge);
  if (root !== undefined) for (const [p, c] of emitRootFiles(root)) files.set(p, c);
  for (const group of forge.groups) {
    const client = groupToModel(group);
    if (client !== undefined) for (const [p, c] of emitGroupFiles(client)) files.set(p, c);
  }
  return files;
}

describe.each(fixtures().map((f) => [basename(f, '.json'), f]))('golden %s', (name, fixture) => {
  const golden = readTree(join(goldenDir, name));

  it('has a golden tree', () => {
    expect(golden.size).toBeGreaterThan(0);
  });

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
  const golden = readTree(join(goldenDir, 'onebusaway-spike'));
  const dropHeader = (s: string): string => s.slice(s.indexOf('\n') + 1);
  it.skipIf(!existsSync(oracle)).each([...golden.keys()])(
    '%s matches but for the header',
    (path) => {
      const expected = readFileSync(join(oracle, path), 'utf8');
      expect(dropHeader(golden.get(path) ?? '')).toBe(dropHeader(expected));
    },
  );
});
