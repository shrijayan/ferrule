import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Generated tests Phase 0 stub: emits a module docstring only. */
export function emitTests(client: SdkClient): string {
  return new PythonPrinter().line(`"""Generated tests for ${client.packageName}."""`).toString();
}
