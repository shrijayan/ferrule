import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { main } from '../src/cli.ts';

const dir = mkdtempSync(join(tmpdir(), 'ferrule-convert-'));
const spec = join(dir, 'openapi.yml');
const config = join(dir, 'stainless.yml');
writeFileSync(spec, 'paths:\n  /a:\n    get: { responses: {} }\n');
/** Keys the public schema requires, before `resources`. */
const HEAD = `organization: { name: acme }
environments: { production: https://api.acme.test }
readme:
  example_requests:
    default: { type: request, endpoint: get /a, params: {} }
    headline: { type: request, endpoint: get /a, params: {} }
`;
writeFileSync(config, `${HEAD}resources:\n  a:\n    methods:\n      list: get /a\n`);

function run(argv: string[]): { code: number; out: string[]; err: string[] } {
  const out: string[] = [];
  const err: string[] = [];
  const code = main(argv, { stdout: (l) => out.push(l), stderr: (l) => err.push(l) });
  return { code, out, err };
}

describe('ferrule-convert', () => {
  it('writes the annotated spec', () => {
    const target = join(dir, 'out.json');
    const r = run([spec, config, '-o', target]);
    expect(r.code).toBe(0);
    expect(r.out).toEqual([`wrote ${target}`]);
    const written = readFileSync(target, 'utf8');
    expect(written.endsWith('}\n')).toBe(true);
    expect(JSON.parse(written).paths['/a'].get['x-ferrule-method-name']).toBe('list');
  });

  it('prints usage on bad arguments', () => {
    expect(run([spec, config]).code).toBe(2);
    expect(run([spec, '--out', 'x']).err[0]).toMatch(/^usage:/);
  });

  it('reports unreadable input', () => {
    const r = run([join(dir, 'missing.yml'), config, '-o', join(dir, 'x.json')]);
    expect(r.code).toBe(2);
    expect(r.err[0]).toMatch(/^ferrule-convert: /);
  });

  it('exits 1 and writes nothing on error diagnostics', () => {
    const bad = join(dir, 'bad.yml');
    writeFileSync(bad, `${HEAD}resources:\n  a:\n    methods:\n      list: get /zzz\n`);
    const target = join(dir, 'bad.json');
    const r = run([spec, bad, '-o', target]);
    expect(r.code).toBe(1);
    expect(r.err[0]).toContain('FER1001');
    expect(r.err[0]).toContain('#/resources/a/methods/list');
    expect(existsSync(target)).toBe(false);
  });

  it('exits 1 with located diagnostics on an invalid config', () => {
    const invalid = join(dir, 'invalid.yml');
    writeFileSync(invalid, 'resources: {}\n');
    const target = join(dir, 'invalid.json');
    const r = run([spec, invalid, '-o', target]);
    expect(r.code).toBe(1);
    expect(r.err[0]).toMatch(/^error FER1111 /);
    expect(existsSync(target)).toBe(false);
  });

  it('runs as a bin under plain node', () => {
    const cli = fileURLToPath(new URL('../src/cli.ts', import.meta.url));
    const target = join(dir, 'bin.json');
    execFileSync(process.execPath, [cli, spec, config, '-o', target]);
    expect(readFileSync(target, 'utf8')).toBe(readFileSync(join(dir, 'out.json'), 'utf8'));
  });
});
