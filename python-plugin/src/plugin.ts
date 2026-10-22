/**
 * The plugin entry: Forge -> adapter -> SdkModel -> emitters. Built to WASM with
 * `forge-plugin build`; it imports Forge's runtime, which only exists inside Forge, so it is
 * exercised by the CLI golden test rather than in-process (see vitest.config.ts).
 */
import { definePlugin, type File } from '@cloudflare/forge/plugin';
import { forgeToModel, groupToModel } from './adapter/to-model.ts';
import { emitGroupFiles, emitRootFiles, type FileTree } from './emit/project.ts';

function toFiles(tree: FileTree): File[] {
  return [...tree].map(([path, content]) => ({ path, content }));
}

export const { generate, generateGroup } = definePlugin({
  generate(forge) {
    const client = forgeToModel(forge);
    return client === undefined ? [] : toFiles(emitRootFiles(client));
  },
  generateGroup(group) {
    const client = groupToModel(group);
    return client === undefined ? [] : toFiles(emitGroupFiles(client));
  },
});
