import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Client and AsyncClient Phase 0 stub: emits a module docstring only. */
export function emitClient(client: SdkClient): string {
  return new PythonPrinter()
    .line(`"""Client and AsyncClient for ${client.packageName}."""`)
    .toString();
}
