import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { convertText } from '../src/index.ts';

const root = fileURLToPath(new URL('../../oracles/cache/onebusaway-config/', import.meta.url));
const present = existsSync(root);

describe.skipIf(!present)('OneBusAway oracle pair', () => {
  const read = (f: string) => readFileSync(`${root}${f}`, 'utf8');
  const result = present
    ? convertText(read('openapi.yml'), read('stainless-config.yml'))
    : { output: '', diagnostics: [] };

  it('converts without diagnostics', () => {
    expect(result.diagnostics).toEqual([]);
  });

  it('annotates agencies_with_coverage.list without pagination', () => {
    const op = JSON.parse(result.output).paths['/api/where/agencies-with-coverage.json'].get;
    expect(op['x-ferrule-resource']).toEqual({ path: ['agencies_with_coverage'] });
    expect(op['x-ferrule-method-name']).toBe('list');
    expect(op).not.toHaveProperty('x-ferrule-pagination');
  });

  it('matches the committed snapshot', async () => {
    await expect(result.output).toMatchFileSnapshot('__snapshots__/onebusaway.json.snap');
  });
});
