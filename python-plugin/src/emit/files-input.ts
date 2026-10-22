/**
 * Maps SdkModel to the project-file emitters' input (`emit/files/input.ts`). Values SdkModel
 * cannot know yet come in `ProjectMeta`: `x-ferrule-config` does not carry them (proposed
 * ADR 0003 amendment), so the plugin passes deterministic placeholders until it does.
 */
import type { SdkClient, SdkMethod, SdkParam, SdkResource } from '../model/index.ts';
import { singular, snakeCase, splitWords } from '../naming.ts';
import {
  methodParams,
  methodRef,
  resourceClassBase,
  resourceModule,
  returnModel,
  sortFields,
  stainlessMethodOrder,
} from './common.ts';
import type {
  ExampleRequest,
  FilesMethod,
  FilesParam,
  FilesResource,
  ParamLocation,
  ProjectFilesInput,
} from './files/input.ts';

/** What the project files need that the annotated spec does not carry yet. */
export interface ProjectMeta {
  readonly version: string;
  readonly copyrightYear: number;
  readonly distributionName?: string;
  readonly organization?: {
    readonly title: string;
    readonly contact: string;
    readonly docsUrl: string;
  };
  readonly readmeTitle?: string;
  readonly stats?: ProjectFilesInput['stats'];
  /** `readme.example_requests.default`; else the first method without required parameters. */
  readonly readmeExample?: ExampleRequest;
}

/** Placeholders the plugin uses until `x-ferrule-config` carries the real values. */
export const PLACEHOLDER_META: ProjectMeta = { version: '0.0.1', copyrightYear: 2026 };

function filesParam(p: SdkParam, location: ParamLocation): FilesParam {
  return {
    pyName: p.pyName,
    wireName: p.wireName,
    required: p.required,
    location,
    type: p.type,
    ...(p.example === undefined ? {} : { example: p.example }),
    ...(p.minimum === undefined ? {} : { minimum: p.minimum }),
    ...(p.maximum === undefined ? {} : { maximum: p.maximum }),
  };
}

function filesMethod(resource: SdkResource, m: SdkMethod): FilesMethod {
  const rest = sortFields([...m.queryParams, ...m.bodyParams]).map((p) =>
    filesParam(p, m.queryParams.includes(p) ? 'query' : 'body'),
  );
  const ret = returnModel(resource, m);
  return {
    name: m.name,
    httpMethod: m.httpMethod,
    path: m.path,
    params: [...m.pathParams.map((p) => filesParam(p, 'path')), ...rest],
    ...(methodParams(m).length === 0
      ? {}
      : { paramsModule: `types.${methodRef(resource, m, 'Params').module}` }),
    ...(ret === undefined
      ? {}
      : { response: { name: ret.ref.name, module: `types.${ret.ref.module}` } }),
  };
}

function filesResource(r: SdkResource): FilesResource {
  const name = r.path[r.path.length - 1] ?? '';
  const methods = stainlessMethodOrder(r.methods);
  const types = methods.flatMap((m) => {
    const ret = returnModel(r, m);
    return ret?.inline === undefined ? [] : [ret.ref.name];
  });
  return {
    name,
    title: resourceClassBase(r),
    variable: snakeCase(singular(splitWords(name))),
    module: resourceModule(r),
    types,
    typesModule: ['types', ...r.path.slice(0, -1)].join('.'),
    methods: methods.map((m) => filesMethod(r, m)),
    subresources: r.subresources.map(filesResource),
  };
}

function firstExample(resources: readonly SdkResource[]): ExampleRequest | undefined {
  for (const r of resources) {
    for (const m of stainlessMethodOrder(r.methods)) {
      if (m.pathParams.length === 0 && methodParams(m).every((p) => !p.required)) {
        return {
          accessor: r.path.join('.'),
          method: m.name,
          args: '',
          httpMethod: m.httpMethod,
          path: m.path,
        };
      }
    }
    const nested = firstExample(r.subresources);
    if (nested !== undefined) return nested;
  }
  return undefined;
}

/** The project-file input, or undefined for a client the file templates cannot describe yet. */
export function toProjectFilesInput(
  client: SdkClient,
  meta: ProjectMeta,
): ProjectFilesInput | undefined {
  const s = client.settings;
  const key = s?.opts.find((o) => o.send?.in === 'query' && o.readEnv !== undefined);
  const example = meta.readmeExample ?? firstExample(client.resources);
  if (
    s === undefined ||
    key?.send === undefined ||
    key.readEnv === undefined ||
    example === undefined
  ) {
    return undefined;
  }
  return {
    edition: 'python.2025-10-08',
    packageName: client.package.name,
    distributionName: meta.distributionName ?? client.package.name,
    clientClass: s.className,
    envPrefix: s.envPrefix,
    repo: client.package.repo,
    version: meta.version,
    copyrightYear: meta.copyrightYear,
    organization: {
      name: s.name,
      title: meta.organization?.title ?? s.className,
      contact: meta.organization?.contact ?? '',
      docsUrl: meta.organization?.docsUrl ?? '',
    },
    readmeTitle: meta.readmeTitle ?? s.className,
    auth: { kind: 'api_key_query', env: key.readEnv, param: key.send.name },
    query: { arrayFormat: s.arrayFormat, objectFormat: s.nestedFormat },
    stats: meta.stats ?? { openapiSpecUrl: '', openapiSpecHash: '', configHash: '' },
    readmeExample: example,
    retryProbe: example,
    sharedTypes: client.models
      .filter((d) => d.ref.module.startsWith('shared.'))
      .map((d) => d.ref.name),
    resources: client.resources.map(filesResource),
  };
}
