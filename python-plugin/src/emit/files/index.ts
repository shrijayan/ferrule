/**
 * The non-code files of a generated SDK (Phase 3 task C): README.md, api.md, .stats.yml,
 * project and CI files, and the test suite. Pure: input in, path -> text out.
 */
import { emitApiMd } from './api-md.ts';
import { emitApiTests } from './api-tests.ts';
import type { ExampleRequest, FilesResource, ProjectFilesInput } from './input.ts';
import { render, type TemplateVars } from './render.ts';
import { TEMPLATES } from './templates/index.ts';

export type { ProjectFilesInput } from './input.ts';

function countMethods(resources: readonly FilesResource[]): number {
  return resources.reduce((n, r) => n + r.methods.length + countMethods(r.subresources), 0);
}

export function emitStats(input: ProjectFilesInput): string {
  return [
    `configured_endpoints: ${countMethods(input.resources)}`,
    `openapi_spec_url: ${input.stats.openapiSpecUrl}`,
    `openapi_spec_hash: ${input.stats.openapiSpecHash}`,
    `config_hash: ${input.stats.configHash}`,
    '',
  ].join('\n');
}

const resultVar = (e: ExampleRequest): string => e.accessor.split('.').at(-1) ?? e.accessor;

/** The value of every `%%TOKEN%%` the templates use. */
export function templateVars(input: ProjectFilesInput): TemplateVars {
  const ex = input.readmeExample;
  const probe = input.retryProbe;
  return {
    PACKAGE: input.packageName,
    DIST: input.distributionName,
    CLIENT: input.clientClass,
    ENV_PREFIX: input.envPrefix,
    REPO: input.repo,
    VERSION: input.version,
    YEAR: String(input.copyrightYear),
    ORG_NAME: input.organization.name,
    ORG_TITLE: input.organization.title,
    CONTACT: input.organization.contact,
    DOCS_URL: input.organization.docsUrl,
    DOCS_LABEL: input.organization.docsUrl.replace(/^https?:\/\//, '').replace(/\/$/, ''),
    README_TITLE: input.readmeTitle,
    API_KEY_ENV: input.auth.env,
    AUTH_QUERY: input.auth.param,
    ARRAY_FORMAT: input.query.arrayFormat,
    OBJECT_FORMAT: input.query.objectFormat,
    EX_ACCESSOR: ex.accessor,
    EX_METHOD: ex.method,
    EX_ARGS: ex.args,
    EX_RESULT: resultVar(ex),
    PROBE_VERB: probe.httpMethod,
    PROBE_PATH: probe.path,
    PROBE_ACCESSOR: probe.accessor,
    PROBE_METHOD: probe.method,
    PROBE_ARGS: probe.args,
    PROBE_ARGS_COMMA: probe.args === '' ? '' : `${probe.args}, `,
  };
}

/** Every project file, keyed by its path in the SDK repository. */
export function emitProjectFiles(input: ProjectFilesInput): Map<string, string> {
  const vars = templateVars(input);
  const files = new Map<string, string>();
  for (const [path, template] of TEMPLATES) files.set(path, render(template, vars));
  files.set('api.md', emitApiMd(input));
  files.set('.stats.yml', emitStats(input));
  for (const [path, text] of emitApiTests(input)) files.set(path, text);
  return new Map([...files].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
}
