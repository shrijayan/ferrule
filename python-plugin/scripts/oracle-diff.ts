/**
 * Oracle comparison: converts an oracle's stainless.yml + OpenAPI pair, runs the pinned Forge CLI
 * with dist/plugin.wasm, and compares the generated code files with the oracle SDK, ignoring
 * only the first (generated-by) line.
 *
 *   node scripts/oracle-diff.ts <config-dir> <sdk-dir> [--out dir] [--diff path-substring]
 *
 * In scope: `_client.py`, `pagination.py`, `resources/**` and `types/**` under `src/<package>/`;
 * everything else is runtime (runtime/) or project files (src/emit/files/).
 */
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { forgeBinary, readTree, wasm } from './golden.ts';

const repo = join(fileURLToPath(import.meta.url), '..', '..', '..');

export interface OracleDiff {
  readonly identical: string[];
  readonly different: string[];
  readonly missing: string[];
  readonly extra: string[];
  readonly actual: Map<string, string>;
  readonly expected: Map<string, string>;
}

function inScope(path: string, pkg: string): boolean {
  const prefix = `src/${pkg}/`;
  if (!path.startsWith(prefix) || !path.endsWith('.py')) return false;
  const rest = path.slice(prefix.length);
  return (
    rest === '_client.py' ||
    rest === 'pagination.py' ||
    rest.startsWith('resources/') ||
    rest.startsWith('types/')
  );
}

const body = (text: string): string => text.slice(text.indexOf('\n') + 1);

export function oracleDiff(
  configDir: string,
  sdkDir: string,
  outDir?: string,
  files = { spec: 'openapi.yml', config: 'stainless-config.yml' },
): OracleDiff {
  const work = mkdtempSync(join(tmpdir(), 'ferrule-oracle-'));
  try {
    const spec = join(work, 'annotated.json');
    execFileSync(
      'node',
      [
        join(repo, 'converter', 'src', 'cli.ts'),
        join(configDir, files.spec),
        join(configDir, files.config),
        '-o',
        spec,
      ],
      { stdio: ['ignore', 'pipe', 'inherit'] },
    );
    const pkg = (JSON.parse(readFileSync(spec, 'utf8')) as Record<string, { packageName: string }>)[
      'x-ferrule-config'
    ]?.packageName;
    if (pkg === undefined) throw new Error('annotated spec has no x-ferrule-config.packageName');
    const out = outDir ?? join(work, 'out');
    rmSync(out, { recursive: true, force: true });
    execFileSync(forgeBinary, ['--target', `wasm://${wasm}`, '--out', out, spec], {
      stdio: ['ignore', 'pipe', 'inherit'],
    });
    // The pipeline formats generated code with ruff (phase 3 task D); do the same here.
    execFileSync(
      'uvx',
      ['ruff@0.17.1', 'format', '--quiet', '--isolated', '--line-length=120', out],
      { stdio: ['ignore', 'pipe', 'inherit'] },
    );
    const actual = new Map([...readTree(out)].filter(([p]) => inScope(p, pkg)));
    const expected = new Map([...readTree(sdkDir)].filter(([p]) => inScope(p, pkg)));
    const result: OracleDiff = {
      identical: [],
      different: [],
      missing: [],
      extra: [],
      actual,
      expected,
    };
    for (const [path, content] of expected) {
      const got = actual.get(path);
      if (got === undefined) result.missing.push(path);
      else if (body(got) === body(content)) result.identical.push(path);
      else result.different.push(path);
    }
    for (const path of actual.keys()) if (!expected.has(path)) result.extra.push(path);
    return result;
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

function main(): void {
  const args = process.argv.slice(2);
  const flag = (name: string): string | undefined => {
    const i = args.indexOf(name);
    return i < 0 ? undefined : args.splice(i, 2)[1];
  };
  const outDir = flag('--out');
  const show = flag('--diff');
  const [configDir, sdkDir] = args;
  if (configDir === undefined || sdkDir === undefined) {
    console.error('usage: oracle-diff.ts <config-dir> <sdk-dir> [--out dir] [--diff substring]');
    process.exit(2);
  }
  const spec = flag('--spec');
  const config = flag('--config');
  const r = oracleDiff(configDir, sdkDir, outDir, {
    spec: spec ?? 'openapi.yml',
    config: config ?? 'stainless-config.yml',
  });
  for (const p of r.different) console.log(`DIFFERENT  ${p}`);
  for (const p of r.missing) console.log(`MISSING    ${p}`);
  for (const p of r.extra) console.log(`EXTRA      ${p}`);
  const total = r.expected.size;
  console.log(
    `identical ${r.identical.length}/${total} (${((100 * r.identical.length) / Math.max(total, 1)).toFixed(1)}%), different ${r.different.length}, missing ${r.missing.length}, extra ${r.extra.length}`,
  );
  if (show !== undefined) {
    const work = mkdtempSync(join(tmpdir(), 'ferrule-oracle-show-'));
    for (const p of r.different.filter((d) => d.includes(show))) {
      const a = join(work, 'a');
      const b = join(work, 'b');
      writeFileSync(a, body(r.expected.get(p) ?? ''));
      writeFileSync(b, body(r.actual.get(p) ?? ''));
      try {
        execFileSync('diff', ['-u', '--label', `oracle/${p}`, '--label', `ours/${p}`, a, b]);
      } catch (e) {
        console.log(String((e as { stdout: Buffer }).stdout));
      }
    }
    rmSync(work, { recursive: true, force: true });
  }
  process.exit(r.identical.length === total && r.extra.length === 0 ? 0 : 1);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
