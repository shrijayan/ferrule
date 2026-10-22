import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Sync and async resources Phase 0 stub: emits a module docstring only. */
export function emitResources(client: SdkClient): string {
  return new PythonPrinter()
    .line(`"""Sync and async resources for ${client.packageName}."""`)
    .toString();
}
