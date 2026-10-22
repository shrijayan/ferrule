import { describe, expect, it } from 'vitest';
import { closest } from '../../src/schema/suggest.ts';
import { CONFIG_CODES, validateConfig, validateConfigValue } from '../../src/schema/validate.ts';

const BASE = `organization:
  name: acme
environments:
  production: https://api.acme.test
resources: {}
readme:
  example_requests:
    default: { type: request, endpoint: get /x, params: {} }
    headline: { type: request, endpoint: get /x, params: {} }
`;

const brief = (text: string) =>
  validateConfig(text).diagnostics.map((d) => [d.code, d.severity, d.pointer, d.message]);

describe('validateConfig', () => {
  it('accepts a minimal config and returns the typed config', () => {
    const r = validateConfig(BASE);
    expect(r.diagnostics).toEqual([]);
    expect(r.config?.organization?.name).toBe('acme');
    expect(r.value).toMatchObject({ environments: { production: 'https://api.acme.test' } });
  });

  it('reports YAML syntax errors as FER1100 with line and column', () => {
    const r = validateConfig('organization:\n  name: [unclosed\n');
    expect(r.config).toBeUndefined();
    expect(r.value).toBeUndefined();
    expect(r.diagnostics[0]).toMatchObject({ code: 'FER1100', severity: 'error', pointer: '' });
    expect(r.diagnostics[0]?.message).toMatch(/line \d+, column \d+/);
  });

  it('reports duplicate keys as FER1100', () => {
    expect(brief(`${BASE}readme: {}\n`)[0]?.[0]).toBe('FER1100');
  });

  it('reports missing required keys as FER1111 at the parent, with a hint', () => {
    const r = validateConfig('organization:\n  name: acme\nresources: {}\n');
    expect(r.diagnostics.map((d) => [d.code, d.pointer, d.message])).toEqual([
      ['FER1111', '', 'config: missing required key "environments" (line 1, column 1)'],
      ['FER1111', '', 'config: missing required key "readme" (line 1, column 1)'],
    ]);
    expect(r.diagnostics[0]?.suggestion).toMatch(/^Add `environments` \(Map of the name/);
  });

  it('treats an empty document as an empty config', () => {
    expect(brief('').map((d) => d[0])).toEqual(['FER1111', 'FER1111', 'FER1111', 'FER1111']);
  });

  it('reports type mismatches as FER1110 at the value position', () => {
    const r = validateConfig(BASE.replace('name: acme', 'name: 42'));
    expect(r.diagnostics).toEqual([
      {
        code: 'FER1110',
        severity: 'error',
        message: 'organization.name: expected string, got integer (line 2, column 9)',
        pointer: '/organization/name',
        suggestion: 'Change the value to a string.',
      },
    ]);
  });

  it('reports a non-mapping root as FER1110', () => {
    expect(brief('- a\n- b\n')).toEqual([
      ['FER1110', 'error', '', 'config: expected object, got array (line 1, column 1)'],
    ]);
  });

  it('reports enum misses as FER1113 with a did-you-mean', () => {
    const r = validateConfig(`${BASE}query_settings:\n  array_format: repeet\n`);
    expect(r.diagnostics).toHaveLength(1);
    expect(r.diagnostics[0]).toMatchObject({
      code: CONFIG_CODES.notAllowed,
      pointer: '/query_settings/array_format',
      suggestion: 'Did you mean "repeat"?',
    });
    expect(r.diagnostics[0]?.message).toContain('(line 11, column 17)');
  });

  it('lists the allowed values when no enum value is close', () => {
    const r = validateConfig(`${BASE}query_settings:\n  array_format: zzzzzzzzzzzz\n`);
    expect(r.diagnostics[0]?.suggestion).toMatch(/^Use one of "comma", /);
  });

  it('reports string length and pattern failures as FER1114', () => {
    const r = validateConfig(
      `${BASE}targets:\n  python:\n    production_repo: not a repo\n  java:\n    reverse_domain: Org.Acme\n`,
    );
    const codes = r.diagnostics.map((d) => [d.code, d.pointer]);
    expect(codes).toContainEqual(['FER1114', '/targets/java/reverse_domain']);
    expect(brief(BASE.replace('name: acme', "name: ''"))[0]?.slice(0, 3)).toEqual([
      'FER1114',
      'error',
      '/organization/name',
    ]);
  });

  it('reports numeric range failures as FER1115', () => {
    const r = validateConfig(
      `${BASE}client_settings:\n  default_retries:\n    max_retries: -1\n    initial_delay_seconds: 1\n    max_delay_seconds: 2\n`,
    );
    expect(r.diagnostics.map((d) => [d.code, d.pointer, d.suggestion])).toEqual([
      [
        'FER1115',
        '/client_settings/default_retries/max_retries',
        'Use a number inside the allowed range.',
      ],
    ]);
  });

  it('reports bad environment names as FER1117 at the key', () => {
    const long = 'e'.repeat(257);
    const r = validateConfig(BASE.replace('production:', `${long}:`));
    expect(r.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1117', `/environments/${long}`],
    ]);
    expect(r.diagnostics[0]?.message).toContain('(line 4, column 3)');
  });

  it('reports unknown keys under additionalProperties: false as FER1112', () => {
    const tf = (body: string) =>
      `${BASE}targets:\n  terraform:\n    options:\n      data_source:\n${body}`;
    const r = validateConfig(
      tf('        filtering: {}\n        response_items: {}\n        filterng: {}\n'),
    );
    expect(r.diagnostics.filter((d) => d.code === 'FER1112')).toEqual([
      {
        code: 'FER1112',
        severity: 'error',
        message:
          'targets.terraform.options.data_source: unknown key "filterng" (line 16, column 9)',
        pointer: '/targets/terraform/options/data_source/filterng',
        suggestion: 'Did you mean `filtering`?',
      },
    ]);
    const far = validateConfig(
      tf('        filtering: {}\n        response_items: {}\n        zzzzzzzzzz: {}\n'),
    );
    expect(far.diagnostics.find((d) => d.code === 'FER1112')?.suggestion).toBe(
      'Remove `zzzzzzzzzz`; allowed keys are `filtering`, `response_items`.',
    );
  });

  it('warns about deprecated keys with FER1118 and still returns the config', () => {
    const r = validateConfig(`${BASE}settings:\n  disable_tests: true\n`);
    expect(r.diagnostics).toEqual([
      {
        code: 'FER1118',
        severity: 'warning',
        message: 'settings.disable_tests: "disable_tests" is deprecated (line 11, column 3)',
        pointer: '/settings/disable_tests',
        suggestion: 'Rename `disable_tests` to `disable_mock_tests`.',
      },
    ]);
    expect(r.config).toBeDefined();
  });

  it('picks the intended anyOf branch for method objects', () => {
    const r = validateConfig(
      BASE.replace(
        'resources: {}',
        'resources:\n  users:\n    methods:\n      list:\n        paginated: false\n',
      ),
    );
    expect(r.diagnostics.map((d) => [d.code, d.pointer, d.message])).toEqual([
      [
        'FER1111',
        '/resources/users/methods/list',
        'resources.users.methods.list: missing required key "endpoint" (line 9, column 9)',
      ],
    ]);
  });

  it('reports values that match no alternative as FER1116', () => {
    const r = validateConfig(
      BASE.replace('resources: {}', 'resources:\n  users:\n    methods:\n      list: 3\n'),
    );
    expect(r.diagnostics).toHaveLength(1);
    expect(r.diagnostics[0]).toMatchObject({
      code: 'FER1116',
      pointer: '/resources/users/methods/list',
    });
    expect(r.diagnostics[0]?.message).toMatch(/expected a string or .*an object.*, got 3/);
  });

  it('follows YAML aliases when locating errors', () => {
    const r = validateConfig(
      BASE.replace('organization:', 'organization: &org').replace('name: acme', 'name: 1') +
        'settings:\n  x: *org\n',
    );
    expect(r.diagnostics.map((d) => d.pointer)).toEqual(['/organization/name']);
  });

  it('locates errors inside sequences', () => {
    const r = validateConfig(`${BASE}pagination:\n  - name: p\n    type: 1\n`);
    const d = r.diagnostics.find((x) => x.pointer === '/pagination/0/type');
    expect(d?.message).toContain('(line 12, column 11)');
  });

  it('reports shapes the typed reader rejects as FER1119', () => {
    // A webhook_unwrap method (no `endpoint`) is schema-valid but not readable by MethodSchema yet.
    const r = validateConfig(
      BASE.replace(
        'resources: {}',
        'resources: { a: { methods: { hook: { type: webhook_unwrap } } } }',
      ),
    );
    expect(r.diagnostics.map((d) => [d.code, d.severity])).toEqual([['FER1119', 'error']]);
    expect(r.diagnostics[0]?.pointer).toBe('/resources/a/methods/hook');
    expect(r.config).toBeUndefined();
  });
});

describe('validateConfigValue', () => {
  it('validates parsed values without positions', () => {
    expect(validateConfigValue({ organization: { name: 'a' } }).map((d) => d.message)).toEqual([
      'config: missing required key "environments"',
      'config: missing required key "resources"',
      'config: missing required key "readme"',
    ]);
  });
});

describe('closest', () => {
  it('returns a near candidate or undefined', () => {
    expect(closest('resorces', ['resources', 'readme'])).toBe('resources');
    expect(closest('qqqqqq', ['resources'])).toBeUndefined();
    expect(closest('', [])).toBeUndefined();
  });
});
