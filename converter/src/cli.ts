#!/usr/bin/env node
// The only module in converter/ that touches the file system.
import { readFileSync, writeFileSync } from 'node:fs';
import { convertText } from './index.ts';

const USAGE = 'usage: ferrule-convert <openapi.(yml|json)> <stainless.yml> -o <out.json>';

export interface Io {
  readonly stdout: (line: string) => void;
  readonly stderr: (line: string) => void;
}

const defaultIo: Io = {
  stdout: (line) => process.stdout.write(`${line}\n`),
  stderr: (line) => process.stderr.write(`${line}\n`),
};

/** Run the CLI; returns the exit code (0 ok, 1 error diagnostics, 2 usage or input error). */
export function main(argv: readonly string[], io: Io = defaultIo): number {
  const positional: string[] = [];
  let out: string | undefined;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '-o' || arg === '--out') out = argv[++i];
    else if (arg !== undefined) positional.push(arg);
  }
  const [specPath, configPath] = positional;
  if (positional.length !== 2 || specPath === undefined || configPath === undefined || !out) {
    io.stderr(USAGE);
    return 2;
  }
  let result: ReturnType<typeof convertText>;
  try {
    result = convertText(readFileSync(specPath, 'utf8'), readFileSync(configPath, 'utf8'));
  } catch (err) {
    io.stderr(`ferrule-convert: ${err instanceof Error ? err.message : String(err)}`);
    return 2;
  }
  for (const d of result.diagnostics) {
    io.stderr(`${d.severity} ${d.code} ${configPath}#${d.pointer}: ${d.message} (${d.suggestion})`);
  }
  if (result.diagnostics.some((d) => d.severity === 'error')) return 1;
  writeFileSync(out, result.output);
  io.stdout(`wrote ${out}`);
  return 0;
}

/* v8 ignore next 3 -- entry point, exercised by the bin smoke test in a child process */
if (import.meta.main) {
  process.exitCode = main(process.argv.slice(2));
}
