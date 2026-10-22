/**
 * `resources/<name>.py`: the sync and async resource classes and their raw/streaming wrappers.
 * One pass per class with a sync/async mode flag (CLAUDE.md: DRY across sync and async).
 */
import { FerruleError } from '../diagnostics.ts';
import type { SdkClient, SdkMethod, SdkResource } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';
import { type AnnotationContext, toAnnotation } from '../type-mapper.ts';
import {
  escapeDocstring,
  HEADER,
  methodRef,
  relImport,
  resourceClassBase,
  returnModel,
  sortFields,
  wrap,
} from './common.ts';
import { allList } from './models.ts';

type Mode = 'sync' | 'async';

const OPTIONS_ARGS = [
  'extra_headers=extra_headers',
  'extra_query=extra_query',
  'extra_body=extra_body',
  'timeout=timeout',
];

function className(resource: SdkResource, mode: Mode, suffix = ''): string {
  return `${mode === 'async' ? 'Async' : ''}${resourceClassBase(resource)}Resource${suffix}`;
}

function unsupported(method: SdkMethod, what: string): FerruleError {
  return new FerruleError(
    'FER201',
    method.pointer,
    `${what} is not generated yet`,
    'leave the operation out of the spike',
  );
}

interface FileState {
  readonly p: PythonPrinter;
  readonly here: string;
  readonly ctx: AnnotationContext;
  readonly client: SdkClient;
  readonly resource: SdkResource;
}

function imp(s: FileState, target: string, name: string): void {
  s.p.importFrom(relImport(s.here, target), name);
}

function methodDocstring(method: SdkMethod, args: readonly [string, string][]): string[] {
  const text = (method.description || method.summary).trim();
  const body = wrap(escapeDocstring(text));
  if (args.length === 0) {
    if (body.length === 0) return [];
    if (body.length === 1) return [`"""${body[0]}"""`];
    return ['"""', ...body, '"""'];
  }
  const all: [string, string][] = [
    ...args,
    ['extra_headers', 'Send extra headers'],
    ['extra_query', 'Add additional query parameters to the request'],
    ['extra_body', 'Add additional JSON properties to the request'],
    ['timeout', 'Override the client-level default timeout for this request, in seconds'],
  ];
  const argLines = all.flatMap(([name, desc], i) => {
    const [first = '', ...rest] = wrap(`${name}: ${desc}`);
    return [...(i === 0 ? [] : ['']), `  ${first}`, ...rest.map((l) => `      ${l}`)];
  });
  return ['"""', ...body, ...(body.length > 0 ? [''] : []), 'Args:', ...argLines, '"""'];
}

function emitMethod(s: FileState, method: SdkMethod, mode: Mode): void {
  const { p, resource } = s;
  if (method.pathParams.length > 0) throw unsupported(method, 'a path parameter');
  if (method.body !== undefined) throw unsupported(method, 'a request body');
  const ret = returnModel(resource, method);
  if (ret === undefined) throw unsupported(method, 'a method without a JSON response');
  const model = s.ctx.refName(ret.ref);
  const page = method.pagination;
  const query = sortFields(method.queryParams);
  const isAsync = mode === 'async';

  let returns = model;
  if (page !== undefined) {
    const cls = `${isAsync ? 'Async' : 'Sync'}${page.page}`;
    imp(s, 'pagination', cls);
    if (isAsync) imp(s, '_base_client', 'AsyncPaginator');
    returns = isAsync ? `AsyncPaginator[${model}, ${cls}[${model}]]` : `${cls}[${model}]`;
  }
  // A paginated async method is a plain def returning an AsyncPaginator.
  const awaits = isAsync && page === undefined;

  p.block(`${awaits ? 'async ' : ''}def ${method.name}(`, () => {
    p.line('self,');
    p.line('*,');
    for (const q of query) {
      const ann = toAnnotation(q.type, p, s.ctx);
      if (q.required) p.line(`${q.pyName}: ${ann},`);
      else {
        imp(s, '_types', 'Omit');
        imp(s, '_types', 'omit');
        p.line(`${q.pyName}: ${ann} | Omit = omit,`);
      }
    }
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
    for (const l of methodDocstring(
      method,
      query.map((q) => [q.pyName, q.description.trim() || '']),
    )) {
      p.line(l);
    }
    const call = page !== undefined ? '_get_api_list' : `_${method.httpMethod}`;
    if (page !== undefined && method.httpMethod !== 'get') {
      throw unsupported(method, `pagination over ${method.httpMethod.toUpperCase()}`);
    }
    imp(s, '_base_client', 'make_request_options');
    p.block(`return ${awaits ? 'await ' : ''}self.${call}(`, () => {
      p.line(`${JSON.stringify(method.path)},`);
      if (page !== undefined) {
        p.line(`page=${isAsync ? 'Async' : 'Sync'}${page.page}[${model}],`);
      }
      if (query.length === 0) {
        p.block('options=make_request_options(', () => p.line(OPTIONS_ARGS.join(', ')));
      } else {
        const params = methodRef(resource, method, 'Params');
        imp(s, 'types', params.module);
        const transform = awaits ? 'async_maybe_transform' : 'maybe_transform';
        imp(s, '_utils', transform);
        p.block('options=make_request_options(', () => {
          for (const a of OPTIONS_ARGS) p.line(`${a},`);
          p.block(`query=${awaits ? 'await ' : ''}${transform}(`, () => {
            p.block('{', () => {
              for (const q of query) p.line(`${JSON.stringify(q.wireName)}: ${q.pyName},`);
            });
            p.line('},');
            p.line(`${params.module}.${params.name},`);
          });
          p.line('),');
        });
      }
      p.line('),');
      p.line(page !== undefined ? `model=${model},` : `cast_to=${model},`);
    });
    p.line(')');
  });
}

function emitResourceClass(s: FileState, mode: Mode): void {
  const { p, resource, client } = s;
  const docs = `https://www.github.com/${client.package.repo}`;
  imp(s, '_resource', mode === 'async' ? 'AsyncAPIResource' : 'SyncAPIResource');
  imp(s, '_compat', 'cached_property');
  p.block(
    `class ${className(resource, mode)}(${mode === 'async' ? 'Async' : 'Sync'}APIResource):`,
    () => {
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
      for (const method of resource.methods) {
        p.line();
        emitMethod(s, method, mode);
      }
    },
  );
}

function emitWrapper(s: FileState, mode: Mode, kind: 'raw' | 'streamed'): void {
  const { p, resource } = s;
  const attr = resource.path[resource.path.length - 1] ?? '';
  const suffix = kind === 'raw' ? 'WithRawResponse' : 'WithStreamingResponse';
  const fn = `${mode === 'async' ? 'async_' : ''}to_${kind}_response_wrapper`;
  imp(s, '_response', fn);
  p.block(`class ${className(resource, mode, suffix)}:`, () => {
    p.block(`def __init__(self, ${attr}: ${className(resource, mode)}) -> None:`, () => {
      p.line(`self._${attr} = ${attr}`);
      for (const method of resource.methods) {
        p.line();
        p.block(`self.${method.name} = ${fn}(`, () => p.line(`${attr}.${method.name},`));
        p.line(')');
      }
    });
  });
}

/** Renders `resources/<path>.py` for one resource. */
export function emitResourceFile(client: SdkClient, resource: SdkResource): string {
  const p = new PythonPrinter([HEADER]);
  const here = `resources.${resource.path.join('.')}`;
  const s: FileState = {
    p,
    here,
    client,
    resource,
    ctx: {
      objectName: () => 'object',
      refName: (ref) => {
        imp(s, `types.${ref.module}`, ref.name);
        return ref.name;
      },
    },
  };
  p.importFrom('__future__', 'annotations');
  p.importModule('httpx');
  for (const name of ['Body', 'Query', 'Headers', 'NotGiven', 'not_given']) imp(s, '_types', name);

  // Imports render above the body whatever order they are registered in.
  for (const l of allList([className(resource, 'sync'), className(resource, 'async')])) p.line(l);
  for (const mode of ['sync', 'async'] as const) {
    p.line().line();
    emitResourceClass(s, mode);
  }
  for (const [mode, kind] of [
    ['sync', 'raw'],
    ['async', 'raw'],
    ['sync', 'streamed'],
    ['async', 'streamed'],
  ] as const) {
    p.line().line();
    emitWrapper(s, mode, kind);
  }
  return p.toString();
}
