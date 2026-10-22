import { describe, expect, it } from 'vitest';
import { JsonPathError, parseJsonPath, query } from '../../src/transforms/jsonpath.ts';

const doc = {
  info: { title: 'T', 'x.dotted': 1 },
  paths: {
    '/users/{id}': {
      get: {
        parameters: [
          { name: 'id', in: 'path', schema: { type: 'integer' } },
          { name: 'limit', in: 'query', deprecated: true, schema: { type: 'integer' } },
          { name: 'Cursor', in: 'query', required: false },
        ],
        responses: { '404': { content: { 'application/json': { schema: { type: 'object' } } } } },
      },
    },
  },
  list: [10, 20, 30, 40, 50],
};

const paths = (p: string) => query(doc, p).map((m) => m.path);
const values = (p: string) => query(doc, p).map((m) => m.value);

describe('jsonpath', () => {
  it('selects the root', () => {
    expect(query(doc, '$')).toEqual([{ path: [], value: doc }]);
  });

  it('selects dot names that contain slashes, braces and digits', () => {
    expect(
      values("$.paths./users/{id}.get.responses.404.content['application/json'].schema.type"),
    ).toEqual(['object']);
    expect(
      values('$.paths./users/{id}.get.responses.404.content.application/json.schema.type'),
    ).toEqual(['object']);
  });

  it('selects bracketed names, quotes and escapes', () => {
    expect(values("$.info['x.dotted']")).toEqual([1]);
    expect(values('$.info["title"]')).toEqual(['T']);
    expect(values("$['info']['ti\\u0074le']")).toEqual(['T']);
    expect(values('$[info][title]')).toEqual(['T']);
    expect(values("$.info['a\\'b']")).toEqual([]);
    expect(values("$.info['\\n\\t']")).toEqual([]);
  });

  it('selects indices, negative indices, unions, wildcards and slices', () => {
    expect(values('$.list[0]')).toEqual([10]);
    expect(values('$.list.1')).toEqual([20]);
    expect(values('$.list[-1]')).toEqual([50]);
    expect(values('$.list[9]')).toEqual([]);
    expect(values('$.list.x')).toEqual([]);
    expect(values('$.list[0,2]')).toEqual([10, 30]);
    expect(values('$.list[1:3]')).toEqual([20, 30]);
    expect(values('$.list[::2]')).toEqual([10, 30, 50]);
    expect(values('$.list[-2:]')).toEqual([40, 50]);
    expect(values('$.list[::-1]')).toEqual([50, 40, 30, 20, 10]);
    expect(values('$.list[3:0:-2]')).toEqual([40, 20]);
    expect(values('$.info[*]')).toEqual(['T', 1]);
    expect(values('$.info.*')).toEqual(['T', 1]);
    expect(values('$.info[0:1]')).toEqual([]);
    expect(values("$.info['title','missing']")).toEqual(['T']);
    expect(values('$.info.title[0]')).toEqual([]);
  });

  it('recursive descent visits every level, deduplicated', () => {
    expect(values('$..type')).toEqual(['integer', 'integer', 'object']);
    expect(paths('$..parameters[*].name')).toHaveLength(3);
    expect(query(doc, '$..*').length).toBeGreaterThan(20);
    expect(values('$..["title"]')).toEqual(['T']);
    // `$.info..*` and `$.info.*` overlap; results are unique.
    expect(paths("$.info['title','title']")).toEqual([['info', 'title']]);
  });

  it('filters with comparisons and boolean logic', () => {
    expect(values('$..parameters[?(@.name == "id")].in')).toEqual(['path']);
    expect(values("$..parameters[?(@.in === 'query' && !@.deprecated)].name")).toEqual(['Cursor']);
    expect(values('$..parameters[?(@.deprecated == true || @.name != "limit")].name')).toEqual([
      'id',
      'limit',
      'Cursor',
    ]);
    expect(values('$..parameters[?(@.required === false)].name')).toEqual(['Cursor']);
    expect(values('$..parameters[?(@.required !== undefined)].name')).toEqual(['Cursor']);
    expect(values('$..parameters[?(@.deprecated)].name')).toEqual(['limit']);
    expect(values('$..parameters[?@.name=="id"].in')).toEqual(['path']);
    expect(values('$.list[?(@ > 20 && @ <= 40)]')).toEqual([30, 40]);
    expect(values('$.list[?(@ >= 40)]')).toEqual([40, 50]);
    expect(values('$.list[?(@ < 20)]')).toEqual([10]);
    expect(values('$.list[?(@ == "10")]')).toEqual([10]);
    expect(values('$.list[?(@ < "z")]')).toEqual([]);
    expect(values('$.list[?(@ === -1.5e0)]')).toEqual([]);
    expect(values('$.list[?((@ == 10) || (@ == 50))]')).toEqual([10, 50]);
    expect(values('$.list[?(@ == null)]')).toEqual([]);
    expect(values('$.list[?(null == undefined)]')).toHaveLength(5);
    expect(values('$.list[?(@ == {})]'.replace('{}', 'false'))).toEqual([]);
  });

  it('filters over object values and exposes @property, @parent and @root', () => {
    expect(paths('$.paths[?(@property.startsWith("/users"))]')).toEqual([['paths', '/users/{id}']]);
    expect(values('$.info[?(@property === "title")]')).toEqual(['T']);
    expect(
      values('$.paths.*.get.parameters[?(@parentProperty === "parameters")].name'),
    ).toHaveLength(3);
    expect(values('$.paths.*.get.parameters[?(@parent.length == 3)].name')).toHaveLength(3);
    expect(values('$.list[?(@root.info.title == "T" && $.list[0] == 10)]')).toHaveLength(5);
    expect(values("$.info[?(@root['info'].title == 'T')]")).toEqual(['T', 1]);
  });

  it('supports string, array and regex methods', () => {
    expect(values('$..parameters[?(@.name.match(/^cur/i))].name')).toEqual(['Cursor']);
    expect(values('$..parameters[?(@.name.match("^l"))].name')).toEqual(['limit']);
    expect(values('$..parameters[?(/[a-z]+/.test(@.in))].name')).toHaveLength(3);
    expect(values('$..parameters[?(@.name.toLowerCase() == "cursor")].name')).toEqual(['Cursor']);
    expect(values('$..parameters[?(@.name.toUpperCase() == "ID")].name')).toEqual(['id']);
    expect(values('$..parameters[?(@.name.trim().endsWith("it"))].name')).toEqual(['limit']);
    expect(values('$..parameters[?(@.name.includes("mi"))].name')).toEqual(['limit']);
    expect(values('$.paths.*.get[?(@property == "parameters" && @.length == 3)]')).toHaveLength(1);
    expect(values('$[?(@.includes(30))]')).toEqual([doc.list]);
    expect(values('$[?(@.includes)]')).toEqual([]);
    // Methods on the wrong receiver or argument type are simply falsy.
    expect(values('$..parameters[?(@.schema.startsWith("x"))]')).toEqual([]);
    expect(values('$..parameters[?(@.name.startsWith(1) || @.name.endsWith(1))]')).toEqual([]);
    expect(values('$..parameters[?(@.name.includes(1) || @.name.match(1))]')).toEqual([]);
    expect(values('$..parameters[?(/x/.test(1) || /x/.trim())]')).toEqual([]);
    expect(values('$.list[?([1][0])]'.replace('[1][0]', '@.length'))).toEqual([]);
    expect(values('$..parameters[?(@.name[0] == "C" || @.name[-1] == "d")].name')).toEqual([
      'id',
      'Cursor',
    ]);
    expect(values('$..parameters[?(@.name["x"] == 1)]')).toEqual([]);
  });

  it.each([
    'info',
    '$.',
    '$.info.',
    '$..',
    '$x',
    '$.info^',
    '$.info~',
    '$.@string()',
    '$[(@.length-1)]',
    '$[',
    "$['open",
    '$[?(@.a ==)]',
    '$[?(@.a.unknown())]',
    '$[?(@foo)]',
    '$[?(/(/.test(@))]',
    '$[?(/abc)]',
    '$[::0]',
    '$[:x]',
    '$[?(@.a[x])]',
    '$[?(@.)]',
    "$['\\u12']",
    '$[?(@.a.includes(1, 2)]',
    '$[@]',
  ])('rejects %s', (p) => {
    expect(() => parseJsonPath(p)).toThrow(JsonPathError);
  });

  it('accepts whitespace and multi-argument calls', () => {
    expect(values(' $ .info [ "title" ] ')).toEqual(['T']);
    expect(values('$.list[?( @ == 10 )]')).toEqual([10]);
    expect(values('$..parameters[?(@.name.includes("i", 0))].name')).toEqual(['id', 'limit']);
    expect(values('$.list[?(/[/]/.test("a/b") && /\\d/.test("1"))]')).toHaveLength(5);
  });
});
