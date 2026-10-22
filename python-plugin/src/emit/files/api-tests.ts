/**
 * tests/api_resources/**: one test module per resource with methods, sync and async from one
 * pass (`Mode`), in the shape of the oracle SDKs' generated tests.
 */
import type { SdkPrimitive, SdkType } from '../../model/index.ts';
import { HEADER } from '../common.ts';
import { importLine } from './api-md.ts';
import type { FilesMethod, FilesParam, FilesResource, ProjectFilesInput } from './input.ts';
import { isortCompare } from './render.ts';

type Mode = 'sync' | 'async';

/** Helpers a test module imports from `<pkg>._utils` because an example value uses them. */
type Uses = Set<'parse_date' | 'parse_datetime'>;

function pyString(s: string): string {
  return JSON.stringify(s);
}

function primitiveExample(
  name: SdkPrimitive,
  uses: Uses,
  example: unknown,
  fallback: string,
  bound: number | undefined,
): string {
  switch (name) {
    case 'string':
      return pyString(typeof example === 'string' ? example : fallback);
    case 'integer':
    case 'number':
      return String(bound ?? 0);
    case 'boolean':
      return 'True';
    case 'null':
      return 'None';
    case 'datetime':
      uses.add('parse_datetime');
      return 'parse_datetime("2019-12-27T18:11:19.117Z")';
    case 'date':
      uses.add('parse_date');
      return 'parse_date("2019-12-27")';
  }
}

/** A Python literal for an example of `type`, laid out the way ruff formats the oracle tests. */
export function exampleValue(
  type: SdkType,
  uses: Uses,
  indent: string,
  example: unknown,
  fallback: string,
  bound?: number,
): string {
  switch (type.kind) {
    case 'primitive':
      return primitiveExample(type.name, uses, example, fallback, bound);
    case 'literal': {
      const v = type.values[0];
      return typeof v === 'string'
        ? pyString(v)
        : v === true
          ? 'True'
          : v === false
            ? 'False'
            : String(v);
    }
    case 'union':
      return type.variants[0] === undefined
        ? '{}'
        : exampleValue(type.variants[0], uses, indent, example, fallback);
    case 'array': {
      const item = exampleValue(type.items, uses, `${indent}    `, undefined, 'string');
      return item.includes('\n') ? `[\n${indent}    ${item}\n${indent}]` : `[${item}]`;
    }
    case 'map':
      return `{"foo": ${exampleValue(type.values, uses, indent, undefined, 'string')}}`;
    case 'object': {
      if (type.fields.length === 0) return '{}';
      const inner = `${indent}    `;
      const lines = type.fields.map(
        (f) =>
          `${inner}${pyString(f.wireName)}: ${exampleValue(f.type, uses, inner, undefined, 'string')},`,
      );
      return `{\n${lines.join('\n')}\n${indent}}`;
    }
    case 'unknown':
    case 'ref':
      return '{}';
  }
}

/** A parameter's example; query parameters ignore the spec's `example`, as in the oracles. */
function exampleOf(p: FilesParam, uses: Uses, indent: string): string {
  const example = p.location === 'query' ? undefined : p.example;
  return exampleValue(p.type, uses, indent, example, p.wireName, p.minimum ?? p.maximum);
}

/** Call arguments: the last path parameter first (it is positional), then path order, then the rest. */
function callOrder(params: readonly FilesParam[]): FilesParam[] {
  const path = params.filter((p) => p.location === 'path');
  const rest = params.filter((p) => p.location !== 'path');
  const byName = (a: FilesParam, b: FilesParam): number => (a.pyName < b.pyName ? -1 : 1);
  const required = rest.filter((p) => p.required).sort(byName);
  const optional = rest.filter((p) => !p.required).sort(byName);
  const ordered =
    path.length > 0 ? [path[path.length - 1] as FilesParam, ...path.slice(0, -1)] : [];
  return [...ordered, ...required, ...optional];
}

function args(
  params: readonly FilesParam[],
  indent: string,
  uses: Uses,
  positional: boolean,
  empty?: FilesParam,
): string {
  if (params.length === 0) return '()';
  const inner = `${indent}    `;
  const only = params[0] as FilesParam;
  if (params.length === 1 && only.location === 'path' && positional) {
    const value = only === empty ? '""' : exampleOf(only, uses, inner);
    return `(\n${inner}${value},\n${indent})`;
  }
  const lines = params.map((p) => {
    const value = p === empty ? '""' : exampleOf(p, uses, inner);
    return `${inner}${p.pyName}=${value},`;
  });
  return `(\n${lines.join('\n')}\n${indent})`;
}

interface Ctx {
  readonly client: string;
  readonly accessor: string;
  readonly variable: string;
  readonly uses: Uses;
}

/** The result check at `indent`, split the way ruff splits a call past 120 columns. */
function assertion(m: FilesMethod, variable: string, indent: string): string {
  if (m.response === undefined) return `${indent}assert ${variable} is None`;
  const callArgs = `${m.response.name}, ${variable}, path=["response"]`;
  const line = `${indent}assert_matches_type(${callArgs})`;
  return line.length <= 120
    ? line
    : `${indent}assert_matches_type(\n${indent}    ${callArgs}\n${indent})`;
}

function methodTests(ctx: Ctx, m: FilesMethod, mode: Mode): string[] {
  const a = mode === 'async';
  const def = a ? 'async def' : 'def';
  const aw = a ? 'await ' : '';
  const cl = a ? 'async_client' : 'client';
  const ty = a ? `Async${ctx.client}` : ctx.client;
  const sig = (name: string): string =>
    `    @parametrize\n    ${def} ${name}(self, ${cl}: ${ty}) -> None:`;
  const target = `${cl}.${ctx.accessor}`;
  const required = callOrder(m.params.filter((p) => p.required || p.location === 'path'));
  const all = callOrder(m.params);
  const v = ctx.variable;
  // A method whose only parameter is one path parameter is called with it positionally.
  const pos = m.params.length === 1;
  const tests: string[] = [];

  tests.push(
    `${sig(`test_method_${m.name}`)}\n        ${v} = ${aw}${target}.${m.name}${args(required, '        ', ctx.uses, pos)}\n${assertion(m, v, '        ')}`,
  );
  if (all.length > required.length) {
    tests.push(
      `${sig(`test_method_${m.name}_with_all_params`)}\n        ${v} = ${aw}${target}.${m.name}${args(all, '        ', ctx.uses, pos)}\n${assertion(m, v, '        ')}`,
    );
  }
  tests.push(
    `${sig(`test_raw_response_${m.name}`)}\n        response = ${aw}${target}.with_raw_response.${m.name}${args(required, '        ', ctx.uses, pos)}\n\n` +
      `        assert response.is_closed is True\n        assert response.http_request.headers.get("X-Stainless-Lang") == "python"\n` +
      `        ${v} = ${aw}response.parse()\n${assertion(m, v, '        ')}`,
  );
  tests.push(
    `${sig(`test_streaming_response_${m.name}`)}\n        ${a ? 'async with' : 'with'} ${target}.with_streaming_response.${m.name}${args(required, '        ', ctx.uses, pos)} as response:\n` +
      `            assert not response.is_closed\n            assert response.http_request.headers.get("X-Stainless-Lang") == "python"\n\n` +
      `            ${v} = ${aw}response.parse()\n${assertion(m, v, '            ')}\n\n` +
      '        assert cast(Any, response.is_closed) is True',
  );
  const path = m.params.filter((p) => p.location === 'path');
  if (path.length > 0) {
    const blocks = path.map(
      (p) =>
        `        with pytest.raises(ValueError, match=r"Expected a non-empty value for \`${p.pyName}\` but received ''"):\n` +
        `            ${aw}${target}.with_raw_response.${m.name}${args(required, '            ', ctx.uses, pos, p)}`,
    );
    tests.push(`${sig(`test_path_params_${m.name}`)}\n${blocks.join('\n\n')}`);
  }
  return tests;
}

function testClass(ctx: Ctx, r: FilesResource, mode: Mode): string {
  const head =
    mode === 'sync'
      ? `class Test${r.title}:\n    parametrize = pytest.mark.parametrize("client", [False, True], indirect=True, ids=["loose", "strict"])`
      : `class TestAsync${r.title}:\n    parametrize = pytest.mark.parametrize(\n        "async_client", [False, True, {"http_client": "aiohttp"}], indirect=True, ids=["loose", "strict", "aiohttp"]\n    )`;
  const tests = r.methods.flatMap((m) => methodTests(ctx, m, mode));
  return [head, ...tests].join('\n\n');
}

function parentModule(module: string): string {
  return module.split('.').slice(0, -1).join('.');
}

export function emitTestModule(
  input: ProjectFilesInput,
  r: FilesResource,
  accessor: string,
): string {
  const pkg = input.packageName;
  const uses: Uses = new Set();
  const ctx: Ctx = { client: input.clientClass, accessor, variable: r.variable, uses };
  const body = `${testClass(ctx, r, 'sync')}\n\n\n${testClass(ctx, r, 'async')}\n`;

  const imports = new Map<string, Set<string>>();
  const add = (module: string, name: string): void => {
    const set = imports.get(module) ?? new Set<string>();
    set.add(name);
    imports.set(module, set);
  };
  add(pkg, input.clientClass);
  add(pkg, `Async${input.clientClass}`);
  for (const m of r.methods) {
    if (m.response === undefined) continue;
    add('tests.utils', 'assert_matches_type');
    add(`${pkg}.${parentModule(m.response.module)}`, m.response.name);
  }
  for (const u of uses) add(`${pkg}._utils`, u);
  const lines = [...imports.keys()]
    .sort(isortCompare)
    .map((mod) => importLine(mod, [...(imports.get(mod) as Set<string>)].sort(isortCompare), 88));

  return [
    HEADER,
    '',
    'from __future__ import annotations',
    '',
    'import os',
    'from typing import Any, cast',
    '',
    'import pytest',
    '',
    ...lines,
    '',
    'base_url = os.environ.get("TEST_API_BASE_URL", "http://127.0.0.1:4010")',
    '',
    '',
    body,
  ].join('\n');
}

/** Every test module and package `__init__.py` under tests/api_resources/. */
export function emitApiTests(input: ProjectFilesInput): Map<string, string> {
  const files = new Map<string, string>();
  const walk = (resources: readonly FilesResource[], dir: string[], parent: string): void => {
    for (const r of resources) {
      const accessor = parent === '' ? r.name : `${parent}.${r.name}`;
      if (r.methods.length > 0) {
        files.set(
          ['tests/api_resources', ...dir, `test_${r.name}.py`].join('/'),
          emitTestModule(input, r, accessor),
        );
      }
      if (r.subresources.length > 0) {
        files.set(['tests/api_resources', ...dir, r.name, '__init__.py'].join('/'), `${HEADER}\n`);
        walk(r.subresources, [...dir, r.name], accessor);
      }
    }
  };
  walk(input.resources, [], '');
  return files;
}
