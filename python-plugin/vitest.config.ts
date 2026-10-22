import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/**'],
      // plugin.ts imports Forge's runtime (`forge:plugin/forge`), which exists only inside
      // Forge; the CLI golden test in test/golden.test.ts exercises it.
      exclude: ['src/plugin.ts'],
      thresholds: { branches: 90, lines: 90, functions: 90, statements: 90 },
    },
  },
});
