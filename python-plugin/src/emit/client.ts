/**
 * `_client.py`: the sync and async client classes, their raw/streaming wrappers and the
 * `Client`/`AsyncClient` aliases, from `SdkClientSettings` and the resource tree. One pass per
 * class with a sync/async mode flag.
 */
import { FerruleError } from '../diagnostics.ts';
import type { SdkClient, SdkClientOpt, SdkClientSettings, SdkResource } from '../model/index.ts';
import { LINE_LENGTH, PythonPrinter } from '../printer.ts';
import { HEADER, pyFString, pyString, resourceModule } from './common.ts';
import { call, printExpr } from './format.ts';
import { allList } from './models.ts';
import { className, emitClientMethods, type Mode } from './resources.ts';

const HERE = '_client';

const STATUS_ERRORS: readonly [string, string][] = [
  ['== 400', 'BadRequestError'],
  ['== 401', 'AuthenticationError'],
  ['== 403', 'PermissionDeniedError'],
  ['== 404', 'NotFoundError'],
  ['== 409', 'ConflictError'],
  ['== 422', 'UnprocessableEntityError'],
  ['== 429', 'RateLimitError'],
  ['>= 500', 'InternalServerError'],
];

const PY_TYPES: Record<SdkClientOpt['type'], string> = {
  string: 'str',
  number: 'float',
  integer: 'int',
  boolean: 'bool',
};

function optType(opt: SdkClientOpt): string {
  const t = PY_TYPES[opt.type];
  return opt.nullable ? `${t} | None` : t;
}

/** `resources.generations` for the package of a resource with subresources. */
function packageModule(r: SdkResource): string {
  return ['resources', ...r.path].join('.');
}

function attr(r: SdkResource): string {
  return r.path[r.path.length - 1] ?? '';
}

interface ClientState {
  readonly p: PythonPrinter;
  readonly client: SdkClient;
  readonly settings: SdkClientSettings;
}

function clientName(s: ClientState, mode: Mode, suffix = ''): string {
  return `${mode === 'async' ? 'Async' : ''}${s.settings.className}${suffix}`;
}

function errorName(s: ClientState): string {
  return `${s.settings.className}Error`;
}

function emitInit(s: ClientState, mode: Mode): void {
  const { p, settings } = s;
  const isAsync = mode === 'async';
  const env = settings.environments[0];
  if (settings.environments.length > 1) {
    throw new FerruleError(
      'FER201',
      '#/x-ferrule-client/environments',
      'a client with several environments is not generated yet',
      'keep one environment in stainless.yml',
    );
  }
  p.block('def __init__(', () => {
    p.line('self,');
    p.line('*,');
    for (const o of settings.opts) p.line(`${o.name}: ${PY_TYPES[o.type]} | None = None,`);
    p.line('base_url: str | httpx.URL | None = None,');
    p.line('timeout: float | Timeout | None | NotGiven = not_given,');
    p.line('max_retries: int = DEFAULT_MAX_RETRIES,');
    p.line('default_headers: Mapping[str, str] | None = None,');
    p.line('default_query: Mapping[str, object] | None = None,');
    p.line('# Configure a custom httpx client.');
    p.line(
      `# We provide a \`Default${isAsync ? 'Async' : ''}HttpxClient\` class that you can pass to retain the default values we use for \`limits\`, \`timeout\` & \`follow_redirects\`.`,
    );
    p.line(
      `# See the [httpx documentation](https://www.python-httpx.org/api/#${isAsync ? 'asyncclient' : 'client'}) for more details.`,
    );
    p.line(`http_client: httpx.${isAsync ? 'AsyncClient' : 'Client'} | None = None,`);
    for (const l of [
      '# Enable or disable schema validation for data returned by the API.',
      '# When enabled an error APIResponseValidationError is raised',
      '# if the API responds with invalid data for the expected schema.',
      '#',
      '# This parameter may be removed or changed in the future.',
      '# If you rely on this feature, please open a GitHub issue',
      '# outlining your use-case to help us decide if it should be',
      '# part of our public interface in the future.',
      '_strict_response_validation: bool = False,',
    ]) {
      p.line(l);
    }
  });
  p.line(') -> None:');
  p.indented(() => {
    const fromEnv = settings.opts.filter((o) => o.readEnv !== undefined);
    p.line(
      `"""Construct a new ${isAsync ? 'async' : 'synchronous'} ${clientName(s, mode)} client instance.`,
    );
    if (fromEnv.length === 1 && fromEnv[0] !== undefined) {
      p.line();
      p.line(
        `This automatically infers the \`${fromEnv[0].name}\` argument from the \`${fromEnv[0].readEnv}\` environment variable if it is not provided.`,
      );
    } else if (fromEnv.length > 1) {
      p.line();
      p.line(
        'This automatically infers the following arguments from their corresponding environment variables if they are not provided:',
      );
      for (const o of fromEnv) p.line(`- \`${o.name}\` from \`${o.readEnv}\``);
    }
    p.line('"""');
    for (const o of settings.opts) {
      if (o.readEnv !== undefined) {
        p.block(`if ${o.name} is None:`, () =>
          p.line(`${o.name} = os.environ.get(${pyString(o.readEnv ?? '')})`),
        );
      }
      if (!o.nullable) {
        p.importFrom('._exceptions', errorName(s));
        const how =
          o.readEnv === undefined
            ? `by passing ${o.name} to the client`
            : `either by passing ${o.name} to the client or by setting the ${o.readEnv} environment variable`;
        p.block(`if ${o.name} is None:`, () =>
          printExpr(
            p,
            call(`raise ${errorName(s)}(`, [
              pyString(`The ${o.name} client option must be set ${how}`),
            ]),
          ),
        );
      }
      p.line(`self.${o.name} = ${o.name}`);
      p.line();
    }
    p.block('if base_url is None:', () =>
      p.line(`base_url = os.environ.get("${settings.envPrefix}_BASE_URL")`),
    );
    p.block('if base_url is None:', () =>
      p.line(`base_url = ${pyFString(env?.url ?? s.client.baseUrl)}`),
    );
    p.line();
    p.importFrom('._utils', 'is_mapping_t');
    p.line(`custom_headers_env = os.environ.get("${settings.envPrefix}_CUSTOM_HEADERS")`);
    p.block('if custom_headers_env is not None:', () => {
      p.line('parsed: dict[str, str] = {}');
      p.block('for line in custom_headers_env.split("\\n"):', () => {
        p.line('colon = line.find(":")');
        p.block('if colon >= 0:', () =>
          p.line('parsed[line[:colon].strip()] = line[colon + 1 :].strip()'),
        );
      });
      p.line(
        'default_headers = {**parsed, **(default_headers if is_mapping_t(default_headers) else {})}',
      );
    });
    p.line();
    p.block('super().__init__(', () => {
      for (const l of [
        'version=__version__,',
        'base_url=base_url,',
        'max_retries=max_retries,',
        'timeout=timeout,',
        'http_client=http_client,',
        'custom_headers=default_headers,',
        'custom_query=default_query,',
        '_strict_response_validation=_strict_response_validation,',
      ]) {
        p.line(l);
      }
    });
    p.line(')');
  });
}

function emitResourceProperties(s: ClientState, mode: Mode): void {
  const { p } = s;
  for (const r of s.client.resources) {
    const cls = className(r, mode);
    p.line();
    p.line('@cached_property');
    p.block(`def ${attr(r)}(self) -> ${cls}:`, () => {
      importLine(p, `.${packageModule(r)}`, cls);
      p.line();
      p.line(`return ${cls}(self)`);
    });
  }
}

function authLines(s: ClientState): string[] | undefined {
  const { settings } = s;
  for (const alt of settings.security) {
    for (const name of alt) {
      const scheme = settings.securitySchemes[name];
      const opt = settings.opts.find((o) => o.auth?.securityScheme === name);
      if (scheme === undefined || opt === undefined) continue;
      if (scheme.type === 'http' && scheme.scheme === 'bearer') {
        return [
          `${opt.name} = self.${opt.name}`,
          `return {"Authorization": f"Bearer {${opt.name}}"}`,
        ];
      }
      if (scheme.type === 'apiKey' && scheme.in === 'header') {
        return [`${opt.name} = self.${opt.name}`, `return {${pyString(scheme.name)}: ${opt.name}}`];
      }
    }
  }
  return undefined;
}

function emitClientClass(s: ClientState, mode: Mode): void {
  const { p, settings } = s;
  const isAsync = mode === 'async';
  p.block(`class ${clientName(s, mode)}(${isAsync ? 'Async' : 'Sync'}APIClient):`, () => {
    if (settings.opts.length > 0) {
      p.line('# client options');
      for (const o of settings.opts) p.line(`${o.name}: ${optType(o)}`);
      p.line();
    }
    emitInit(s, mode);
    emitResourceProperties(s, mode);
    for (const [prop, suffix] of [
      ['with_raw_response', 'WithRawResponse'],
      ['with_streaming_response', 'WithStreamedResponse'],
    ] as const) {
      p.line();
      p.line('@cached_property');
      p.block(`def ${prop}(self) -> ${clientName(s, mode, suffix)}:`, () =>
        p.line(`return ${clientName(s, mode, suffix)}(self)`),
      );
    }
    p.line();
    p.line('@property');
    p.line('@override');
    p.block('def qs(self) -> Querystring:', () =>
      p.line(`return Querystring(array_format=${pyString(settings.arrayFormat)})`),
    );
    p.line();
    p.line('@property');
    p.line('@override');
    p.block('def auth_headers(self) -> dict[str, str]:', () => {
      for (const l of authLines(s) ?? ['return {}']) p.line(l);
    });
    p.line();
    p.line('@property');
    p.line('@override');
    p.block('def default_headers(self) -> dict[str, str | Omit]:', () => {
      p.block('return {', () => {
        p.line('**super().default_headers,');
        p.line(`"X-Stainless-Async": ${isAsync ? 'f"async:{get_async_library()}"' : '"false"'},`);
        for (const [k, v] of Object.entries(settings.defaultHeaders)) {
          p.line(`${pyString(k)}: ${pyString(v)},`);
        }
        p.line('**self._custom_headers,');
      });
      p.line('}');
    });
    const queryOpts = settings.opts.filter((o) => o.send?.in === 'query');
    if (queryOpts.length > 0) {
      p.line();
      p.line('@property');
      p.line('@override');
      p.block('def default_query(self) -> dict[str, object]:', () => {
        p.block('return {', () => {
          p.line('**super().default_query,');
          for (const o of queryOpts) p.line(`${pyString(o.send?.name ?? '')}: self.${o.name},`);
          p.line('**self._custom_query,');
        });
        p.line('}');
      });
    }
    p.line();
    emitCopy(s, mode);
    if (s.client.clientMethods.length > 0) emitClientMethods(p, s.client, HERE, mode);
    for (const o of settings.opts) {
      const scheme =
        o.auth === undefined ? undefined : settings.securitySchemes[o.auth.securityScheme];
      if (scheme?.type === 'apiKey' && scheme.in === 'query') {
        p.line();
        p.block(`def _get_${o.name}_query_param(self) -> str | None:`, () =>
          p.line(`return self.${o.name}`),
        );
      }
    }
    p.line();
    p.line('@override');
    p.block('def _make_status_error(', () => {
      for (const l of [
        'self,',
        'err_msg: str,',
        '*,',
        'body: object,',
        'response: httpx.Response,',
      ])
        p.line(l);
    });
    p.line(') -> APIStatusError:');
    p.indented(() => {
      STATUS_ERRORS.forEach(([cond, err], i) => {
        if (i > 0) p.line();
        p.block(`if response.status_code ${cond}:`, () =>
          p.line(`return _exceptions.${err}(err_msg, response=response, body=body)`),
        );
      });
      p.line('return APIStatusError(err_msg, response=response, body=body)');
    });
  });
}

function emitCopy(s: ClientState, mode: Mode): void {
  const { p, settings } = s;
  p.block('def copy(', () => {
    p.line('self,');
    p.line('*,');
    for (const o of settings.opts) p.line(`${o.name}: ${PY_TYPES[o.type]} | None = None,`);
    for (const l of [
      'base_url: str | httpx.URL | None = None,',
      'timeout: float | Timeout | None | NotGiven = not_given,',
      `http_client: httpx.${mode === 'async' ? 'AsyncClient' : 'Client'} | None = None,`,
      'max_retries: int | NotGiven = not_given,',
      'default_headers: Mapping[str, str] | None = None,',
      'set_default_headers: Mapping[str, str] | None = None,',
      'default_query: Mapping[str, object] | None = None,',
      'set_default_query: Mapping[str, object] | None = None,',
      '_extra_kwargs: Mapping[str, Any] = {},',
    ]) {
      p.line(l);
    }
  });
  p.line(') -> Self:');
  p.indented(() => {
    p.line('"""');
    p.line(
      'Create a new client instance re-using the same options given to the current client with optional overriding.',
    );
    p.line('"""');
    p.block('if default_headers is not None and set_default_headers is not None:', () =>
      p.line(
        'raise ValueError("The `default_headers` and `set_default_headers` arguments are mutually exclusive")',
      ),
    );
    p.line();
    p.block('if default_query is not None and set_default_query is not None:', () =>
      p.line(
        'raise ValueError("The `default_query` and `set_default_query` arguments are mutually exclusive")',
      ),
    );
    p.line();
    p.line('headers = self._custom_headers');
    p.block('if default_headers is not None:', () =>
      p.line('headers = {**headers, **default_headers}'),
    );
    p.block('elif set_default_headers is not None:', () => p.line('headers = set_default_headers'));
    p.line();
    p.line('params = self._custom_query');
    p.block('if default_query is not None:', () => p.line('params = {**params, **default_query}'));
    p.block('elif set_default_query is not None:', () => p.line('params = set_default_query'));
    p.line();
    p.line('http_client = http_client or self._client');
    p.block('return self.__class__(', () => {
      for (const o of settings.opts) p.line(`${o.name}=${o.name} or self.${o.name},`);
      for (const l of [
        'base_url=base_url or self.base_url,',
        'timeout=self.timeout if isinstance(timeout, NotGiven) else timeout,',
        'http_client=http_client,',
        'max_retries=max_retries if is_given(max_retries) else self.max_retries,',
        'default_headers=headers,',
        'default_query=params,',
        '**_extra_kwargs,',
      ]) {
        p.line(l);
      }
    });
    p.line(')');
  });
  p.line();
  p.line('# Alias for `copy` for nicer inline usage, e.g.');
  p.line('# client.with_options(timeout=10).foo.create(...)');
  p.line('with_options = copy');
}

function emitWrapperClass(s: ClientState, mode: Mode, kind: 'raw' | 'streamed'): void {
  const { p } = s;
  const suffix = kind === 'raw' ? 'WithRawResponse' : 'WithStreamedResponse';
  const resSuffix = kind === 'raw' ? 'WithRawResponse' : 'WithStreamingResponse';
  p.block(`class ${clientName(s, mode, suffix)}:`, () => {
    p.line(`_client: ${clientName(s, mode)}`);
    p.line();
    p.block(`def __init__(self, client: ${clientName(s, mode)}) -> None:`, () =>
      p.line('self._client = client'),
    );
    for (const r of s.client.resources) {
      const cls = className(r, mode, resSuffix);
      p.line();
      p.line('@cached_property');
      selfDef(p, attr(r), `${attr(r)}.${cls}`);
      p.indented(() => {
        importLine(p, `.${packageModule(r)}`, cls);
        p.line();
        printExpr(p, call(`${cls}(`, [`self._client.${attr(r)}`]), 'return ');
      });
    }
  });
}

/** `def name(self) -> T:`, split as ruff splits it when it is too long. */
function selfDef(p: PythonPrinter, name: string, returns: string): void {
  const one = `def ${name}(self) -> ${returns}:`;
  if (`${p.indentation}${one}`.length <= LINE_LENGTH) {
    p.line(one);
    return;
  }
  p.line(`def ${name}(`);
  p.indented(() => p.line('self,'));
  p.line(`) -> ${returns}:`);
}

/** A function-local `from module import name`, wrapped as isort wraps it. */
function importLine(p: PythonPrinter, module: string, name: string): void {
  const one = `from ${module} import ${name}`;
  if (`${p.indentation}${one}`.length <= LINE_LENGTH) {
    p.line(one);
    return;
  }
  p.line(`from ${module} import (`);
  p.indented(() => p.line(`${name},`));
  p.line(')');
}

/** Renders `_client.py`. */
export function emitClientFile(client: SdkClient): string {
  const settings = client.settings;
  if (settings === undefined) {
    throw new FerruleError(
      'FER103',
      '#/x-ferrule-client',
      'missing x-ferrule-client',
      'regenerate the annotated spec with ferrule-convert',
    );
  }
  const p = new PythonPrinter([HEADER]);
  const s: ClientState = { p, client, settings };
  p.importFrom('__future__', 'annotations');
  p.importModule('os');
  for (const n of ['TYPE_CHECKING', 'Any', 'Mapping']) p.importFrom('typing', n);
  for (const n of ['Self', 'override']) p.importFrom('typing_extensions', n);
  p.importModule('httpx');
  p.importFrom('.', '_exceptions');
  p.importFrom('._qs', 'Querystring');
  for (const n of [
    'Omit',
    'Timeout',
    'NotGiven',
    'Transport',
    'ProxiesTypes',
    'RequestOptions',
    'not_given',
  ]) {
    p.importFrom('._types', n);
  }
  for (const n of ['is_given', 'get_async_library']) p.importFrom('._utils', n);
  for (const m of ['._types', '._utils', '._base_client']) p.explode(m);
  p.importFrom('._compat', 'cached_property');
  p.importFrom('._version', '__version__');
  p.importFrom('._streaming', 'Stream as Stream');
  p.importFrom('._streaming', 'AsyncStream as AsyncStream');
  p.importFrom('._exceptions', 'APIStatusError');
  for (const n of ['DEFAULT_MAX_RETRIES', 'SyncAPIClient', 'AsyncAPIClient']) {
    p.importFrom('._base_client', n);
  }

  if (client.resources.length > 0) {
    p.block('if TYPE_CHECKING:', () => {
      for (const l of typeCheckingImports(client, p.indentation)) p.raw(l);
    });
    p.line();
  }
  for (const l of allList([
    'Timeout',
    'Transport',
    'ProxiesTypes',
    'RequestOptions',
    clientName(s, 'sync'),
    clientName(s, 'async'),
    'Client',
    'AsyncClient',
  ])) {
    p.line(l);
  }
  for (const mode of ['sync', 'async'] as const) {
    p.line().line();
    emitClientClass(s, mode);
  }
  for (const [mode, kind] of [
    ['sync', 'raw'],
    ['async', 'raw'],
    ['sync', 'streamed'],
    ['async', 'streamed'],
  ] as const) {
    p.line().line();
    emitWrapperClass(s, mode, kind);
  }
  p.line().line();
  p.line(`Client = ${clientName(s, 'sync')}`);
  p.line();
  p.line(`AsyncClient = ${clientName(s, 'async')}`);
  return p.toString();
}

/** The `if TYPE_CHECKING:` imports, sorted like the module's own imports. */
function typeCheckingImports(client: SdkClient, indent: string): string[] {
  const p = new PythonPrinter();
  for (const r of client.resources) {
    p.importFrom('.resources', attr(r));
    p.importFrom(`.${resourceModule(r)}`, className(r, 'sync'));
    p.importFrom(`.${resourceModule(r)}`, className(r, 'async'));
  }
  return p.importBlock(indent);
}
