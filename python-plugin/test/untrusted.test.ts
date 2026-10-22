/**
 * Spec and config text is untrusted: values that become Python identifiers are validated, and
 * values that become string literals are escaped, so nothing can inject code into the SDK.
 */
import type { Json } from '@cloudflare/forge/plugin';
import { describe, expect, it } from 'vitest';
import { readClient, readMethodExtensions } from '../src/adapter/extensions.ts';
import { FerruleError } from '../src/diagnostics.ts';

const fail = (): FerruleError => new FerruleError('FER101', '#', 'bad', 'fix');

const CLIENT: Record<string, Json> = {
  name: 'acme',
  envPrefix: 'ACME',
  opts: [{ name: 'api_key', type: 'string', nullable: false, readEnv: 'ACME_API_KEY' }],
  environments: [{ name: 'production', url: 'https://api.acme.test' }],
  securitySchemes: {},
  security: [],
  timeoutSeconds: 60,
};

function code(f: () => unknown): string {
  try {
    f();
  } catch (e) {
    if (e instanceof FerruleError) return e.code;
    throw e;
  }
  return 'none';
}

describe('untrusted x-ferrule-client values', () => {
  it('rejects an environment URL that is not http(s) or could break a string', () => {
    for (const url of ['javascript:x', 'https://a"+__import__("os")+"', 'https://{x}']) {
      expect(code(() => readClient({ ...CLIENT, environments: [{ name: 'p', url }] }, fail))).toBe(
        'FER101',
      );
    }
  });

  it('rejects an env prefix that is not an upper-case identifier', () => {
    expect(code(() => readClient({ ...CLIENT, envPrefix: 'A_") or ("' }, fail))).toBe('FER101');
  });

  it('rejects a readEnv that is not an identifier', () => {
    const opts = [{ name: 'api_key', type: 'string', nullable: false, readEnv: 'X")\nimport os#' }];
    expect(code(() => readClient({ ...CLIENT, opts }, fail))).toBe('FER101');
  });
});

describe('untrusted x-ferrule-pagination fields', () => {
  it.each([
    ['response', { 'data; import os': 'items' }],
    ['response', { 'data.[].id)': 'cursor_item_id' }],
    ['request', { 'a-b': 'next_cursor_param' }],
  ])('rejects %s field %j', (side, map) => {
    const page = { scheme: 'cursor', page: 'p', request: {}, response: {}, [side]: map };
    const ext: [string, Json][] = [
      ['x-ferrule-resource', { path: ['a'] }],
      ['x-ferrule-method-name', 'list'],
      ['x-ferrule-package', { name: 'acme', repo: '' }],
      ['x-ferrule-pagination', page as Json],
    ];
    expect(code(() => readMethodExtensions(ext, '#/p'))).toBe('FER101');
  });
});
