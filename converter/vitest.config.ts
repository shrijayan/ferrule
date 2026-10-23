import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    // Property tests and the bin-under-node test spawn work that outlasts 5 s on a busy runner.
    testTimeout: 30_000,
    coverage: {
      provider: 'v8',
      include: ['src/**'],
      thresholds: { branches: 90, lines: 90, functions: 90, statements: 90 },
    },
  },
});
