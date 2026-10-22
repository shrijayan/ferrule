import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const src = join(dirname(fileURLToPath(import.meta.url)), '..', 'src');

function files(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? files(join(dir, e.name)) : [join(dir, e.name)],
  );
}

describe('ADR 0002: Forge stays behind the adapter', () => {
  it('only adapter/ and plugin.ts import @cloudflare/forge', () => {
    const offenders = files(src)
      .map((f) => relative(src, f).split('\\').join('/'))
      .filter((f) => !f.startsWith('adapter/') && f !== 'plugin.ts')
      .filter((f) => readFileSync(join(src, f), 'utf8').includes('@cloudflare/forge'));
    expect(offenders).toEqual([]);
  });

  it('adapter/ imports Forge types only', () => {
    for (const f of files(join(src, 'adapter'))) {
      for (const line of readFileSync(f, 'utf8').split('\n')) {
        if (line.includes("'@cloudflare/forge")) expect(line).toMatch(/^(import type|} from)/);
      }
    }
  });
});
