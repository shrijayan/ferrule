/**
 * `resources/<name>.py`: the sync and async resource classes and their raw/streaming wrappers.
 * One pass per class with a sync/async mode flag (CLAUDE.md: DRY across sync and async).
 */
import { FerruleError } from '../diagnostics.ts';
import type { SdkClient, SdkMethod, SdkParam, SdkResource } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';
import { type AnnotationContext, isNullable, toAnnotation } from '../type-mapper.ts';
import {
  argsDocstring,
  HEADER,
  methodParams,
  methodRef,
  pyString,
  relImport,
  resourceClassBase,
  resourceModule,
  returnModel,
  stainlessMethodOrder,
} from './common.ts';
import { type Bracket, call, type Expr, printExpr } from './format.ts';
import { allList } from './models.ts';
import { nameParams, paramsContext } from './params.ts';

export type Mode = 'sync' | 'async';

const OPTIONS_ARGS = [
  'extra_headers=extra_headers',
  'extra_query=extra_query',
  'extra_body=extra_body',
  'timeout=timeout',
];

/** `AsyncStopResourceWithRawResponse` and friends. */
export function className(resource: SdkResource, mode: Mode, suffix = ''): string {
  return `${mode === 'async' ? 'Async' : ''}${resourceClassBase(resource)}Resource${suffix}`;
}

function unsupported(method: SdkMethod, what: string): FerruleError {
  return new FerruleError(
    'FER201',
    method.pointer,
    `${what} is not generated yet`,
    'leave the operation out, or file an issue with the spec fragment',
  );
}

interface FileState {
  readonly p: PythonPrinter;
  readonly here: string;
  readonly client: SdkClient;
  readonly resource: SdkResource | undefined;
}

function imp(s: FileState, target: string, name: string): void {
  s.p.importFrom(relImport(s.here, target), name);
}

function annotationContext(
  s: FileState,
  params: readonly SdkParam[],
  paramsModule: string | undefined,
): AnnotationContext {
  const leaf = paramsModule?.split('.').pop() ?? '';
  return paramsContext(
    nameParams(params),
    `${leaf}.`,
    (ref) => {
      imp(s, `types.${ref.module}`, ref.name);
      return ref.name;
    },
    (module, name) => imp(s, module, name),
  );
}

/** One keyword argument of a method signature. */
function argLine(s: FileState, q: SdkParam, ctx: AnnotationContext): string {
  let ann = toAnnotation(q.type, s.p, ctx);
  if (q.required) return `${q.pyName}: ${ann},`;
  if (!isNullable(q.type) && q.type.kind === 'unknown') ann = 'object';
  imp(s, '_types', 'Omit');
  imp(s, '_types', 'omit');
  return `${q.pyName}: ${ann} | Omit = omit,`;
}

/** `path_template("/a/{b_id}", b_id=b_id)` or the plain path string. */
function pathExpr(method: SdkMethod, s: FileState): Expr {
  if (method.pathParams.length === 0) return pyString(method.path);
  imp(s, '_utils', 'path_template');
  let template = method.path;
  for (const p of method.pathParams)
    template = template.replace(`{${p.wireName}}`, `{${p.pyName}}`);
  return call('path_template(', [
    pyString(template),
    ...method.pathParams.map((p) => `${p.pyName}=${p.pyName}`),
  ]);
}

/** `maybe_transform({...}, module.Params)`; awaited in async methods that await. */
function transformExpr(
  s: FileState,
  key: string,
  params: readonly SdkParam[],
  paramsName: string,
  awaits: boolean,
): Bracket {
  const fn = awaits ? 'async_maybe_transform' : 'maybe_transform';
  imp(s, '_utils', fn);
  return call(`${key}=${awaits ? 'await ' : ''}${fn}(`, [
    call(
      '{',
      params.map((q) => `${pyString(q.pyName)}: ${q.pyName}`),
      params.length > 1,
      '}',
    ),
    paramsName,
  ]);
}

function emitMethod(
  s: FileState,
  method: SdkMethod,
  mode: Mode,
  holder: SdkResource | undefined,
): void {
  const { p } = s;
  const isAsync = mode === 'async';
  const page = method.pagination;
  const stream = method.streaming;
  if (stream?.paramDiscriminator) throw unsupported(method, 'a param-switched streaming method');
  if (method.body !== undefined && method.bodyParams.length === 0) {
    throw unsupported(method, 'a request body that is not an object');
  }
  const ret = returnModel(holder, method);
  const params = methodParams(method);
  const paramsRef = params.length > 0 ? methodRef(holder, method, 'Params') : undefined;
  const ctx = annotationContext(s, params, paramsRef?.module);

  let model = 'None';
  if (ret !== undefined) model = ctx.refName(ret.ref);
  let returns = model;
  if (page !== undefined) {
    const cls = `${isAsync ? 'Async' : 'Sync'}${page.page}`;
    imp(s, 'pagination', cls);
    if (isAsync) imp(s, '_base_client', 'AsyncPaginator');
    returns = isAsync ? `AsyncPaginator[${model}, ${cls}[${model}]]` : `${cls}[${model}]`;
  } else if (stream !== undefined) {
    const cls = isAsync ? 'AsyncStream' : 'Stream';
    imp(s, '_streaming', cls);
    returns = `${cls}[${model}]`;
  }
  // A paginated async method is a plain def returning an AsyncPaginator.
  const awaits = isAsync && page === undefined;

  p.block(`${awaits ? 'async ' : ''}def ${method.name}(`, () => {
    p.line('self,');
    for (const q of method.pathParams) p.line(argLine(s, q, ctx));
    p.line('*,');
    for (const q of params) p.line(argLine(s, q, ctx));
    p.line(
      "# Use the following arguments if you need to pass additional parameters to the API that aren't available via kwargs.",
    );
    p.line(
      '# The extra values given here take precedence over values defined on the client or passed to this method.',
    );
    p.line('extra_headers: Headers | None = None,');
    p.line('extra_query: Query | None = None,');
    p.line('extra_body: Body | None = None,');
    p.line('timeout: float | httpx.Timeout | None | NotGiven = not_given,');
  });
  p.line(`) -> ${returns}:`);
  p.indented(() => {
    const hasArgs = method.pathParams.length > 0 || params.length > 0;
    for (const l of argsDocstring(method, hasArgs ? params : undefined)) p.line(l);
    for (const q of method.pathParams) {
      p.block(`if not ${q.pyName}:`, () =>
        p.line(
          `raise ValueError(f"Expected a non-empty value for \`${q.pyName}\` but received {${q.pyName}!r}")`,
        ),
      );
    }
    if (ret === undefined && stream === undefined) {
      p.line('extra_headers = {"Accept": "*/*", **(extra_headers or {})}');
    }
    const paramsName =
      paramsRef === undefined ? '' : `${paramsRef.module.split('.').pop()}.${paramsRef.name}`;
    if (paramsRef !== undefined) {
      const parts = paramsRef.module.split('.');
      imp(s, ['types', ...parts.slice(0, -1)].join('.'), parts[parts.length - 1] ?? '');
    }
    const query = params.filter((q) => method.queryParams.includes(q));
    const body = params.filter((q) => method.bodyParams.includes(q));
    const inBody = page?.paramLocation === 'body' || method.httpMethod !== 'get';
    const items: Expr[] = [pathExpr(method, s)];
    if (page !== undefined) items.push(`page=${isAsync ? 'Async' : 'Sync'}${page.page}[${model}]`);
    if (body.length > 0 && inBody) {
      items.push(transformExpr(s, 'body', body, paramsName, awaits && page === undefined));
    }
    imp(s, '_base_client', 'make_request_options');
    if (query.length === 0) {
      items.push(call('options=make_request_options(', OPTIONS_ARGS));
    } else {
      items.push(
        call(
          'options=make_request_options(',
          [...OPTIONS_ARGS, transformExpr(s, 'query', query, paramsName, awaits)],
          true,
        ),
      );
    }
    if (page !== undefined) items.push(`model=${model}`);
    else if (ret === undefined && stream === undefined) {
      imp(s, '_types', 'NoneType');
      items.push('cast_to=NoneType');
    } else items.push(`cast_to=${model}`);
    if (page !== undefined && method.httpMethod !== 'get')
      items.push(`method="${method.httpMethod}"`);
    if (stream !== undefined) {
      items.push('stream=True');
      items.push(`stream_cls=${isAsync ? 'AsyncStream' : 'Stream'}[${model}]`);
    }
    const fn = page !== undefined ? '_get_api_list' : `_${method.httpMethod}`;
    printExpr(p, call(`self.${fn}(`, items, true), `return ${awaits ? 'await ' : ''}`);
  });
}

/** The class attribute name of a subresource. */
function attrName(resource: SdkResource): string {
  return resource.path[resource.path.length - 1] ?? '';
}

function emitResourceClass(s: FileState, resource: SdkResource, mode: Mode): void {
  const { p, client } = s;
  const docs = `https://www.github.com/${client.package.repo}`;
  imp(s, '_resource', mode === 'async' ? 'AsyncAPIResource' : 'SyncAPIResource');
  imp(s, '_compat', 'cached_property');
  p.block(
    `class ${className(resource, mode)}(${mode === 'async' ? 'Async' : 'Sync'}APIResource):`,
    () => {
      for (const sub of resource.subresources) {
        importSubresource(s, sub, className(sub, mode));
        p.line('@cached_property');
        p.block(`def ${attrName(sub)}(self) -> ${className(sub, mode)}:`, () =>
          p.line(`return ${className(sub, mode)}(self._client)`),
        );
        p.line();
      }
      p.line('@cached_property');
      p.block(
        `def with_raw_response(self) -> ${className(resource, mode, 'WithRawResponse')}:`,
        () => {
          p.line('"""');
          p.line('This property can be used as a prefix for any HTTP method call to return');
          p.line('the raw response object instead of the parsed content.');
          p.line();
          p.line(`For more information, see ${docs}#accessing-raw-response-data-eg-headers`);
          p.line('"""');
          p.line(`return ${className(resource, mode, 'WithRawResponse')}(self)`);
        },
      );
      p.line();
      p.line('@cached_property');
      p.block(
        `def with_streaming_response(self) -> ${className(resource, mode, 'WithStreamingResponse')}:`,
        () => {
          p.line('"""');
          p.line(
            "An alternative to `.with_raw_response` that doesn't eagerly read the response body.",
          );
          p.line();
          p.line(`For more information, see ${docs}#with_streaming_response`);
          p.line('"""');
          p.line(`return ${className(resource, mode, 'WithStreamingResponse')}(self)`);
        },
      );
      for (const method of stainlessMethodOrder(resource.methods)) {
        p.line();
        emitMethod(s, method, mode, resource);
      }
    },
  );
}

function importSubresource(s: FileState, sub: SdkResource, name: string): void {
  imp(s, resourceModule(sub), name);
}

function emitWrapper(
  s: FileState,
  resource: SdkResource,
  mode: Mode,
  kind: 'raw' | 'streamed',
): void {
  const { p } = s;
  const attr = attrName(resource);
  const suffix = kind === 'raw' ? 'WithRawResponse' : 'WithStreamingResponse';
  const fn = `${mode === 'async' ? 'async_' : ''}to_${kind}_response_wrapper`;
  p.block(`class ${className(resource, mode, suffix)}:`, () => {
    p.block(`def __init__(self, ${attr}: ${className(resource, mode)}) -> None:`, () => {
      p.line(`self._${attr} = ${attr}`);
      const methods = stainlessMethodOrder(resource.methods);
      if (methods.length > 0) {
        imp(s, '_response', fn);
        p.line();
      }
      for (const method of methods) {
        p.block(`self.${method.name} = ${fn}(`, () => p.line(`${attr}.${method.name},`));
        p.line(')');
      }
    });
    for (const sub of resource.subresources) {
      importSubresource(s, sub, className(sub, mode, suffix));
      p.line();
      p.line('@cached_property');
      p.block(`def ${attrName(sub)}(self) -> ${className(sub, mode, suffix)}:`, () =>
        printExpr(
          p,
          call(`${className(sub, mode, suffix)}(`, [`self._${attr}.${attrName(sub)}`]),
          'return ',
        ),
      );
    }
  });
}

/** Renders the module of one resource (`resources/<path>.py`, or `<path>/<name>.py` with children). */
export function emitResourceFile(client: SdkClient, resource: SdkResource): string {
  const p = new PythonPrinter([HEADER]);
  const s: FileState = { p, here: resourceModule(resource), client, resource };
  p.importFrom('__future__', 'annotations');
  p.importModule('httpx');
  for (const name of ['Body', 'Query', 'Headers', 'NotGiven', 'not_given']) imp(s, '_types', name);

  // Imports render above the body whatever order they are registered in.
  for (const l of allList([className(resource, 'sync'), className(resource, 'async')])) p.line(l);
  for (const mode of ['sync', 'async'] as const) {
    p.line().line();
    emitResourceClass(s, resource, mode);
  }
  for (const [mode, kind] of [
    ['sync', 'raw'],
    ['async', 'raw'],
    ['sync', 'streamed'],
    ['async', 'streamed'],
  ] as const) {
    p.line().line();
    emitWrapper(s, resource, mode, kind);
  }
  return p.toString();
}

/** Emits `$client` methods into a client class (used by the client emitter). */
export function emitClientMethods(
  p: PythonPrinter,
  client: SdkClient,
  here: string,
  mode: Mode,
): void {
  const s: FileState = { p, here, client, resource: undefined };
  for (const method of stainlessMethodOrder(client.clientMethods)) {
    p.line();
    emitMethod(s, method, mode, undefined);
  }
}
