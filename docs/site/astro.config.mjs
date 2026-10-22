import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://ferrule-dev.github.io',
  base: '/ferrule',
  integrations: [
    starlight({
      title: 'Ferrule',
      description: 'Leave Stainless without your users noticing.',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/ferrule-dev/ferrule' }],
      sidebar: [
        { label: 'Start', items: ['getting-started'] },
        { label: 'Design', items: ['architecture', 'principles'] },
        { label: 'Fidelity', items: ['fidelity'] },
        { label: 'Decisions', items: ['adr'] },
      ],
    }),
  ],
});
