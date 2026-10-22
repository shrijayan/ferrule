import type { SdkClient } from '../model/index.ts';
import { emitClient } from './client.ts';
import { emitErrors } from './errors.ts';
import { emitPagination } from './pagination.ts';
import { emitResources } from './resources.ts';
import { emitTests } from './tests.ts';
import { emitTypes } from './types.ts';

/** Output tree: relative path -> file contents. */
export type FileTree = ReadonlyMap<string, string>;

/** Assemble every emitted file for one SDK (pyproject, README, api.md, .stats.yml come later). */
export function emitProject(client: SdkClient): FileTree {
  const pkg = `src/${client.packageName}`;
  return new Map([
    [`${pkg}/_client.py`, emitClient(client)],
    [`${pkg}/_exceptions.py`, emitErrors(client)],
    [`${pkg}/pagination.py`, emitPagination(client)],
    [`${pkg}/resources/__init__.py`, emitResources(client)],
    [`${pkg}/types/__init__.py`, emitTypes(client)],
    ['tests/__init__.py', emitTests(client)],
  ]);
}
