// Copyright 2026 Ferrule contributors
// SPDX-License-Identifier: Apache-2.0
// Parse a YAML corpus spec (data) and write it as JSON, so the Python harness needs only the
// stdlib. Uses the workspace's pinned `yaml` package with its alias-expansion limit kept on.
// Usage: node scripts/corpus/to_json.mjs <in.yaml> <out.json>
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../../package.json', import.meta.url));
const { parse } = require('yaml');

const [input, output] = process.argv.slice(2);
if (!input || !output) {
  process.stderr.write('usage: to_json.mjs <in.yaml> <out.json>\n');
  process.exit(2);
}
const doc = parse(readFileSync(input, 'utf8'), { maxAliasCount: 1000, merge: true });
writeFileSync(output, `${JSON.stringify(doc)}\n`);
