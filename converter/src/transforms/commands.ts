import type { Diagnostic } from '../diagnostic.ts';
import { isRecord, type Match, query } from './jsonpath.ts';
import {
  type Doc,
  deleteAt,
  describePath,
  error,
  getAt,
  isPrefix,
  mutationOrder,
  noMatch,
  parentOf,
  resolveDestinations,
  resolveTargets,
  setAt,
  type Target,
  tryPath,
} from './targets.ts';

/**
 * Every command checks first and mutates only when it found no errors, so a failing transform
 * leaves the document exactly as it was. `pointer` addresses the transform's `args` object.
 */
export type Command<A> = (doc: Doc, args: A, pointer: string) => Diagnostic[];

const clone = <T>(v: T): T => structuredClone(v);

function typeMismatch(pointer: string, m: Match, expected: string, command: string): Diagnostic {
  const actual = Array.isArray(m.value) ? 'array' : m.value === null ? 'null' : typeof m.value;
  return error(
    'FER1205',
    pointer,
    `${command} needs ${expected} at ${describePath(m.path)}, found ${actual}.`,
    `Point the target at ${expected}, or use a different command.`,
  );
}

const ROOT_SUGGESTION = 'The document root must stay an object; target a node inside it.';

export interface UpdateArgs {
  readonly target: Target;
  readonly value: unknown;
  readonly template?: boolean | undefined;
}

export const update: Command<UpdateArgs> = (doc, args, pointer) => {
  const { matches, diagnostics } = resolveTargets(doc, args.target, `${pointer}/target`);
  const template = args.template === true;
  if (template && (typeof args.value !== 'string' || !args.value.includes('{{value}}'))) {
    diagnostics.push(
      error(
        'FER1210',
        `${pointer}/value`,
        'A templated update needs a string value containing {{value}}.',
        'Include {{value}} where the current string goes, e.g. "[BETA] {{value}}", or drop template.',
      ),
    );
  }
  for (const m of matches) {
    if (template && typeof m.value !== 'string') {
      diagnostics.push(typeMismatch(`${pointer}/target`, m, 'a string', 'A templated update'));
    }
    if (m.path.length === 0 && !template && !isRecord(args.value)) {
      diagnostics.push(
        error('FER1205', `${pointer}/value`, 'update $ needs an object value.', ROOT_SUGGESTION),
      );
    }
  }
  if (diagnostics.length > 0) return diagnostics;
  for (const m of mutationOrder(matches)) {
    const next =
      template && typeof args.value === 'string'
        ? args.value.split('{{value}}').join(String(m.value))
        : clone(args.value);
    setAt(doc, m.path, next);
  }
  return [];
};

export interface AppendArgs {
  readonly target: Target;
  readonly value: unknown;
}

export const append: Command<AppendArgs> = (doc, args, pointer) => {
  const { matches, diagnostics } = resolveTargets(doc, args.target, `${pointer}/target`);
  for (const m of matches) {
    if (Array.isArray(m.value)) continue;
    if (!isRecord(m.value)) {
      diagnostics.push(typeMismatch(`${pointer}/target`, m, 'an array or object', 'append'));
    } else if (!isRecord(args.value)) {
      diagnostics.push(
        typeMismatch(
          `${pointer}/value`,
          { path: m.path, value: args.value },
          'an object value',
          'Appending to an object',
        ),
      );
    } else {
      for (const key of Object.keys(args.value)) {
        if (Object.hasOwn(m.value, key)) {
          diagnostics.push(
            error(
              'FER1206',
              `${pointer}/value/${key.replaceAll('~', '~0').replaceAll('/', '~1')}`,
              `Property "${key}" already exists at ${describePath(m.path)}.`,
              'The spec now has this property: delete the append, or use merge to override it on purpose.',
            ),
          );
        }
      }
    }
  }
  if (diagnostics.length > 0) return diagnostics;
  for (const m of mutationOrder(matches)) {
    const node = getAt(doc, m.path);
    if (Array.isArray(node)) {
      const items = Array.isArray(args.value) ? args.value : [args.value];
      node.push(...items.map(clone));
    } else if (isRecord(node) && isRecord(args.value)) {
      for (const [k, v] of Object.entries(args.value)) node[k] = clone(v);
    }
  }
  return [];
};

export interface MergeArgs {
  readonly target: Target;
  readonly value: Readonly<Record<string, unknown>>;
}

function deepMerge(into: Record<string, unknown>, from: Readonly<Record<string, unknown>>): void {
  for (const [k, v] of Object.entries(from)) {
    const existing = into[k];
    if (isRecord(existing) && isRecord(v) && Object.hasOwn(into, k)) deepMerge(existing, v);
    else into[k] = clone(v);
  }
}

export const merge: Command<MergeArgs> = (doc, args, pointer) => {
  const { matches, diagnostics } = resolveTargets(doc, args.target, `${pointer}/target`);
  for (const m of matches) {
    if (!isRecord(m.value))
      diagnostics.push(typeMismatch(`${pointer}/target`, m, 'an object', 'merge'));
  }
  if (diagnostics.length > 0) return diagnostics;
  for (const m of mutationOrder(matches)) {
    const node = getAt(doc, m.path);
    if (isRecord(node)) deepMerge(node, args.value);
  }
  return [];
};

export interface RemoveArgs {
  readonly target: Target;
  readonly keys?: readonly string[] | undefined;
}

export const remove: Command<RemoveArgs> = (doc, args, pointer) => {
  const { matches, diagnostics } = resolveTargets(doc, args.target, `${pointer}/target`);
  const { keys } = args;
  for (const m of matches) {
    if (keys === undefined) {
      if (m.path.length === 0) {
        diagnostics.push(
          error(
            'FER1211',
            `${pointer}/target`,
            'remove cannot delete the document root.',
            ROOT_SUGGESTION,
          ),
        );
      }
    } else if (!isRecord(m.value)) {
      diagnostics.push(typeMismatch(`${pointer}/target`, m, 'an object', 'remove with keys'));
    } else {
      keys.forEach((key, i) => {
        if (!Object.hasOwn(m.value as Record<string, unknown>, key)) {
          diagnostics.push(
            error(
              'FER1209',
              `${pointer}/keys/${i}`,
              `Key "${key}" is not present at ${describePath(m.path)}.`,
              'The spec no longer has this key: drop it from keys, or delete the transform.',
            ),
          );
        }
      });
    }
  }
  if (diagnostics.length > 0) return diagnostics;
  for (const m of mutationOrder(matches)) {
    if (keys === undefined) {
      deleteAt(doc, m.path);
      continue;
    }
    const node = getAt(doc, m.path);
    if (isRecord(node)) for (const key of keys) delete node[key];
  }
  return [];
};

/** Resolve `from` to exactly one node. */
function single(
  doc: Doc,
  from: string,
  pointer: string,
  diagnostics: Diagnostic[],
): Match | undefined {
  const found = tryPath(pointer, from, diagnostics, () => query(doc.root, from));
  if (found === undefined) return undefined;
  const [only] = found;
  if (only === undefined) {
    diagnostics.push(noMatch(pointer, from));
    return undefined;
  }
  if (found.length > 1) {
    diagnostics.push(
      error(
        'FER1207',
        pointer,
        `"${from}" must match exactly one node, found ${found.length}.`,
        'Narrow the path (avoid wildcards, recursive descent and filters) so it selects one node.',
      ),
    );
    return undefined;
  }
  return only;
}

export interface CopyArgs {
  readonly from: string;
  readonly to: Target;
}

export const copy: Command<CopyArgs> = (doc, args, pointer) => {
  const diagnostics: Diagnostic[] = [];
  const source = single(doc, args.from, `${pointer}/from`, diagnostics);
  const dest = resolveDestinations(doc, args.to, `${pointer}/to`);
  diagnostics.push(...dest.diagnostics);
  if (
    source !== undefined &&
    !isRecord(source.value) &&
    dest.matches.some((m) => m.path.length === 0)
  ) {
    diagnostics.push(
      error('FER1205', `${pointer}/to`, 'copy to $ needs an object source.', ROOT_SUGGESTION),
    );
  }
  if (source === undefined || diagnostics.length > 0) return diagnostics;
  for (const m of mutationOrder(dest.matches)) {
    const parent = parentOf(doc, m.path);
    const key = m.path.at(-1);
    if (Array.isArray(parent) && key === parent.length) parent.push(clone(source.value));
    else setAt(doc, m.path, clone(source.value));
  }
  return [];
};

export interface MoveArgs {
  readonly from: string;
  readonly to: string;
}

export const move: Command<MoveArgs> = (doc, args, pointer) => {
  const diagnostics: Diagnostic[] = [];
  const source = single(doc, args.from, `${pointer}/from`, diagnostics);
  const dest = resolveDestinations(doc, args.to, `${pointer}/to`);
  diagnostics.push(...dest.diagnostics);
  const [target] = dest.matches;
  if (dest.matches.length > 1) {
    diagnostics.push(
      error(
        'FER1207',
        `${pointer}/to`,
        `"${args.to}" must address exactly one destination, found ${dest.matches.length}.`,
        'Narrow the destination path so it selects one location.',
      ),
    );
  }
  if (source !== undefined && target !== undefined) {
    if (source.path.length === 0) {
      diagnostics.push(
        error(
          'FER1211',
          `${pointer}/from`,
          'move cannot relocate the document root.',
          ROOT_SUGGESTION,
        ),
      );
    } else if (isPrefix(source.path, target.path) && source.path.length < target.path.length) {
      diagnostics.push(
        error(
          'FER1211',
          `${pointer}/to`,
          `Cannot move ${describePath(source.path)} into itself (${describePath(target.path)}).`,
          'Pick a destination outside the node being moved.',
        ),
      );
    } else if (target.path.length === 0 && !isRecord(source.value)) {
      diagnostics.push(
        error('FER1205', `${pointer}/to`, 'move to $ needs an object source.', ROOT_SUGGESTION),
      );
    }
  }
  if (source === undefined || target === undefined || diagnostics.length > 0) return diagnostics;
  relocate(doc, source, target.path);
  return [];
};

function relocate(doc: Doc, source: Match, to: readonly (string | number)[]): void {
  const from = source.path;
  if (from.length === to.length && from.every((s, i) => s === to[i])) return;
  const fromParent = parentOf(doc, from);
  const sameParent = from.length === to.length && isPrefix(from.slice(0, -1), to);
  const fromKey = from.at(-1);
  const toKey = to.at(-1);
  if (
    sameParent &&
    isRecord(fromParent) &&
    typeof fromKey === 'string' &&
    typeof toKey === 'string'
  ) {
    // Rename in place so the key keeps its position.
    const entries = Object.entries(fromParent).filter(([k]) => k !== toKey);
    for (const k of Object.keys(fromParent)) delete fromParent[k];
    for (const [k, v] of entries) fromParent[k === fromKey ? toKey : k] = v;
    return;
  }
  if (
    sameParent &&
    Array.isArray(fromParent) &&
    typeof fromKey === 'number' &&
    typeof toKey === 'number'
  ) {
    const [item] = fromParent.splice(fromKey, 1);
    fromParent.splice(Math.min(toKey, fromParent.length), 0, item);
    return;
  }
  // Write the destination first: it is never inside the source, so the source path stays valid,
  // unless the destination is an ancestor of the source, which the write already replaced.
  const parent = parentOf(doc, to);
  if (Array.isArray(parent) && toKey === parent.length) parent.push(source.value);
  else setAt(doc, to, source.value);
  if (!isPrefix(to, from)) deleteAt(doc, from);
}
