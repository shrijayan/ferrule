import { describe, expect, it } from 'vitest';
import { relImport, sortFields, wrap } from '../src/emit/common.ts';
import { docstring, emitModelFile } from '../src/emit/models.ts';
import { emitParamsFile } from '../src/emit/params.ts';
import { emitGroupFiles } from '../src/emit/project.ts';
import { emitResourceFile } from '../src/emit/resources.ts';
import type { SdkClient, SdkMethod, SdkParam, SdkResource, SdkType } from '../src/model/index.ts';
import { classSegment, singular } from '../src/naming.ts';
import { PythonPrinter } from '../src/printer.ts';
import { toAnnotation } from '../src/type-mapper.ts';

const str: SdkType = { kind: 'primitive', name: 'string' };
const obj: SdkType = {
  kind: 'object',
  bases: [],
  fields: [{ wireName: 'x', pyName: 'x', required: true, description: '', type: str }],
};

function method(over: Partial<SdkMethod> = {}): SdkMethod {
  return {
    name: 'list',
    pointer: '#/paths/~1t/get',
    httpMethod: 'get',
    path: '/t',
    summary: '',
    description: '',
    pathParams: [],
    queryParams: [],
    body: undefined,
    response: obj,
    pagination: undefined,
    ...over,
  };
}

function client(m: SdkMethod): { c: SdkClient; r: SdkResource } {
  const r: SdkResource = { path: ['things'], methods: [m], subresources: [] };
  return {
    c: { package: { name: 'acme', repo: 'a/b' }, baseUrl: '', resources: [r], models: [] },
    r,
  };
}

const param = (over: Partial<SdkParam>): SdkParam => ({
  wireName: 'q',
  pyName: 'q',
  required: false,
  description: '',
  type: str,
  ...over,
});

describe('toAnnotation', () => {
  it('maps every SdkType kind and collects imports', () => {
    const p = new PythonPrinter();
    const ctx = { objectName: () => 'Obj', refName: () => 'Ref' };
    const t: SdkType = {
      kind: 'union',
      variants: [
        str,
        { kind: 'primitive', name: 'integer' },
        { kind: 'primitive', name: 'number' },
        { kind: 'primitive', name: 'boolean' },
        { kind: 'primitive', name: 'datetime' },
        { kind: 'primitive', name: 'date' },
        { kind: 'unknown' },
        { kind: 'array', items: { kind: 'ref', name: 'User', module: 'user' } },
        {
          kind: 'map',
          values: { kind: 'union', variants: [str, { kind: 'primitive', name: 'null' }] },
        },
        { kind: 'literal', values: ['a', 1, true, false] },
        obj,
        { kind: 'primitive', name: 'null' },
      ],
    };
    expect(toAnnotation(t, p, ctx)).toBe(
      'Optional[Union[str, int, float, bool, datetime, date, object, List[Ref], Dict[str, Optional[str]], Literal["a", 1, True, False], Obj]]',
    );
    expect(p.toString()).toBe(
      [
        'from typing import Dict, List, Union, Optional',
        'from datetime import date, datetime',
        'from typing_extensions import Literal',
        '',
      ].join('\n'),
    );
  });
});

describe('naming helpers', () => {
  it('upper-cases acronyms and singularizes', () => {
    expect(classSegment(['situation', 'url'])).toBe('SituationURL');
    expect(classSegment(['route', 'ids'])).toBe('RouteIDs');
    expect(classSegment(['https'])).toBe('HTTPS');
    expect(singular(['agencies'])).toEqual(['agency']);
    expect(singular(['status'])).toEqual(['status']);
    expect(singular(['stop', 'times'])).toEqual(['stop', 'time']);
    expect(singular(['list'])).toEqual(['list']);
    expect(singular([])).toEqual([]);
  });

  it('computes relative imports and wraps text', () => {
    expect(relImport('types.foo', '_models')).toBe('.._models');
    expect(relImport('types.shared.a', 'types.shared.b')).toBe('.b');
    expect(relImport('resources.a', 'types')).toBe('..types');
    expect(wrap('')).toEqual([]);
    expect(wrap('a b\n\nc', 3)).toEqual(['a b', '', 'c']);
  });

  it('sorts required first, id first, then by wire name', () => {
    const names = sortFields([
      param({ wireName: 'b', required: true }),
      param({ wireName: 'z' }),
      param({ wireName: 'id', required: true }),
      param({ wireName: 'A', required: true }),
      param({ wireName: 'a' }),
      param({ wireName: 'a' }),
    ]).map((f) => f.wireName);
    expect(names).toEqual(['id', 'A', 'b', 'a', 'a', 'z']);
  });
});

describe('models', () => {
  it('wraps long docstrings and prints pass for empty classes', () => {
    expect(docstring('', '')).toEqual([]);
    expect(docstring('word '.repeat(30), '    ')).toEqual([
      `    """${'word '.repeat(16).trim()}`,
      `    ${'word '.repeat(14).trim()}`,
      '    """',
    ]);
    const out = emitModelFile({
      ref: { kind: 'ref', name: 'Empty', module: 'empty' },
      type: {
        kind: 'object',
        bases: [],
        fields: [
          {
            wireName: 'items',
            pyName: 'items',
            required: true,
            description: '',
            type: { kind: 'map', values: { kind: 'object', bases: [], fields: [] } },
          },
          {
            wireName: 'same',
            pyName: 'same',
            required: false,
            description: '',
            type: { kind: 'ref', name: 'Empty', module: 'empty' },
          },
        ],
      },
      prefixChildren: true,
    });
    expect(out).toContain('class EmptyItems(BaseModel):\n    pass\n');
    expect(out).toContain('items: Dict[str, EmptyItems]\n');
    expect(out).toContain('same: Optional[Empty] = None\n');
  });
});

describe('resources', () => {
  it('awaits async_maybe_transform for a non-paginated async method with query', () => {
    const m = method({
      httpMethod: 'post',
      description: 'Does things.',
      queryParams: [
        param({ required: true, description: 'Q.' }),
        param({ wireName: 'r', pyName: 'r' }),
      ],
    });
    const { c, r } = client(m);
    const out = emitResourceFile(c, r);
    expect(out).toContain('query=await async_maybe_transform(');
    expect(out).toContain('return await self._post(');
    expect(out).toContain('        q: str,\n');
    expect(out).toContain('          q: Q.\n\n          r:\n');
  });

  it('prints a docstring-less method', () => {
    const { c, r } = client(method());
    expect(emitResourceFile(c, r)).toContain(') -> ThingListResponse:\n        return self._get(');
  });

  it.each([
    ['path parameters', method({ pathParams: [param({})] })],
    ['bodies', method({ body: str })],
    ['methods without a response', method({ response: undefined })],
    [
      'pagination over POST',
      method({
        httpMethod: 'post',
        pagination: { scheme: 'cursor', page: 'CursorPage', request: {}, response: {}, item: obj },
      }),
    ],
  ])('rejects %s', (_, m) => {
    const { c, r } = client(m);
    expect(() => emitResourceFile(c, r)).toThrow(/FER201/);
  });

  it('rejects a non-object inline response', () => {
    const { c } = client(method({ response: { kind: 'array', items: str } }));
    expect(() => emitGroupFiles(c)).toThrow(/FER201 .*a array response/);
  });

  it('skips resources without methods', () => {
    const { c, r } = client(method());
    const parent: SdkResource = {
      path: ['p'],
      methods: [],
      subresources: [{ ...r, path: ['p', 'things'] }],
    };
    const files = emitGroupFiles({ ...c, resources: [parent] });
    expect([...files.keys()]).toEqual([
      'src/acme/resources/p/things.py',
      'src/acme/types/thing_list_response.py',
    ]);
  });
});

describe('params', () => {
  it('marks required params and imports named models', () => {
    const { r } = client(method());
    const m = method({
      queryParams: [
        param({ required: true }),
        param({ wireName: 'u', pyName: 'u', type: { kind: 'ref', name: 'U', module: 'shared.u' } }),
      ],
    });
    const out = emitParamsFile(r, m);
    expect(out).toContain('q: Required[str]');
    expect(out).toContain('from .shared.u import U');
  });

  it.each([
    ['aliased', param({ wireName: 'qQ' })],
    ['object-typed', param({ type: obj })],
  ])('rejects %s query params', (_, q) => {
    const { r } = client(method());
    expect(() => emitParamsFile(r, method({ queryParams: [q] }))).toThrow(/FER201/);
  });
});
