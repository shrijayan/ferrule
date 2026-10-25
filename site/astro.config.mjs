import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://shrijayan.github.io',
  base: '/ferrule',
  // The landing page is src/pages/index.astro; the docs moved under /docs. Keep old links alive.
  redirects: {
    '/getting-started': '/ferrule/docs/quickstart/',
    '/action': '/ferrule/docs/action/',
    '/architecture': '/ferrule/docs/architecture/',
    '/principles': '/ferrule/docs/principles/',
    '/fidelity': '/ferrule/docs/fidelity/',
    '/adr': '/ferrule/docs/adr/',
  },
  integrations: [
    starlight({
      title: 'Ferrule',
      description: 'Leave Stainless without your users noticing.',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/shrijayan/ferrule' }],
      customCss: ['./src/styles/docs.css'],
      sidebar: [
        { label: 'Start', items: ['docs', 'docs/quickstart', 'docs/commands', 'docs/action'] },
        { label: 'Design', items: ['docs/architecture', 'docs/principles'] },
        { label: 'Fidelity', items: ['docs/fidelity'] },
        { label: 'Decisions', items: ['docs/adr'] },
      ],
    }),
  ],
});
