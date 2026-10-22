import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Pagination helpers Phase 0 stub: emits a module docstring only. */
export function emitPagination(client: SdkClient): string {
  return new PythonPrinter().line(`"""Pagination helpers for ${client.packageName}."""`).toString();
}
