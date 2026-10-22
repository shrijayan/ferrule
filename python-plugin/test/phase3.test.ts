/** Unit tests of the Phase 3 emitters: client, bodies, pagination, streaming, unions. */
import type { Json } from '@cloudflare/forge/plugin';
import { describe, expect, it } from 'vitest';
import { readClient, readMethodExtensions } from '../src/adapter/extensions.ts';
import { FerruleError } from '../src/diagnostics.ts';
import { emitClientFile } from '../src/emit/client.ts';
import { argsDocstring, stainlessMethodOrder } from '../src/emit/common.ts';
import { PLACEHOLDER_META, toProjectFilesInput } from '../src/emit/files-input.ts';
import { call, formatExpr } from '../src/emit/format.ts';
import { emitAliasFile, emitModelFile } from '../src/emit/models.ts';
import { emitParamsFile } from '../src/emit/params.ts';
import { emitRootFiles } from '../src/emit/project.ts';
import { emitResourceFile } from '../src/emit/resources.ts';
import type {
  SdkClient,
  SdkClientSettings,
  SdkMethod,
  SdkObject,
  SdkParam,
  SdkResource,
  SdkType,
} from '../src/model/index.ts';
import { PythonPrinter } from '../src/printer.ts';
import { toAnnotation } from '../src/type-mapper.ts';

const str: SdkType = { kind: 'primitive', name: 'string' };
const int: SdkType = { kind: 'primitive', name: 'integer' };
const obj = (over: Partial<SdkObject> = {}): SdkObject => ({
  kind: 'object',
  bases: [],
  fields: [{ wireName: 'id', pyName: 'id', required: true, description: '', type: str }],
  ...over,
});

const param = (over: Partial<SdkParam>): SdkParam => ({
  wireName: 'q',
  pyName: 'q',
  required: false,
  description: '',
  type: str,
  ...over,
});

function method(over: Partial<SdkMethod> = {}): SdkMethod {
  return {
    name: 'list',
    pointer: '#/paths/~1t/get',
    httpMethod: 'get',
    path: '/t',
    summary: '',
    description: '',
    order: 0,
    deprecated: false,
    pathParams: [],
    queryParams: [],
    headerParams: [],
    body: undefined,
    bodyParams: [],
    bodyMediaType: undefined,
    response: obj(),
    pagination: undefined,
    streaming: undefined,
    ...over,
  };
}

const settings = (over: Partial<SdkClientSettings> = {}): SdkClientSettings => ({
  name: 'acme',
  className: 'Acme',
  envPrefix: 'ACME',
  opts: [
    {
      name: 'api_key',
      type: 'string',
      nullable: false,
      default: undefined,
      readEnv: 'ACME_API_KEY',
      description: undefined,
      auth: { securityScheme: 'Bearer', role: 'value' },
      send: undefined,
    },
  ],
  environments: [{ name: 'production', url: 'https://api.acme.test' }],
  securitySchemes: { Bearer: { type: 'http', scheme: 'bearer' } },
  security: [['Bearer']],
  timeoutSeconds: 60,
  maxRetries: 2,
  idempotencyHeader: undefined,
  defaultHeaders: {},
  arrayFormat: 'comma',
  nestedFormat: 'brackets',
  ...over,
});

function sdk(
  methods: SdkMethod[],
  over: Partial<SdkClient> = {},
): { c: SdkClient; r: SdkResource } {
  const r: SdkResource = { path: ['things'], order: 0, methods, subresources: [] };
  return {
    c: {
      package: { name: 'acme', repo: 'acme/acme-python' },
      baseUrl: '',
      settings: settings(),
      resources: [r],
      clientMethods: [],
      clientClass: undefined,
      envPrefix: undefined,
      models: [],
      ...over,
    },
    r,
  };
}

function code(f: () => unknown): string {
  try {
    f();
  } catch (e) {
    if (e instanceof FerruleError) return e.code;
    throw e;
  }
  return 'none';
}

const CLIENT: Record<string, Json> = {
  name: 'acme-sdk',
  envPrefix: 'ACME_SDK',
  opts: [
    {
      name: 'api_key',
      type: 'string',
      nullable: false,
      readEnv: 'ACME_API_KEY',
      auth: { securityScheme: 'K', role: 'value' },
      send: { in: 'query', name: 'key' },
    },
  ],
  environments: [{ name: 'production', url: 'https://x.test' }],
  securitySchemes: {
    K: { type: 'apiKey', in: 'query', name: 'key' },
    B: { type: 'http', scheme: 'bearer', keyword: 'Token' },
    D: { type: 'http', scheme: 'digest' },
    O: { type: 'oauth2', flows: {} },
  },
  security: [{ K: [] }],
  timeoutSeconds: 60,
  retries: { maxRetries: 3 },
  query: { arrayFormat: 'comma', nestedFormat: 'dots' },
  defaultHeaders: { 'X-A': 'b' },
};

describe('x-ferrule-client', () => {
  const fail = (): FerruleError => new FerruleError('FER101', '#', 'bad', 'fix');

  it('reads every member', () => {
    const c = readClient(CLIENT, fail);
    expect(c.maxRetries).toBe(3);
    expect(c.arrayFormat).toBe('comma');
    expect(c.nestedFormat).toBe('dots');
    expect(Object.values(c.securitySchemes)[1]).toEqual({
      type: 'http',
      scheme: 'bearer',
      keyword: 'Token',
    });
    expect(c.security).toEqual([['K']]);
    expect(c.opts[0]?.send).toEqual({ in: 'query', name: 'key' });
    const d = readClient(
      { ...CLIENT, retries: {}, query: {}, defaultHeaders: undefined } as never,
      fail,
    );
    expect([d.maxRetries, d.arrayFormat, d.nestedFormat]).toEqual([2, 'repeat', 'brackets']);
  });

  it.each([
    ['name', 1],
    ['environments', [{ name: 1 }]],
    ['security', ['K']],
    ['retries', { maxRetries: 'x' }],
    ['query', { arrayFormat: 'pipes' }],
    ['query', { nestedFormat: 'pipes' }],
    ['opts', [{ name: 'a-b', type: 'string', nullable: false }]],
    ['opts', [{ name: 'a', type: 'string', nullable: false, auth: { role: 'x' } }]],
    ['opts', [{ name: 'a', type: 'string', nullable: false, send: { in: 'cookie' } }]],
    ['securitySchemes', { X: { type: 'http', scheme: 'negotiate' } }],
    ['securitySchemes', { X: { type: 'http', scheme: 'bearer', keyword: 1 } }],
    ['securitySchemes', { X: { type: 'apiKey', in: 'body', name: 'k' } }],
    ['securitySchemes', { X: { type: 'mutualTLS' } }],
  ])('rejects a bad %s', (key, value) => {
    expect(code(() => readClient({ ...CLIENT, [key]: value as Json }, fail))).toBe('FER101');
  });

  it('is read from operations with order, streaming and pagination', () => {
    const e = readMethodExtensions(
      [
        ['x-ferrule-resource', { path: ['trip-details', 'sub'], order: [3, 1] }],
        ['x-ferrule-method-name', 'retrieve'],
        ['x-ferrule-method-order', 2],
        ['x-ferrule-package', { name: 'acme', repo: '' }],
        ['x-ferrule-client', CLIENT],
        ['x-ferrule-streaming', { format: 'jsonl', paramDiscriminator: null }],
        [
          'x-ferrule-pagination',
          {
            scheme: 'offset',
            page: 'offset_page',
            paramLocation: 'body',
            request: { offset: 'offset_count_param' },
            response: { data: 'items' },
            headers: { 'X-Total': 'total' },
          },
        ],
      ],
      '#/p',
    );
    expect(e.resourceOrder).toEqual([3, 1]);
    expect(e.methodOrder).toBe(2);
    expect(e.streaming).toEqual({
      format: 'jsonl',
      paramDiscriminator: null,
      eventSchema: undefined,
    });
    expect(e.pagination?.paramLocation).toBe('body');
    expect(e.client?.name).toBe('acme-sdk');
  });

  it.each([
    ['x-ferrule-resource', { path: ['a'], order: [1, 2] }],
    ['x-ferrule-method-order', -1],
    ['x-ferrule-streaming', { format: 'sse', paramDiscriminator: 3 }],
    ['x-ferrule-client', { name: 'x' }],
  ])('rejects %s = %j', (key, value) => {
    const base: [string, Json][] = [
      ['x-ferrule-resource', { path: ['a'] }],
      ['x-ferrule-method-name', 'list'],
      ['x-ferrule-package', { name: 'acme', repo: '' }],
    ];
    const ext = [...base.filter(([k]) => k !== key), [key, value as Json] as [string, Json]];
    expect(code(() => readMethodExtensions(ext, '#/p'))).toBe('FER101');
  });
});

describe('client', () => {
  it('emits bearer auth, a client method and default headers', () => {
    const { c } = sdk([method()], {
      clientMethods: [method({ name: 'health', response: undefined })],
      settings: settings({ defaultHeaders: { 'X-Version': '2' } }),
    });
    const out = emitClientFile(c);
    expect(out).toContain('return {"Authorization": f"Bearer {api_key}"}');
    expect(out).toContain('"X-Version": "2",');
    expect(out).toContain('    def health(\n');
    expect(out).toContain('class AsyncAcmeWithStreamedResponse:');
    expect(out).toContain('Client = Acme\n\nAsyncClient = AsyncAcme\n');
  });

  it('emits header api keys, optional and several env options', () => {
    const opt = settings().opts[0];
    if (opt === undefined) throw new Error('no opt');
    const { c } = sdk([method()], {
      settings: settings({
        opts: [
          { ...opt, auth: { securityScheme: 'H', role: 'value' } },
          { ...opt, name: 'org', nullable: true, readEnv: 'ACME_ORG', auth: undefined },
          { ...opt, name: 'region', readEnv: undefined, auth: undefined },
        ],
        securitySchemes: { H: { type: 'apiKey', in: 'header', name: 'X-Key' } },
        security: [['Missing', 'H']],
      }),
    });
    const out = emitClientFile(c);
    expect(out).toContain('return {"X-Key": api_key}');
    expect(out).toContain('org: str | None\n');
    expect(out).toContain('- `org` from `ACME_ORG`');
    expect(out).toContain('by passing region to the client"');
  });

  it('falls back to no auth headers and rejects what it cannot emit', () => {
    const { c } = sdk([method()], { settings: settings({ security: [] }) });
    expect(emitClientFile(c)).toContain(
      'def auth_headers(self) -> dict[str, str]:\n        return {}',
    );
    const many = sdk([method()], {
      settings: settings({
        environments: [
          { name: 'a', url: 'https://a' },
          { name: 'b', url: 'https://b' },
        ],
      }),
    });
    expect(code(() => emitClientFile(many.c))).toBe('FER201');
    expect(code(() => emitClientFile({ ...c, settings: undefined }))).toBe('FER103');
  });
});

describe('resources', () => {
  it('sends bodies, path params, headers and no-content responses', () => {
    const body = obj({
      fields: [
        { wireName: 'name', pyName: 'name', required: true, description: 'The name.', type: str },
        {
          wireName: 'tags',
          pyName: 'tags',
          required: false,
          description: '',
          type: { kind: 'array', items: str },
        },
      ],
    });
    const m = method({
      name: 'update',
      httpMethod: 'post',
      path: '/t/{thingID}',
      pathParams: [param({ wireName: 'thingID', pyName: 'thing_id', required: true })],
      queryParams: [param({ wireName: 'dryRun', pyName: 'dry_run' })],
      body,
      bodyParams: body.fields,
      response: undefined,
    });
    const { c, r } = sdk([m]);
    const out = emitResourceFile(c, r);
    expect(out).toContain('path_template("/t/{thing_id}", thing_id=thing_id)');
    expect(out).toContain('raise ValueError(f"Expected a non-empty value for `thing_id`');
    expect(out).toContain('tags: SequenceNotStr[str] | Omit = omit,');
    expect(out).toContain('body=await async_maybe_transform(');
    expect(out).toContain('cast_to=NoneType,');
    expect(out).toContain('extra_headers = {"Accept": "*/*", **(extra_headers or {})}');
    expect(emitParamsFile(r, m)).toContain('class ThingUpdateParams(TypedDict, total=False):');
  });

  it('pages over GET and POST and streams events', () => {
    const page = {
      scheme: 'cursor' as const,
      page: 'CursorPage',
      paramLocation: 'body' as const,
      request: {},
      response: { data: 'items' },
      headers: {},
      item: obj(),
    };
    const { c, r } = sdk([
      method({ pagination: page }),
      method({
        name: 'search',
        httpMethod: 'post',
        pagination: page,
        bodyParams: [param({})],
        body: obj(),
      }),
      method({
        name: 'events',
        streaming: { format: 'sse', paramDiscriminator: null, event: obj() },
      }),
    ]);
    const out = emitResourceFile(c, r);
    expect(out).toContain('-> SyncCursorPage[ThingListResponse]:');
    expect(out).toContain(
      '-> AsyncPaginator[ThingSearchResponse, AsyncCursorPage[ThingSearchResponse]]:',
    );
    expect(out).toContain('method="post",');
    expect(out).toContain('stream_cls=AsyncStream[ThingEventsResponse],');
    expect(stainlessMethodOrder(r.methods).map((x) => x.name)).toEqual([
      'list',
      'events',
      'search',
    ]);
  });

  it('builds nested resource packages with subresource properties', () => {
    const child: SdkResource = {
      path: ['things', 'parts'],
      order: 0,
      methods: [method()],
      subresources: [],
    };
    const parent: SdkResource = {
      path: ['things'],
      order: 0,
      methods: [method()],
      subresources: [child],
    };
    const { c } = sdk([], { resources: [parent] });
    const files = emitRootFiles(c);
    const pkg = files.get('src/acme/resources/things/things.py') ?? '';
    expect(pkg).toContain(
      'def parts(self) -> PartsResource:\n        return PartsResource(self._client)',
    );
    expect(pkg).toContain('return AsyncPartsResourceWithStreamingResponse(self._things.parts)');
    expect(files.get('src/acme/resources/things/__init__.py')).toContain('"PartsResource",');
    expect(files.has('src/acme/types/things/part_list_response.py')).toBe(true);
    expect(files.has('src/acme/_client.py')).toBe(true);
  });
});

describe('types', () => {
  const variant = (hint: string, tag: string): SdkObject =>
    obj({
      hint,
      description: `${hint} object`,
      fields: [
        {
          wireName: 'type',
          pyName: 'type',
          required: true,
          description: '',
          type: { kind: 'literal', values: [tag] },
        },
      ],
    });

  it('names union variants after their schemas and declares aliases', () => {
    const union: SdkType = {
      kind: 'union',
      variants: [variant('ImageRef', 'image'), variant('VideoRef', 'video')],
    };
    const m = method({
      httpMethod: 'post',
      bodyParams: [
        param({ wireName: 'frame', pyName: 'frame', type: union }),
        param({
          wireName: 'date',
          pyName: 'date',
          type: { kind: 'primitive', name: 'date' },
          required: true,
        }),
      ],
      body: obj(),
    });
    const { r } = sdk([m]);
    const out = emitParamsFile(r, m);
    expect(out).toContain('Frame: TypeAlias = Union[FrameImageRef, FrameVideoRef]');
    expect(out).toContain(
      'class FrameImageRef(TypedDict, total=False):\n    """ImageRef object"""',
    );
    expect(out).toContain(
      'date: Required[Annotated[Union[str, datetime.date], PropertyInfo(format="iso8601")]]',
    );

    const model = emitModelFile({
      ref: { kind: 'ref', name: 'Thing', module: 'thing' },
      type: obj({
        description: 'A thing.',
        fields: [
          {
            wireName: 'frame',
            pyName: 'frame',
            required: false,
            description: '',
            type: { ...union, discriminator: 'type' } as SdkType,
          },
          {
            wireName: 'n',
            pyName: 'n',
            required: false,
            description: '',
            type: { kind: 'union', variants: [str, int] },
          },
        ],
      }),
      prefixChildren: false,
    });
    expect(model).toContain('class Thing(BaseModel):\n    """A thing."""');
    expect(model).toContain(
      'Frame: TypeAlias = Annotated[Union[FrameImageRef, FrameVideoRef], PropertyInfo(discriminator="type")]',
    );
    expect(model).toContain('n: Union[str, int, None] = None');
  });

  it('aliases non-object responses and maps input arrays', () => {
    expect(
      emitAliasFile(
        { kind: 'ref', name: 'Ids', module: 'ids' },
        { kind: 'array', items: { kind: 'ref', name: 'Id', module: 'shared.id' } },
      ),
    ).toContain('from .shared.id import Id');
    const p = new PythonPrinter();
    const ctx = { objectName: () => 'O', refName: () => 'R', input: true };
    expect(toAnnotation({ kind: 'array', items: int }, p, ctx)).toBe('Iterable[int]');
    expect(toAnnotation({ kind: 'primitive', name: 'datetime' }, p, ctx)).toBe(
      'Union[str, datetime]',
    );
  });

  it('documents documented arguments only', () => {
    expect(
      argsDocstring(method({ summary: 'Sum.' }), [
        param({ description: 'D.' }),
        param({ pyName: 'u' }),
      ]),
    ).toContain('  q: D.');
    expect(argsDocstring(method({ description: 'One.\nTwo.' }), undefined)).toEqual([
      '"""',
      'One.',
      'Two.',
      '"""',
    ]);
  });

  it('formats calls the way ruff splits them', () => {
    expect(formatExpr(call('f(', ['x'.repeat(130)]), '')).toEqual([
      'f(',
      `    ${'x'.repeat(130)}`,
      ')',
    ]);
    expect(formatExpr(call('f(', ['a', call('g(', ['b'], true)]), '', '', '')).toEqual([
      'f(',
      '    a,',
      '    g(',
      '        b,',
      '    ),',
      ')',
    ]);
  });
});

describe('project-file input', () => {
  it('maps resources, params and the api-key query auth', () => {
    const m = method({
      pathParams: [param({ wireName: 'id', pyName: 'id', required: true })],
      queryParams: [param({ example: 'x', minimum: 1, maximum: 2 })],
    });
    const { c } = sdk([m]);
    const key = {
      ...c.settings?.opts[0],
      send: { in: 'query', name: 'key' },
    } as SdkClientSettings['opts'][number];
    const client = { ...c, settings: settings({ opts: [key] }) };
    expect(toProjectFilesInput(client, PLACEHOLDER_META)).toBeUndefined();
    const free = sdk([method(), m]).c;
    const input = toProjectFilesInput(
      { ...free, settings: settings({ opts: [key] }) },
      PLACEHOLDER_META,
    );
    expect(input?.auth).toEqual({ kind: 'api_key_query', env: 'ACME_API_KEY', param: 'key' });
    expect(input?.readmeExample.accessor).toBe('things');
    expect(input?.resources[0]?.methods[0]?.params.map((p) => [p.location, p.example])).toEqual([]);
    expect(input?.resources[0]?.variable).toBe('thing');
    expect(toProjectFilesInput({ ...free, settings: undefined }, PLACEHOLDER_META)).toBeUndefined();
  });
});
