// /llms.txt (https://llmstxt.org): a plain-text map of the docs for language models.
// Built from the docs collection so titles and descriptions never drift from the pages.

import { getCollection } from 'astro:content';
import type { APIRoute } from 'astro';

const SITE = 'https://shrijayan.github.io/ferrule';

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
    '> Open-source toolkit for teams leaving Stainless. Ferrule turns an OpenAPI spec and a',
    '> `stainless.yml` into the same SDKs Stainless would have generated, then proves that',
    '> nothing public changed. Built on Cloudflare Forge; Apache-2.0; runs locally and in CI.',
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
