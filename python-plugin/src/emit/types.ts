import type { SdkClient } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';

/** Models, TypedDict params and Literal enums Phase 0 stub: emits a module docstring only. */
export function emitTypes(client: SdkClient): string {
  return new PythonPrinter()
    .line(`"""Models, TypedDict params and Literal enums for ${client.packageName}."""`)
    .toString();
}
