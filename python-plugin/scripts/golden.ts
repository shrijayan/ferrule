/**
 * Golden test: runs the Forge CLI with dist/plugin.wasm on each test/fixtures/*.json and
 * compares the output byte for byte with test/golden/<fixture>/.
 *
 *   node scripts/golden.ts            # compare
 *   node scripts/golden.ts --update   # rewrite the goldens (a human approves the diff)
 */
import { execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const pluginDir = join(dirname(fileURLToPath(import.meta.url)), '..');
export const fixturesDir = join(pluginDir, 'test', 'fixtures');
export const goldenDir = join(pluginDir, 'test', 'golden');
export const wasm = join(pluginDir, 'dist', 'plugin.wasm');
const { FORGE_BINARY: forgeEnv } = process.env;
export const forgeBinary =
  forgeEnv ?? join(pluginDir, '..', '.cache', 'forge', 'target', 'release', 'forge');

/** Every file under `dir`, relative path -> content. */
export function readTree(dir: string): Map<string, string> {
  const out = new Map<string, string>();
  if (!existsSync(dir)) return out;
  const walk = (d: string): void => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else out.set(relative(dir, p).split('\\').join('/'), readFileSync(p, 'utf8'));
    }
  };
  walk(dir);
  return new Map([...out].sort(([a], [b]) => (a < b ? -1 : 1)));
}

/** Runs Forge with the plugin on `fixture` and returns the files it writes. */
export function runForge(fixture: string): Map<string, string> {
  const out = mkdtempSync(join(tmpdir(), 'ferrule-golden-'));
  try {
    execFileSync(forgeBinary, ['--target', `wasm://${wasm}`, '--out', out, fixture], {
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    // Code files only: the project files (README, pyproject, tests) have their own goldens
    // under test/files, and a generated pyproject.toml here would re-root ruff's config.
    return new Map([...readTree(out)].filter(([path]) => path.startsWith('src/')));
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
}

export function fixtures(): string[] {
  return readdirSync(fixturesDir)
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((f) => join(fixturesDir, f));
}

/** Differences between `actual` and `expected`, one message per path. */
export function compareTrees(actual: Map<string, string>, expected: Map<string, string>): string[] {
  const problems: string[] = [];
  for (const [path, content] of actual) {
    if (!expected.has(path)) problems.push(`unexpected file ${path}`);
    else if (expected.get(path) !== content) problems.push(`content differs: ${path}`);
  }
  for (const path of expected.keys()) if (!actual.has(path)) problems.push(`missing file ${path}`);
  return problems;
}

function main(): void {
  const update = process.argv.includes('--update');
  let failed = false;
  for (const fixture of fixtures()) {
    const name = basename(fixture, '.json');
    const target = join(goldenDir, name);
    const actual = runForge(fixture);
    if (update) {
      rmSync(target, { recursive: true, force: true });
      for (const [path, content] of actual) {
        mkdirSync(dirname(join(target, path)), { recursive: true });
        writeFileSync(join(target, path), content);
      }
      console.log(`updated ${relative(pluginDir, target)} (${actual.size} files)`);
      continue;
    }
    const problems = compareTrees(actual, readTree(target));
    for (const p of problems) console.error(`${name}: ${p}`);
    if (problems.length > 0) failed = true;
    else console.log(`${name}: ${actual.size} files match`);
  }
  if (failed) {
    console.error('golden output differs; run `pnpm golden --update` and review the diff');
    process.exit(1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
