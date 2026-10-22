import { describe, expect, it } from 'vitest';
import { convertText } from '../src/index.ts';

// Minimal reductions of config oracles that failed `mise run converter:oracles` (Phase 2).
const HEAD = `organization: { name: acme }
environments: { production: https://api.acme.test }
targets: { python: { package_name: acme } }
readme:
  example_requests:
    default: { type: request, endpoint: get /a, params: {} }
    headline: { type: request, endpoint: get /a, params: {} }
`;
const SPEC = JSON.stringify({
  openapi: '3.1.0',
  paths: {
    '/a': {
      get: {
        responses: {
          '200': {
            description: 'ok',
            content: {
              'application/json': { schema: { oneOf: [{ type: 'string' }, { type: 'integer' }] } },
            },
          },
        },
      },
    },
  },
  components: { schemas: { App: { type: 'object' } } },
});

const errors = (r: ReturnType<typeof convertText>) =>
  r.diagnostics.filter((d) => d.severity === 'error');

describe('config oracle regressions', () => {
  it('opencode: a model naming a missing schema is skipped with a warning, as Stainless does', () => {
    const r = convertText(
      SPEC,
      `${HEAD}resources:\n  a:\n    models: { app: App, log_level: LogLevel }\n    methods: { list: get /a }\n`,
    );
    expect(errors(r)).toEqual([]);
    expect(r.diagnostics).toContainEqual(
      expect.objectContaining({ code: 'FER1005', severity: 'warning' }),
    );
    const op = JSON.parse(r.output).paths['/a'].get;
    expect(Object.keys(op['x-ferrule-models'])).toEqual(['App']);
  });

  it('imagekit: webhook_unwrap methods have no endpoint and are accepted', () => {
    const r = convertText(
      SPEC,
      `${HEAD}resources:\n  webhooks:\n    methods:\n      unwrap: { type: webhook_unwrap, discriminator: type, webhook_key_opt: webhook_secret }\n  a:\n    methods: { list: get /a }\n`,
    );
    expect(errors(r)).toEqual([]);
  });

  it('ogx: the legacy oneOfToAnyOf transformation turns every oneOf into anyOf', () => {
    const r = convertText(
      SPEC,
      `${HEAD}openapi:\n  transformations:\n    - command: oneOfToAnyOf\n      reason: prism\nresources:\n  a:\n    methods: { list: get /a }\n`,
    );
    expect(errors(r)).toEqual([]);
    expect(r.output).not.toContain('"oneOf"');
    expect(r.output).toContain('"anyOf"');
  });
});
