import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { validateConfig } from '../../src/schema/validate.ts';

const path = fileURLToPath(
  new URL('../../../oracles/cache/onebusaway-config/stainless-config.yml', import.meta.url),
);

describe.skipIf(!existsSync(path))('OneBusAway stainless.yml', () => {
  it('validates against the public schema with zero diagnostics', () => {
    const result = validateConfig(readFileSync(path, 'utf8'));
    expect(result.diagnostics).toEqual([]);
    expect(result.config?.organization?.name).toBe('onebusaway-sdk');
  });
});
