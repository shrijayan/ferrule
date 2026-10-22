import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Exception hierarchy Phase 0 stub: emits a module docstring only. */
export function emitErrors(client: SdkClient): string {
  return new PythonPrinter()
    .line(`"""Exception hierarchy for ${client.packageName}."""`)
    .toString();
}
