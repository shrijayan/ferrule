import { type AdapterInput, toModel } from './adapter/to-model.ts';
import { emitProject, type FileTree } from './emit/project.ts';

export type { SdkClient, SdkMethod, SdkResource, SdkType } from './model/index.ts';
export { pascalCase, snakeCase, splitWords } from './naming.ts';
export { PythonPrinter } from './printer.ts';
export { toAnnotation } from './type-mapper.ts';

/**
 * Phase 0 entry point. Becomes `definePlugin({ generate, generateGroup })` once Forge is
 * installable; nothing here may import Forge yet.
 */
export function generate(input: AdapterInput): FileTree {
  return emitProject(toModel(input));
}
