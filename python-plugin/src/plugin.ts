/**
 * The plugin entry: Forge -> adapter -> SdkModel -> emitters. Built to WASM with
 * `forge-plugin build`; it imports Forge's runtime, which only exists inside Forge, so it is
 * exercised by the CLI golden test rather than in-process (see vitest.config.ts).
 */
import { definePlugin, type File } from '@cloudflare/forge/plugin';
import { forgeToModel } from './adapter/to-model.ts';
import { PLACEHOLDER_META } from './emit/files-input.ts';
import { emitRootFiles, type FileTree } from './emit/project.ts';

function toFiles(tree: FileTree): File[] {
  return [...tree].map(([path, content]) => ({ path, content }));
}

export const { generate, generateGroup } = definePlugin({
  generate(forge) {
    const client = forgeToModel(forge);
    return client === undefined ? [] : toFiles(emitRootFiles(client, PLACEHOLDER_META));
  },
  // Every file needs document-level data (gaps G7, G10), so `generate` emits them all.
  generateGroup() {
    return [];
  },
});
