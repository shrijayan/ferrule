/**
 * Agent context shipped inside every generated SDK (plan §5 item 4, ADR 0013): SKILL.md,
 * AGENTS.md and llms.txt under `src/<pkg>/_agent/` so the wheel carries them, locked to its
 * version; with project metadata also the repository copies (plan §10), one runnable recipe
 * per example under `examples/agent/` and a test that runs them against a mock.
 * Pure: SdkModel in, path -> text out. Not part of the Stainless output.
 */
import type { SdkClient, SdkClientOpt, SdkMethod, SdkResource } from '../../model/index.ts';
import { methodParams, sortFields, stainlessMethodOrder } from '../common.ts';
import { exampleValue } from '../files/api-tests.ts';
import type { ProjectMeta } from '../files-input.ts';

/** One documented call: the code an agent copies, and whether a recipe runs it. */
export interface Snippet {
  readonly title: string;
  /** Recipe file stem, such as `stop_retrieve`. */
  readonly name: string;
  readonly imports: readonly string[];
  /** The statements after `client = ...`. */
  readonly body: readonly string[];
  /** Plain request-response calls run as recipes; paginated and streaming ones are shown only. */
  readonly runnable: boolean;
}

export interface AgentContext {
  readonly pkg: string;
  readonly distribution: string;
  readonly version: string;
  readonly clientClass: string;
  readonly envPrefix: string;
  /** The API key option, if the client reads one from the environment. */
  readonly auth: SdkClientOpt | undefined;
  readonly repo: string;
  readonly quickstart: Snippet | undefined;
  readonly resources: readonly Snippet[];
  readonly pagination: Snippet | undefined;
  readonly streaming: Snippet | undefined;
}

function allResources(resources: readonly SdkResource[]): SdkResource[] {
  return resources.flatMap((r) => [r, ...allResources(r.subresources)]);
}

/** Keyword arguments for the required parameters, path parameters first. */
function callArgs(method: SdkMethod, uses: Set<'parse_date' | 'parse_datetime'>): string[] {
  const required = sortFields(methodParams(method).filter((p) => p.required));
  return [...method.pathParams, ...required].map(
    (p) =>
      `${p.pyName}=${exampleValue(p.type, uses, '', p.example, p.wireName, p.minimum ?? p.maximum)}`,
  );
}

type Ctx = Pick<AgentContext, 'pkg' | 'clientClass'>;

function program(
  ctx: Ctx,
  resource: SdkResource,
  method: SdkMethod,
  body: (call: string) => string[],
  extra: readonly string[] = [],
): Pick<Snippet, 'imports' | 'body'> {
  const uses = new Set<'parse_date' | 'parse_datetime'>();
  const args = [...callArgs(method, uses), ...extra];
  const call = `client.${resource.path.join('.')}.${method.name}(${args.join(', ')})`;
  const imports = [`from ${ctx.pkg} import ${ctx.clientClass}`];
  if (uses.size > 0) imports.push(`from ${ctx.pkg}._utils import ${[...uses].sort().join(', ')}`);
  return { imports, body: body(call) };
}

/** A snippet as a complete program. */
export function snippetCode(ctx: Ctx, s: Snippet): string {
  return [...s.imports, '', `client = ${ctx.clientClass}()`, '', ...s.body, ''].join('\n');
}

function snippetName(resource: SdkResource, method: SdkMethod): string {
  return [...resource.path, method.name].join('_');
}

function plain(ctx: Ctx, r: SdkResource, m: SdkMethod): Snippet {
  const variable = r.path[r.path.length - 1] ?? 'result';
  return {
    title: `\`client.${r.path.join('.')}.${m.name}\` (${m.httpMethod.toUpperCase()} ${m.path})`,
    name: snippetName(r, m),
    ...program(ctx, r, m, (call) => [`${variable} = ${call}`, `print(${variable})`]),
    runnable: m.pagination === undefined && m.streaming === undefined,
  };
}

/** The first method of a resource with no required parameters, else its first method. */
function exampleMethod(r: SdkResource): SdkMethod | undefined {
  const methods = stainlessMethodOrder(r.methods);
  return (
    methods.find((m) => m.pathParams.length === 0 && methodParams(m).every((p) => !p.required)) ??
    methods[0]
  );
}

export function agentContext(client: SdkClient, meta?: ProjectMeta): AgentContext | undefined {
  const s = client.settings;
  if (s === undefined) return undefined;
  const ctx = { pkg: client.package.name, clientClass: s.className };
  const resources = allResources(client.resources);
  const examples = resources.flatMap((r) => {
    const m = exampleMethod(r);
    return m === undefined ? [] : [plain(ctx, r, m)];
  });
  const methods = resources.flatMap((r) =>
    stainlessMethodOrder(r.methods).map((m) => [r, m] as const),
  );
  const paged = methods.find(([, m]) => m.pagination !== undefined);
  const streamed = methods.find(([, m]) => m.streaming !== undefined);
  return {
    pkg: ctx.pkg,
    distribution: meta?.distributionName ?? ctx.pkg,
    version: meta?.version ?? '0.0.1',
    clientClass: s.className,
    envPrefix: s.envPrefix,
    auth: s.opts.find((o) => o.auth !== undefined && o.readEnv !== undefined),
    repo: client.package.repo,
    quickstart: examples.find((e) => e.runnable) ?? examples[0],
    resources: examples,
    pagination:
      paged === undefined
        ? undefined
        : {
            title: 'Pagination',
            name: `${snippetName(...paged)}_pages`,
            ...program(ctx, ...paged, (call) => [
              '# Iterating the page fetches the next pages as needed.',
              `for item in ${call}:`,
              '    print(item)',
            ]),
            runnable: false,
          },
    streaming:
      streamed === undefined
        ? undefined
        : {
            title: 'Streaming',
            name: `${snippetName(...streamed)}_stream`,
            ...program(
              ctx,
              ...streamed,
              (call) => [`stream = ${call}`, 'for event in stream:', '    print(event)'],
              streamed[1].streaming?.paramDiscriminator == null
                ? []
                : [`${streamed[1].streaming.paramDiscriminator}=True`],
            ),
            runnable: false,
          },
  };
}

const fence = (code: string): string => `\`\`\`python\n${code}\`\`\``;

export function emitSkillMd(c: AgentContext): string {
  const env = c.auth?.readEnv;
  const lines = [
    '---',
    `name: ${c.pkg}`,
    `description: Use the ${c.distribution} Python SDK (version ${c.version}) to call the ${c.clientClass} API: client setup, authentication, one example per resource, errors and retries.`,
    '---',
    '',
    `# ${c.distribution} ${c.version}`,
    '',
    `This skill ships inside the \`${c.distribution}\` package, so it matches the installed version. The full method and type reference is \`api.md\` in the repository (https://github.com/${c.repo}).`,
    '',
    '## Install',
    '',
    '```sh',
    `pip install ${c.distribution}==${c.version}`,
    '```',
    '',
    '## Authenticate and create a client',
    '',
  ];
  if (env !== undefined && c.auth !== undefined) {
    lines.push(
      `Set \`${env}\`; the client reads it when \`${c.auth.name}\` is not passed. Never hard-code the key.`,
      '',
    );
  }
  lines.push(
    fence(
      [
        'import os',
        '',
        `from ${c.pkg} import ${c.clientClass}`,
        '',
        `client = ${c.clientClass}(`,
        ...(env !== undefined && c.auth !== undefined
          ? [`    ${c.auth.name}=os.environ.get(${JSON.stringify(env)}),  # the default`]
          : []),
        '    max_retries=2,  # the default; connection errors, 408, 409, 429 and 5xx are retried',
        ')',
        '',
      ].join('\n'),
    ),
    '',
    `Use \`Async${c.clientClass}\` the same way, with \`await\` on every call. \`${c.envPrefix}_BASE_URL\` overrides the base URL.`,
    '',
  );
  if (c.resources.length > 0) {
    lines.push('## Examples', '', 'One call per resource; the arguments are placeholders.', '');
    for (const s of c.resources) lines.push(`### ${s.title}`, '', fence(snippetCode(c, s)), '');
  }
  for (const s of [c.pagination, c.streaming]) {
    if (s !== undefined) lines.push(`## ${s.title}`, '', fence(snippetCode(c, s)), '');
  }
  lines.push(
    '## Errors',
    '',
    fence(
      [
        `import ${c.pkg}`,
        ...(c.quickstart?.imports ?? [`from ${c.pkg} import ${c.clientClass}`]),
        '',
        `client = ${c.clientClass}()`,
        '',
        'try:',
        ...(c.quickstart === undefined
          ? ['    pass']
          : c.quickstart.body
              .join('\n')
              .split('\n')
              .map((l) => `    ${l}`)),
        `except ${c.pkg}.APIConnectionError as e:`,
        '    print("The server could not be reached:", e.__cause__)',
        `except ${c.pkg}.RateLimitError:`,
        '    print("Rate limited (429); back off and retry.")',
        `except ${c.pkg}.APIStatusError as e:`,
        '    print("Non-2xx response:", e.status_code, e.response)',
        '',
      ].join('\n'),
    ),
    '',
    'Every method also has `.with_raw_response` (headers and the raw response) and `.with_streaming_response`.',
    '',
  );
  return lines.join('\n');
}

export function emitAgentsMd(c: AgentContext): string {
  return [
    `# Using ${c.distribution} ${c.version}`,
    '',
    `Instructions for coding agents writing code that uses the \`${c.pkg}\` package.`,
    '',
    `- Read \`SKILL.md\` next to this file first: install, authentication${c.auth?.readEnv === undefined ? '' : ` (\`${c.auth.readEnv}\`)`}, one example per resource and error handling, for this exact version.`,
    `- Look up method signatures and types in \`api.md\` (https://github.com/${c.repo}/blob/main/api.md); every request and response type is importable from \`${c.pkg}.types\`.`,
    `- Construct one \`${c.clientClass}\` (or \`Async${c.clientClass}\`) and reuse it; do not pass secrets in code.`,
    `- Catch \`${c.pkg}.APIError\` subclasses rather than \`httpx\` errors; retries and timeouts are already built in.`,
    '- Do not edit generated files of the SDK; they are regenerated from the API spec.',
    '',
  ].join('\n');
}

export function emitLlmsTxt(c: AgentContext): string {
  const base = `https://github.com/${c.repo}/blob/main`;
  return [
    `# ${c.distribution}`,
    '',
    `> Python SDK for the ${c.clientClass} API, version ${c.version}. Install with \`pip install ${c.distribution}\` and import \`${c.pkg}\`.`,
    '',
    '## Docs',
    '',
    `- [README](${base}/README.md): installation, usage, async client, retries, timeouts`,
    `- [api.md](${base}/api.md): every resource, method and type`,
    `- [SKILL.md](${base}/src/${c.pkg}/_agent/SKILL.md): agent skill with tested examples`,
    '',
  ].join('\n');
}

const RECIPE_HEADER =
  '# Agent recipe generated by Ferrule from SKILL.md; tests/test_agent_recipes.py runs it.';

export function emitRecipeTest(c: AgentContext): string {
  const env = c.auth?.readEnv;
  return [
    '# Generated by Ferrule: runs every examples/agent/ recipe against a mock server.',
    '',
    'from __future__ import annotations',
    '',
    'import os',
    'import runpy',
    'from pathlib import Path',
    '',
    'import httpx',
    'import pytest',
    'from respx import MockRouter',
    '',
    'base_url = os.environ.get("TEST_API_BASE_URL", "http://127.0.0.1:4010")',
    'RECIPES = sorted((Path(__file__).parent.parent / "examples" / "agent").glob("*.py"))',
    '',
    '',
    '@pytest.mark.parametrize("recipe", RECIPES, ids=[p.stem for p in RECIPES])',
    '@pytest.mark.respx(base_url=base_url)',
    'def test_recipe_runs(recipe: Path, respx_mock: MockRouter, monkeypatch: pytest.MonkeyPatch) -> None:',
    '    respx_mock.route().mock(return_value=httpx.Response(200, json={}))',
    ...(env === undefined ? [] : [`    monkeypatch.setenv(${JSON.stringify(env)}, "My API Key")`]),
    `    monkeypatch.setenv(${JSON.stringify(`${c.envPrefix}_BASE_URL`)}, base_url)`,
    '    runpy.run_path(str(recipe), run_name="__main__")',
    '',
  ].join('\n');
}

/** Every agent-context file; the repository copies and recipes need project metadata. */
export function emitAgentFiles(client: SdkClient, meta?: ProjectMeta): Map<string, string> {
  const files = new Map<string, string>();
  const c = agentContext(client, meta);
  if (c === undefined) return files;
  const skill = emitSkillMd(c);
  const agents = emitAgentsMd(c);
  const llms = emitLlmsTxt(c);
  files.set(`src/${c.pkg}/_agent/SKILL.md`, skill);
  files.set(`src/${c.pkg}/_agent/AGENTS.md`, agents);
  files.set(`src/${c.pkg}/_agent/llms.txt`, llms);
  if (meta === undefined) return files;
  files.set(`skills/${c.pkg}/SKILL.md`, skill);
  files.set('AGENTS.md', agents);
  files.set('llms.txt', llms);
  const recipes = c.resources.filter((s) => s.runnable);
  for (const s of recipes)
    files.set(`examples/agent/${s.name}.py`, `${RECIPE_HEADER}\n\n${snippetCode(c, s)}`);
  if (recipes.length > 0) files.set('tests/test_agent_recipes.py', emitRecipeTest(c));
  return files;
}
