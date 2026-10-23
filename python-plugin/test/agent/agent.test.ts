import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { forgeToModel } from '../../src/adapter/to-model.ts';
import { agentContext, emitAgentFiles } from '../../src/emit/agent/index.ts';
import { PLACEHOLDER_META } from '../../src/emit/files-input.ts';
import type { SdkClient, SdkMethod } from '../../src/model/index.ts';
import { fakeForge } from '../helpers/fake-forge.ts';

const here = dirname(fileURLToPath(import.meta.url));
/**
 * One JSON file (path -> text) rather than a tree, so the root ruff and pyright runs, which
 * exclude only test/golden, do not check the generated recipes as repository code.
 */
const golden = join(here, 'onebusaway.golden.json');
const { UPDATE_AGENT_GOLDEN } = process.env;
/** `UPDATE_AGENT_GOLDEN=1` rewrites the goldens (a human approves the diff). */
const updating = UPDATE_AGENT_GOLDEN === '1';

function model(fixture: string): SdkClient {
  const path = join(here, '..', 'fixtures', `${fixture}.json`);
  const client = forgeToModel(fakeForge(JSON.parse(readFileSync(path, 'utf8'))));
  if (client === undefined) throw new Error(`no model for ${fixture}`);
  return client;
}

/** Every ```python block of a Markdown file. */
function pythonBlocks(md: string): string[] {
  return [...md.matchAll(/```python\n([\s\S]*?)```/g)].map((m) => m[1] ?? '');
}

/** Python sources that `ast.parse` rejects, with the error. */
function unparsable(sources: readonly string[]): string[] {
  const script =
    'import ast, json, sys\nfor i, s in enumerate(json.load(sys.stdin)):\n' +
    '    try: ast.parse(s)\n    except SyntaxError as e: print(i, e)\n';
  const r = spawnSync('python3', ['-c', script], {
    input: JSON.stringify(sources),
    encoding: 'utf8',
  });
  if (r.status !== 0) throw new Error(r.stderr);
  return r.stdout.split('\n').filter((l) => l !== '');
}

describe('agent context for onebusaway', () => {
  const files = emitAgentFiles(model('onebusaway'), PLACEHOLDER_META);

  it('matches the golden files', () => {
    if (updating) {
      writeFileSync(golden, `${JSON.stringify(Object.fromEntries(files), null, 2)}\n`);
    }
    expect(Object.fromEntries(files)).toEqual(JSON.parse(readFileSync(golden, 'utf8')));
  });

  it('ships the skill, AGENTS.md and llms.txt inside the package', () => {
    for (const name of ['SKILL.md', 'AGENTS.md', 'llms.txt']) {
      expect(files.has(`src/onebusaway/_agent/${name}`)).toBe(true);
    }
    expect(files.get('skills/onebusaway/SKILL.md')).toBe(
      files.get('src/onebusaway/_agent/SKILL.md'),
    );
  });

  it('writes Python that parses: every SKILL.md block, recipe and the recipe test', () => {
    const skill = files.get('src/onebusaway/_agent/SKILL.md') ?? '';
    const blocks = pythonBlocks(skill);
    expect(blocks.length).toBeGreaterThan(3);
    const py = [...files].filter(([p]) => p.endsWith('.py')).map(([, t]) => t);
    expect(py.length).toBeGreaterThan(1);
    expect(unparsable([...blocks, ...py])).toEqual([]);
  });

  it('starts SKILL.md with name and description frontmatter', () => {
    const skill = files.get('src/onebusaway/_agent/SKILL.md') ?? '';
    expect(skill).toMatch(/^---\nname: onebusaway\ndescription: .+\n---\n/);
  });
});

describe('emitAgentFiles', () => {
  it('emits only the package copies without project metadata', () => {
    expect([...emitAgentFiles(model('onebusaway')).keys()]).toEqual([
      'src/onebusaway/_agent/SKILL.md',
      'src/onebusaway/_agent/AGENTS.md',
      'src/onebusaway/_agent/llms.txt',
    ]);
  });

  it('emits nothing for a client without settings', () => {
    const client = { ...model('onebusaway'), settings: undefined };
    expect(emitAgentFiles(client, PLACEHOLDER_META).size).toBe(0);
  });

  it('is deterministic', () => {
    expect(emitAgentFiles(model('onebusaway'), PLACEHOLDER_META)).toEqual(
      emitAgentFiles(model('onebusaway'), PLACEHOLDER_META),
    );
  });

  it('documents pagination and streaming when a method has them', () => {
    const base = model('onebusaway');
    const [first, ...rest] = base.resources;
    if (first === undefined) throw new Error('no resources');
    const method = first.methods[0] as SdkMethod;
    const paged: SdkMethod = {
      ...method,
      name: 'list',
      pagination: {
        scheme: 'cursor',
        page: 'CursorPage',
        request: {},
        response: {},
        paramLocation: 'query',
        headers: {},
        item: { kind: 'unknown' },
      },
    };
    const streamed: SdkMethod = {
      ...method,
      name: 'stream',
      streaming: { format: 'sse', paramDiscriminator: 'stream', event: undefined },
    };
    const client = {
      ...base,
      resources: [{ ...first, methods: [paged, streamed, ...first.methods] }, ...rest],
    };
    const c = agentContext(client);
    expect(c?.pagination?.body.join('\n')).toContain('for item in client.');
    expect(c?.streaming?.body[0]).toContain('stream=True');
    const skill = emitAgentFiles(client).get('src/onebusaway/_agent/SKILL.md') ?? '';
    expect(skill).toContain('## Pagination');
    expect(skill).toContain('## Streaming');
    expect(unparsable(pythonBlocks(skill))).toEqual([]);
  });
});
