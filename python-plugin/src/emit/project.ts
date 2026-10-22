/**
 * Assembles the code files of the SDK from one SdkClient: `_client.py`, `resources/**`,
 * `types/**` (`pagination.py` is rendered by the runtime). Everything is emitted from Forge's `generate`, which sees every
 * group; `generateGroup` emits nothing (Forge gaps G7, G10).
 */
import type { SdkClient, SdkMethod, SdkResource } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';
import { emitClientFile } from './client.ts';
import {
  HEADER,
  methodParams,
  methodRef,
  pyString,
  resourceModule,
  returnModel,
} from './common.ts';
import { emitAliasFile, emitModelFile } from './models.ts';
import { emitParamsFile } from './params.ts';
import { className, emitResourceFile } from './resources.ts';

/** Output tree: relative path -> file contents. */
export type FileTree = ReadonlyMap<string, string>;

function modulePath(pkg: string, module: string): string {
  return `src/${pkg}/${module.replace(/\./g, '/')}.py`;
}

export function allResources(resources: readonly SdkResource[]): SdkResource[] {
  return resources.flatMap((r) => [r, ...allResources(r.subresources)]);
}

const WRAPPERS = [
  ['sync', ''],
  ['async', ''],
  ['sync', 'WithRawResponse'],
  ['async', 'WithRawResponse'],
  ['sync', 'WithStreamingResponse'],
  ['async', 'WithStreamingResponse'],
] as const;

/** `resources/__init__.py` or `resources/<a>/__init__.py`: the resource classes in order. */
function resourcesInit(members: readonly SdkResource[]): string {
  const p = new PythonPrinter([HEADER]);
  const names: string[] = [];
  for (const r of members) {
    const leaf = r.path[r.path.length - 1] ?? '';
    for (const [mode, suffix] of WRAPPERS) {
      const name = className(r, mode, suffix);
      p.importFrom(`.${leaf}`, name);
      names.push(name);
    }
  }
  for (const l of allListLines(names)) p.line(l);
  return p.toString();
}

function allListLines(names: readonly string[]): string[] {
  return ['__all__ = [', ...names.map((n) => `    ${pyString(n)},`), ']'];
}

/** Emits the types a method synthesizes: its inline response model and its params TypedDict. */
function methodTypes(
  files: Map<string, string>,
  exports: Map<string, Map<string, string>>,
  pkg: string,
  resource: SdkResource | undefined,
  method: SdkMethod,
): void {
  const ret = returnModel(resource, method);
  const add = (module: string, name: string, content: string): void => {
    files.set(modulePath(pkg, `types.${module}`), content);
    const parts = module.split('.');
    const dir = parts.slice(0, -1).join('.');
    const dirExports = exports.get(dir) ?? new Map<string, string>();
    dirExports.set(parts[parts.length - 1] ?? '', name);
    exports.set(dir, dirExports);
  };
  if (ret?.inline !== undefined) {
    add(
      ret.ref.module,
      ret.ref.name,
      ret.inline.kind === 'object'
        ? emitModelFile({ ref: ret.ref, type: ret.inline, prefixChildren: true })
        : emitAliasFile(ret.ref, ret.inline),
    );
  }
  if (methodParams(method).length > 0) {
    const ref = methodRef(resource, method, 'Params');
    add(ref.module, ref.name, emitParamsFile(resource, method));
  }
}

/** Every code file of the SDK. */
export function emitRootFiles(client: SdkClient): FileTree {
  const files = new Map<string, string>();
  const pkg = client.package.name;
  /** types package directory ('' for `types`) -> module -> exported name. */
  const exports = new Map<string, Map<string, string>>([['', new Map()]]);

  const shared = client.models.filter((d) => d.ref.module.startsWith('shared.'));
  if (shared.length > 0) exports.get('')?.set('shared', shared.map((d) => d.ref.name).join(', '));
  for (const decl of client.models) {
    files.set(
      modulePath(pkg, `types.${decl.ref.module}`),
      emitModelFile({
        ref: decl.ref,
        type: { description: decl.description, ...decl.type },
        prefixChildren: false,
      }),
    );
    if (!decl.ref.module.startsWith('shared.')) {
      const parts = decl.ref.module.split('.');
      const dir = parts.slice(0, -1).join('.');
      const dirExports = exports.get(dir) ?? new Map<string, string>();
      dirExports.set(parts[parts.length - 1] ?? '', decl.ref.name);
      exports.set(dir, dirExports);
    }
  }
  if (shared.length > 0) {
    const p = new PythonPrinter([HEADER]);
    for (const d of shared) {
      p.importFrom(`.${d.ref.module.slice('shared.'.length)}`, `${d.ref.name} as ${d.ref.name}`);
    }
    files.set(`src/${pkg}/types/shared/__init__.py`, p.toString());
  }

  for (const resource of allResources(client.resources)) {
    files.set(modulePath(pkg, resourceModule(resource)), emitResourceFile(client, resource));
    if (resource.subresources.length > 0) {
      files.set(
        modulePath(pkg, `resources.${resource.path.join('.')}.__init__`),
        resourcesInit([...resource.subresources, resource]),
      );
    }
    for (const method of resource.methods) methodTypes(files, exports, pkg, resource, method);
  }
  for (const method of client.clientMethods) {
    methodTypes(files, exports, pkg, undefined, method);
  }
  if (client.resources.length > 0) {
    files.set(modulePath(pkg, 'resources.__init__'), resourcesInit(client.resources));
  }

  for (const [dir, modules] of exports) {
    const init = dir === '' ? 'types.__init__' : `types.${dir}.__init__`;
    const p = new PythonPrinter([HEADER]);
    p.importFrom('__future__', 'annotations');
    for (const [module, names] of modules) {
      for (const name of names.split(', ')) p.importFrom(`.${module}`, `${name} as ${name}`);
    }
    files.set(modulePath(pkg, init), p.toString());
  }

  if (client.settings !== undefined) files.set(modulePath(pkg, '_client'), emitClientFile(client));
  return new Map([...files].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
}
