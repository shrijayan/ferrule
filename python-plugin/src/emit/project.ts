/** Assembles the files of the document (`generate`) and of one group (`generateGroup`). */
import { FerruleError } from '../diagnostics.ts';
import type { SdkClient, SdkResource } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';
import { HEADER, methodRef, returnModel } from './common.ts';
import { emitModelFile } from './models.ts';
import { emitParamsFile } from './params.ts';
import { emitResourceFile } from './resources.ts';

/** Output tree: relative path -> file contents. */
export type FileTree = ReadonlyMap<string, string>;

function modulePath(pkg: string, module: string): string {
  return `src/${pkg}/${module.replace(/\./g, '/')}.py`;
}

function allResources(resources: readonly SdkResource[]): SdkResource[] {
  return resources.flatMap((r) => [r, ...allResources(r.subresources)]);
}

/**
 * Files no single group owns: the named models (several groups may use one), and
 * `types/shared/__init__.py`. Built from every group's methods.
 */
export function emitRootFiles(client: SdkClient): FileTree {
  const files = new Map<string, string>();
  const pkg = client.package.name;
  for (const decl of client.models) {
    files.set(
      modulePath(pkg, `types.${decl.ref.module}`),
      emitModelFile({ ref: decl.ref, type: decl.type, prefixChildren: false }),
    );
  }
  const shared = client.models.filter((d) => d.ref.module.startsWith('shared.'));
  if (shared.length > 0) {
    const p = new PythonPrinter([HEADER]);
    for (const d of shared) {
      p.importFrom(`.${d.ref.module.slice('shared.'.length)}`, `${d.ref.name} as ${d.ref.name}`);
    }
    files.set(`src/${pkg}/types/shared/__init__.py`, p.toString());
  }
  return files;
}

/** One group's files: its resource modules and its methods' synthesized types. */
export function emitGroupFiles(client: SdkClient): FileTree {
  const files = new Map<string, string>();
  const pkg = client.package.name;
  for (const resource of allResources(client.resources)) {
    if (resource.methods.length === 0) continue;
    files.set(
      modulePath(pkg, `resources.${resource.path.join('.')}`),
      emitResourceFile(client, resource),
    );
    for (const method of resource.methods) {
      const ret = returnModel(resource, method);
      if (ret?.inline !== undefined) {
        if (ret.inline.kind !== 'object') {
          throw new FerruleError(
            'FER201',
            method.pointer,
            `a ${ret.inline.kind} response is not generated yet`,
            'name the response schema in x-ferrule-models, or leave the operation out',
          );
        }
        files.set(
          modulePath(pkg, `types.${ret.ref.module}`),
          emitModelFile({ ref: ret.ref, type: ret.inline, prefixChildren: true }),
        );
      }
      if (method.queryParams.length > 0) {
        const { module } = methodRef(resource, method, 'Params');
        files.set(modulePath(pkg, `types.${module}`), emitParamsFile(resource, method));
      }
    }
  }
  return files;
}
