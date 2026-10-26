import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://ferrule.cpluz.com',
  // The landing page is src/pages/index.astro; the docs moved under /docs. Keep old links alive.
  redirects: {
    '/getting-started': '/docs/quickstart/',
    '/action': '/docs/action/',
    '/architecture': '/docs/architecture/',
    '/principles': '/docs/principles/',
    '/fidelity': '/docs/fidelity/',
    '/adr': '/docs/adr/',
  },
  integrations: [
    starlight({
      title: 'Ferrule',
      description: 'Generate SDKs from an OpenAPI spec. Open source, on your machine, no account.',
      logo: { src: './src/assets/ferrule-mark.svg', alt: '' },
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/shrijayan/ferrule' }],
      customCss: ['./src/styles/docs.css'],
      head: [
        { tag: 'link', attrs: { rel: 'preconnect', href: 'https://fonts.googleapis.com' } },
        {
          tag: 'link',
          attrs: { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: true },
        },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: 'https://fonts.googleapis.com/css2?family=Geist:wght@300..700&family=Geist+Mono:wght@400;500&family=Instrument+Serif:ital@0;1&display=swap',
          },
        },
      ],
      expressiveCode: {
        themes: ['vitesse-dark', 'vitesse-light'],
        styleOverrides: {
          borderRadius: '12px',
          borderColor: 'var(--ferrule-line-2)',
          codeFontFamily: 'var(--__sl-font-mono)',
          codeFontSize: '0.8125rem',
          uiFontFamily: 'var(--__sl-font)',
          frames: {
            shadowColor: 'transparent',
            editorTabBarBackground: 'var(--ferrule-panel)',
            terminalTitlebarBackground: 'var(--ferrule-panel)',
            terminalTitlebarBorderBottomColor: 'var(--ferrule-line)',
          },
        },
      },
      sidebar: [
        { label: 'Start', items: ['docs', 'docs/quickstart', 'docs/commands', 'docs/action'] },
        { label: 'Design', items: ['docs/architecture', 'docs/principles'] },
        { label: 'Fidelity', items: ['docs/fidelity', 'docs/real-world', 'docs/corpus-findings'] },
        { label: 'Decisions', items: ['docs/adr'] },
      ],
    }),
  ],
});
