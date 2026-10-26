// /llms.txt (https://llmstxt.org): a plain-text map of the docs for language models.
// Built from the docs collection so titles and descriptions never drift from the pages.

import { getCollection } from 'astro:content';
import type { APIRoute } from 'astro';

const SITE = 'https://ferrule.cpluz.com';

// Same groups and order as the Starlight sidebar in astro.config.mjs.
const SECTIONS: [string, string[]][] = [
  ['Start', ['docs', 'docs/quickstart', 'docs/commands', 'docs/action']],
  ['Design', ['docs/architecture', 'docs/principles']],
  ['Fidelity', ['docs/fidelity']],
  ['Decisions', ['docs/adr']],
];

export const GET: APIRoute = async () => {
  const pages = new Map((await getCollection('docs')).map((entry) => [entry.id, entry.data]));
  const link = (id: string) => {
    const page = pages.get(id);
    if (!page) throw new Error(`llms.txt: no docs page "${id}"`);
    return `- [${page.title}](${SITE}/${id}/): ${page.description}`;
  };

  const body = [
    '# Ferrule',
    '',
    '> Open-source toolkit that generates idiomatic SDKs from an OpenAPI spec and a',
    '> `stainless.yml`: Python, TypeScript, Go, Java, Kotlin, Ruby, PHP and C#. Deterministic.',
    '> One Rust binary, built on Cloudflare Forge. Apache-2.0. Runs locally and in CI. No account.',
    '> If you already have a stainless.yml, moving over is seamless: keep the config, and',
    '> `adopt` proves the public surface did not change.',
    '',
    'Ferrule is pre-alpha and not on PyPI yet: run it from a clone with `uv run ferrule …`.',
    'The core is one Rust binary, `ferrule-gen`, with Forge built in. The `ferrule` Python CLI',
    'wraps it with `generate`, `adopt`, `rescue`, `verify` and `merge`.',
    '',
    ...SECTIONS.flatMap(([name, ids]) => [`## ${name}`, '', ...ids.map(link), '']),
    '## Optional',
    '',
    '- [Source](https://github.com/shrijayan/ferrule): repository, issues and releases',
    '- [Plan](https://github.com/shrijayan/ferrule/blob/main/docs/plan.html): the full project plan',
    '- [ADRs](https://github.com/shrijayan/ferrule/tree/main/docs/adr): every architecture decision record',
    '',
  ].join('\n');

  return new Response(body, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
};
