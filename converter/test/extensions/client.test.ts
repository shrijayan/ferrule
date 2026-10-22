import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { buildClient, timeoutSeconds, upperSnake } from '../../src/extensions/client.ts';

const spec = {
  servers: [{ url: 'https://spec.example.com' }],
  security: [{ SpecKey: [] }],
  components: {
    securitySchemes: {
      SpecKey: { type: 'apiKey', in: 'query', name: 'key' },
      Bearer: { type: 'http', scheme: 'Bearer' },
      Broken: { type: 'mutualTLS' },
    },
  },
};

describe('x-ferrule-client', () => {
  it('reads every client_settings field', () => {
    const { client, diagnostics } = buildClient(spec, {
      organization: { name: 'acme-corp' },
      environments: { production: 'https://api.acme.com', sandbox: 'https://sandbox.acme.com' },
      security: [{ Bearer: [] }, { Basic: ['read'] }],
      security_schemes: {
        Basic: { type: 'http', scheme: 'basic', 'x-stainless-auth': { disable_base64: true } },
        Bearer: { type: 'http', scheme: 'bearer', 'x-stainless-auth': { scheme_keyword: 'Token' } },
        OAuth: { type: 'oauth2', flows: { clientCredentials: { tokenUrl: '/t', scopes: {} } } },
        Oidc: { type: 'openIdConnect', openIdConnectUrl: 'https://id' },
        Digest: { type: 'http', scheme: 'digest' },
        Cookie: { type: 'apiKey', in: 'cookie', name: 'sid' },
      },
      client_settings: {
        default_client_name: 'Acme',
        default_env_prefix: 'ACME',
        opts: {
          api_key: {
            type: 'string',
            read_env: 'ACME_API_KEY',
            auth: { security_scheme: 'Bearer' },
            description: 'The key',
          },
          region: { type: 'string', nullable: true, default: 'us', server_variable: 'region' },
          org_id: { type: 'integer', send_in_header: 'X-Org', send_as_query_param: 'org' },
          flag: { type: 'boolean', auth: { security_scheme: 'Basic', role: 'username' } },
        },
        idempotency: { header: 'Idempotency-Key' },
        default_headers: { 'X-Version': '2', bad: 3 },
        omit_stainless_headers: true,
        default_timeout: 'PT1M30S',
        default_retries: { max_retries: 5, initial_delay_seconds: 1, max_delay_seconds: 30 },
        python: { default_http_limits: { max_connections: 10, keepalive_expiry: 5 } },
      },
      query_settings: { nested_format: 'dots', array_format: 'indices' },
    });
    expect(client).toEqual({
      name: 'Acme',
      envPrefix: 'ACME',
      opts: [
        {
          name: 'api_key',
          type: 'string',
          nullable: false,
          readEnv: 'ACME_API_KEY',
          description: 'The key',
          auth: { securityScheme: 'Bearer', role: 'value' },
        },
        {
          name: 'region',
          type: 'string',
          nullable: true,
          default: 'us',
          serverVariable: 'region',
        },
        { name: 'org_id', type: 'integer', nullable: false, send: { in: 'header', name: 'X-Org' } },
        {
          name: 'flag',
          type: 'boolean',
          nullable: false,
          auth: { securityScheme: 'Basic', role: 'username' },
        },
      ],
      environments: [
        { name: 'production', url: 'https://api.acme.com' },
        { name: 'sandbox', url: 'https://sandbox.acme.com' },
      ],
      defaultEnvironment: 'production',
      securitySchemes: {
        SpecKey: { type: 'apiKey', in: 'query', name: 'key' },
        Bearer: { type: 'http', scheme: 'bearer', keyword: 'Token' },
        Basic: { type: 'http', scheme: 'basic', disableBase64: true },
        OAuth: { type: 'oauth2', flows: { clientCredentials: { tokenUrl: '/t', scopes: {} } } },
        Oidc: { type: 'openIdConnect', url: 'https://id' },
        Digest: { type: 'http', scheme: 'digest' },
        Cookie: { type: 'apiKey', in: 'cookie', name: 'sid' },
      },
      security: [{ Bearer: [] }, { Basic: ['read'] }],
      timeoutSeconds: 90,
      retries: { maxRetries: 5, initialDelaySeconds: 1, maxDelaySeconds: 30 },
      idempotencyHeader: 'Idempotency-Key',
      defaultHeaders: { 'X-Version': '2' },
      omitPlatformHeaders: true,
      query: { nestedFormat: 'dots', arrayFormat: 'indices' },
      httpLimits: { maxConnections: 10, keepaliveExpiry: 5 },
    });
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1302', 'warning', '/client_settings/opts/org_id'],
      ['FER1303', 'warning', '/components/securitySchemes/Broken'],
    ]);
  });

  it('defaults to Stainless defaults, the spec server and the spec security', () => {
    const { client, diagnostics } = buildClient(spec, {
      organization: { name: 'OneBusAway SDK' },
    });
    expect(diagnostics).toHaveLength(1);
    expect(client).toMatchObject({
      name: 'OneBusAway SDK',
      envPrefix: 'ONE_BUS_AWAY_SDK',
      opts: [],
      environments: [{ name: 'production', url: 'https://spec.example.com' }],
      defaultEnvironment: 'production',
      security: [{ SpecKey: [] }],
      timeoutSeconds: 60,
      retries: { maxRetries: 2, initialDelaySeconds: 0.5, maxDelaySeconds: 8 },
      defaultHeaders: {},
      omitPlatformHeaders: false,
      query: { nestedFormat: 'brackets', arrayFormat: 'comma' },
    });
    expect(client).not.toHaveProperty('idempotencyHeader');
    expect(client).not.toHaveProperty('httpLimits');
  });

  it('falls back to the package name and leaves environments empty without servers', () => {
    const { client } = buildClient(
      { security: 'nope', servers: ['x'] },
      { targets: { python: { package_name: 'acme' } }, security: [{ A: 'x' }, 3] },
    );
    expect(client).toMatchObject({ name: 'acme', envPrefix: 'ACME', environments: [] });
    expect(client.security).toEqual([{ A: [] }]);
    expect(client).not.toHaveProperty('defaultEnvironment');
    expect(buildClient({}, {}).client.name).toBe('');
  });

  it('reports bad opts, undefined auth schemes and bad timeouts', () => {
    const { client, diagnostics } = buildClient(
      {},
      {
        environments: { a: 3 },
        client_settings: {
          opts: {
            x: { type: 'date' },
            y: 'nope',
            k: { type: 'string', auth: { security_scheme: 'S' } },
          },
          default_timeout: 'soon',
          python: { default_http_limits: {} },
        },
        query_settings: { nested_format: 'weird', array_format: 7 },
      },
    );
    expect(diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1301', 'error', '/client_settings/opts/x/type'],
      ['FER1301', 'error', '/client_settings/opts/y/type'],
      ['FER1304', 'error', '/client_settings/opts/k/auth/security_scheme'],
      ['FER1305', 'error', '/client_settings/default_timeout'],
    ]);
    expect(diagnostics[2]?.suggestion).toContain('(none defined)');
    expect(client).toMatchObject({
      timeoutSeconds: 60,
      environments: [],
      httpLimits: {},
      query: { nestedFormat: 'brackets', arrayFormat: 'comma' },
    });
  });
});

describe('timeoutSeconds', () => {
  it.each([
    [60000, 60],
    [1500, 1.5],
    ['PT60S', 60],
    ['PT0.5S', 0.5],
    ['PT2M', 120],
    ['PT1H', 3600],
    ['P1D', 86400],
  ])('%s -> %s s', (input, seconds) => {
    expect(timeoutSeconds(input)).toBe(seconds);
  });

  it.each([0, -1, Number.POSITIVE_INFINITY, 'P', 'PT', 'PT0S', '60', null, true])(
    'rejects %s',
    (input) => {
      expect(timeoutSeconds(input)).toBeUndefined();
    },
  );

  it('round-trips whole seconds written as PT<n>S', () => {
    fc.assert(
      fc.property(fc.integer({ min: 1, max: 10_000 }), (n) => {
        expect(timeoutSeconds(`PT${n}S`)).toBe(n);
        expect(timeoutSeconds(n * 1000)).toBe(n);
      }),
    );
  });
});

describe('upperSnake', () => {
  it.each([
    ['onebusaway-sdk', 'ONEBUSAWAY_SDK'],
    ['AcmeCorp', 'ACME_CORP'],
    ['  my  api ', 'MY_API'],
  ])('%s -> %s', (input, out) => {
    expect(upperSnake(input)).toBe(out);
  });

  it('always yields an upper-case identifier or nothing', () => {
    fc.assert(
      fc.property(fc.string(), (s) => {
        expect(upperSnake(s)).toMatch(/^([A-Z0-9]+(_[A-Z0-9]+)*)?$/);
      }),
    );
  });
});
