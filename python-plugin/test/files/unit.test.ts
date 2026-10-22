import { describe, expect, it } from 'vitest';
import { emitApiMd, importLine } from '../../src/emit/files/api-md.ts';
import { emitApiTests, exampleValue } from '../../src/emit/files/api-tests.ts';
import { emitProjectFiles, emitStats, templateVars } from '../../src/emit/files/index.ts';
import type { FilesResource, ProjectFilesInput } from '../../src/emit/files/input.ts';
import { isortCompare, render } from '../../src/emit/files/render.ts';
import type { SdkType } from '../../src/model/index.ts';

const str: SdkType = { kind: 'primitive', name: 'string' };

const comments: FilesResource = {
  name: 'comments',
  title: 'Comments',
  variable: 'comment',
  module: 'resources.cases.comments',
  types: [],
  typesModule: 'types.cases',
  methods: [
    {
      name: 'delete',
      httpMethod: 'delete',
      path: '/cases/{case_token}/comments/{comment_token}',
      params: [
        {
          pyName: 'case_token',
          wireName: 'case_token',
          required: true,
          location: 'path',
          type: str,
        },
        {
          pyName: 'comment_token',
          wireName: 'comment_token',
          required: true,
          location: 'path',
          type: str,
          example: 'c-1',
        },
      ],
    },
  ],
  subresources: [],
};

const cases: FilesResource = {
  name: 'cases',
  title: 'Cases',
  variable: 'case',
  module: 'resources.cases.cases',
  types: ['Case', 'CaseCreateResponse'],
  typesModule: 'types',
  methods: [
    {
      name: 'create',
      httpMethod: 'post',
      path: '/cases',
      params: [
        {
          pyName: 'items',
          wireName: 'items',
          required: true,
          location: 'body',
          type: {
            kind: 'array',
            items: {
              kind: 'object',
              bases: [],
              fields: [
                {
                  wireName: 'at',
                  pyName: 'at',
                  required: true,
                  description: '',
                  type: { kind: 'primitive', name: 'datetime' },
                },
                {
                  wireName: 'on',
                  pyName: 'on',
                  required: true,
                  description: '',
                  type: { kind: 'primitive', name: 'date' },
                },
              ],
            },
          },
        },
        {
          pyName: 'count',
          wireName: 'count',
          required: false,
          location: 'query',
          type: { kind: 'primitive', name: 'integer' },
          minimum: 1,
        },
        {
          pyName: 'name',
          wireName: 'name',
          required: false,
          location: 'body',
          type: str,
          example: 'n',
        },
      ],
      paramsModule: 'types.case_create_params',
      response: { name: 'CaseCreateResponse', module: 'types.case_create_response' },
    },
  ],
  subresources: [
    comments,
    { ...comments, name: 'empty', title: 'Empty', methods: [], subresources: [] },
  ],
};

const input: ProjectFilesInput = {
  edition: 'python.2025-10-08',
  packageName: 'acme',
  distributionName: 'acme-sdk',
  clientClass: 'Acme',
  envPrefix: 'ACME',
  repo: 'acme/acme-python',
  version: '0.1.0',
  copyrightYear: 2030,
  organization: {
    name: 'acme',
    title: 'Acme',
    contact: 'sec@acme.test',
    docsUrl: 'https://docs.acme.test/',
  },
  readmeTitle: 'Acme',
  auth: { kind: 'api_key_query', env: 'ACME_API_KEY', param: 'token' },
  query: { arrayFormat: 'comma', objectFormat: 'brackets' },
  stats: {
    openapiSpecUrl: 'https://example.test/spec.yml',
    openapiSpecHash: 'abc',
    configHash: 'def',
  },
  readmeExample: {
    accessor: 'cases',
    method: 'create',
    args: 'name="n"',
    httpMethod: 'post',
    path: '/cases',
  },
  retryProbe: {
    accessor: 'cases',
    method: 'create',
    args: 'name="n"',
    httpMethod: 'post',
    path: '/cases',
  },
  sharedTypes: [],
  resources: [cases],
};

describe('render', () => {
  it('fills tokens and re-sorts marked imports', () => {
    expect(render('a %%X%% b\n%%isort%%from p import Longer, A\n', { X: '1' })).toBe(
      'a 1 b\nfrom p import A, Longer\n',
    );
  });
  it('rejects unknown tokens and bad isort lines', () => {
    expect(() => render('%%NOPE%%', {})).toThrow(/no value/);
    expect(() => render('%%isort%%import x', {})).toThrow(/one-line import/);
  });
  it('orders like ruff length-sort', () => {
    expect(['bar', 'Foo', 'FOO', 'ab', 'Bb', 'Ba'].sort(isortCompare)).toEqual([
      'Ba',
      'Bb',
      'ab',
      'FOO',
      'Foo',
      'bar',
    ]);
  });
});

describe('project files', () => {
  const files = emitProjectFiles(input);

  it('fills every template with the input', () => {
    expect(files.get('LICENSE')).toContain('Copyright 2030 Acme');
    expect(files.get('pyproject.toml')).toContain('name = "acme-sdk"');
    expect(files.get('tests/conftest.py')).toContain(
      'from acme import Acme, AsyncAcme, DefaultAioHttpClient',
    );
    expect(files.get('tests/test_client.py')).toContain(
      'client.cases.with_raw_response.create(name="n", extra_headers=',
    );
    expect(files.get('README.md')).toContain('[docs.acme.test](https://docs.acme.test/)');
    expect(files.get('scripts/mock')).toContain('--validator-query-array-format=comma');
    for (const text of files.values()) expect(text).not.toMatch(/%%|Stainless\]|by Stainless/);
    expect([...files.keys()]).toEqual([...files.keys()].sort());
  });

  it('leaves the PROBE args comma out when there are no args', () => {
    const vars = templateVars({ ...input, retryProbe: { ...input.retryProbe, args: '' } });
    const { PROBE_ARGS_COMMA, EX_RESULT } = vars;
    expect(PROBE_ARGS_COMMA).toBe('');
    expect(EX_RESULT).toBe('cases');
  });

  it('writes .stats.yml', () => {
    expect(emitStats(input)).toBe(
      'configured_endpoints: 2\nopenapi_spec_url: https://example.test/spec.yml\nopenapi_spec_hash: abc\nconfig_hash: def\n',
    );
  });
});

describe('api.md', () => {
  it('lists nested resources, keyword-only path params and None returns', () => {
    const md = emitApiMd({ ...input, sharedTypes: ['Shared'] });
    expect(md).toContain('# Shared Types\n\n```python\nfrom acme.types import Shared\n```');
    expect(md).toContain('## Comments\n\nMethods:');
    expect(md).toContain('(comment_token, \\*, case_token) -> None</code>');
    expect(md).toMatch(/## Comments\n[\s\S]*## Empty\n$/);
    expect(md).toContain('(\\*\\*<a href="src/acme/types/case_create_params.py">params</a>)');
  });
  it('wraps long imports', () => {
    expect(importLine('m', ['A', 'B'], 10)).toBe('from m import (\n    A,\n    B,\n)');
  });
});

describe('api tests', () => {
  const tests = emitApiTests(input);

  it('writes a module per resource with methods and an __init__ per package', () => {
    expect([...tests.keys()].sort()).toEqual([
      'tests/api_resources/cases/__init__.py',
      'tests/api_resources/cases/test_comments.py',
      'tests/api_resources/test_cases.py',
    ]);
  });

  it('renders examples and helper imports', () => {
    const text = tests.get('tests/api_resources/test_cases.py') ?? '';
    expect(text).toContain('from acme._utils import parse_date, parse_datetime');
    expect(text).toContain('"at": parse_datetime("2019-12-27T18:11:19.117Z"),');
    expect(text).toContain('count=1,\n            name="n",');
    expect(text).toContain('async with async_client.cases.with_streaming_response.create(');
  });

  it('checks every path parameter and None results', () => {
    const text = tests.get('tests/api_resources/cases/test_comments.py') ?? '';
    expect(text).toContain('comment_token="c-1",\n            case_token="case_token",');
    expect(text).toContain('`case_token` but received');
    expect(text).toContain('`comment_token` but received');
    expect(text).toContain('assert comment is None');
    expect(text).not.toContain('assert_matches_type');
  });

  it('wraps a long assert_matches_type call', () => {
    const long = 'X'.repeat(100);
    const r: FilesResource = {
      ...cases,
      subresources: [],
      methods: [
        {
          name: 'get',
          httpMethod: 'get',
          path: '/x',
          params: [],
          response: { name: long, module: 'types.x' },
        },
      ],
    };
    const text =
      emitApiTests({ ...input, resources: [r] }).get('tests/api_resources/test_cases.py') ?? '';
    expect(text).toContain(
      `assert_matches_type(\n            ${long}, case, path=["response"]\n        )`,
    );
  });
});

describe('exampleValue', () => {
  const ex = (type: SdkType, example?: unknown): string =>
    exampleValue(type, new Set(), '', example, 'w');
  it('covers every type', () => {
    expect(ex(str)).toBe('"w"');
    expect(ex(str, 'e')).toBe('"e"');
    expect(ex({ kind: 'primitive', name: 'number' })).toBe('0');
    expect(ex({ kind: 'primitive', name: 'boolean' })).toBe('True');
    expect(ex({ kind: 'primitive', name: 'null' })).toBe('None');
    expect(ex({ kind: 'literal', values: ['a'] })).toBe('"a"');
    expect(ex({ kind: 'literal', values: [true] })).toBe('True');
    expect(ex({ kind: 'literal', values: [false] })).toBe('False');
    expect(ex({ kind: 'literal', values: [3] })).toBe('3');
    expect(ex({ kind: 'union', variants: [] })).toBe('{}');
    expect(ex({ kind: 'union', variants: [str] })).toBe('"w"');
    expect(ex({ kind: 'array', items: str })).toBe('["string"]');
    expect(ex({ kind: 'map', values: str })).toBe('{"foo": "string"}');
    expect(ex({ kind: 'object', bases: [], fields: [] })).toBe('{}');
    expect(ex({ kind: 'unknown' })).toBe('{}');
    expect(ex({ kind: 'ref', name: 'R', module: 'r' })).toBe('{}');
  });
});
