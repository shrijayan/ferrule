import type { Diagnostic, FerCode } from '../diagnostic.ts';
import {
  evaluateSegments,
  isRecord,
  JsonPathError,
  type Match,
  type PathSegment,
  parseJsonPath,
  pathKey,
  query,
  type Segment,
} from './jsonpath.ts';

/** One entry of a `target`/`to` argument: a JSONPath or a `matches_schema` query. */
export type TargetEntry =
  | string
  | { readonly matches_schema: string; readonly ignore?: string | readonly string[] | undefined };
export type Target = string | readonly TargetEntry[];

/** The mutable document a transform works on. `root` may be replaced by `update $`. */
export interface Doc {
  root: unknown;
}

export interface Resolved {
  readonly matches: Match[];
  readonly diagnostics: Diagnostic[];
}

export function error(
  code: FerCode,
  pointer: string,
  message: string,
  suggestion: string,
): Diagnostic {
  return { code, severity: 'error', message, pointer, suggestion };
}

function syntaxError(pointer: string, path: string, e: JsonPathError): Diagnostic {
  return error(
    'FER1202',
    pointer,
    `Invalid or unsupported JSONPath "${path}": ${e.message}`,
    'Write the path in the supported JSONPath Plus subset, e.g. $.components.schemas.User or $.paths..parameters[?(@.name == "id")].',
  );
}

/** Run `fn`, turning a JsonPathError into a FER1202 diagnostic. */
export function tryPath<T>(
  pointer: string,
  path: string,
  diagnostics: Diagnostic[],
  fn: () => T,
): T | undefined {
  try {
    return fn();
  } catch (e) {
    if (e instanceof JsonPathError) {
      diagnostics.push(syntaxError(pointer, path, e));
      return undefined;
    }
    throw e;
  }
}

export function noMatch(pointer: string, path: string): Diagnostic {
  return error(
    'FER1201',
    pointer,
    `Target "${path}" matches nothing in the OpenAPI spec.`,
    'Check the path against the current spec; if the spec was fixed upstream, delete this transform.',
  );
}

/** Structural equality of JSON values; object key order is ignored. */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a)) {
    return Array.isArray(b) && a.length === b.length && a.every((v, i) => deepEqual(v, b[i]));
  }
  if (isRecord(a) && isRecord(b)) {
    const ka = Object.keys(a);
    return (
      ka.length === Object.keys(b).length &&
      ka.every((k) => Object.hasOwn(b, k) && deepEqual(a[k], b[k]))
    );
  }
  return false;
}

function allNodes(value: unknown, path: PathSegment[], out: Match[]): void {
  out.push({ path: [...path], value });
  const keys: PathSegment[] = Array.isArray(value)
    ? value.map((_, i) => i)
    : isRecord(value)
      ? Object.keys(value)
      : [];
  for (const k of keys) {
    path.push(k);
    allNodes((value as Record<PathSegment, unknown>)[k], path, out);
    path.pop();
  }
}

function matchesSchema(
  doc: Doc,
  entry: Exclude<TargetEntry, string>,
  pointer: string,
  diagnostics: Diagnostic[],
): Match[] {
  const template = tryPath(`${pointer}/matches_schema`, entry.matches_schema, diagnostics, () =>
    query(doc.root, entry.matches_schema),
  );
  if (template === undefined) return [];
  const [only] = template;
  if (only === undefined || template.length !== 1) {
    diagnostics.push(
      error(
        'FER1207',
        `${pointer}/matches_schema`,
        `matches_schema "${entry.matches_schema}" must match exactly one node, found ${template.length}.`,
        'Point matches_schema at the single template schema, e.g. $.components.schemas.ErrorTemplate.',
      ),
    );
    return [];
  }
  const ignores = entry.ignore === undefined ? [] : [entry.ignore].flat();
  const ignored = new Set<string>();
  ignores.forEach((path, i) => {
    const at = typeof entry.ignore === 'string' ? `${pointer}/ignore` : `${pointer}/ignore/${i}`;
    const found = tryPath(at, path, diagnostics, () => query(doc.root, path)) ?? [];
    for (const m of found) ignored.add(pathKey(m.path));
  });
  const nodes: Match[] = [];
  allNodes(doc.root, [], nodes);
  const out = nodes.filter(
    (n) => !ignored.has(pathKey(n.path)) && isRecord(n.value) && deepEqual(n.value, only.value),
  );
  if (out.length === 0)
    diagnostics.push(noMatch(pointer, `matches_schema ${entry.matches_schema}`));
  return out;
}

function entries(target: Target, pointer: string): [TargetEntry, string][] {
  return typeof target === 'string'
    ? [[target, pointer]]
    : target.map((e, i): [TargetEntry, string] => [e, `${pointer}/${i}`]);
}

function dedupe(matches: readonly Match[]): Match[] {
  const seen = new Set<string>();
  return matches.filter((m) => {
    const k = pathKey(m.path);
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

/** Resolve a `target` argument to existing nodes. Every entry must match something. */
export function resolveTargets(doc: Doc, target: Target, pointer: string): Resolved {
  const diagnostics: Diagnostic[] = [];
  const matches: Match[] = [];
  for (const [entry, at] of entries(target, pointer)) {
    if (typeof entry !== 'string') {
      matches.push(...matchesSchema(doc, entry, at, diagnostics));
      continue;
    }
    const found = tryPath(at, entry, diagnostics, () => query(doc.root, entry));
    if (found === undefined) continue;
    if (found.length === 0) diagnostics.push(noMatch(at, entry));
    matches.push(...found);
  }
  return { matches: dedupe(matches), diagnostics };
}

/**
 * Resolve a destination (`to`) argument. A path ending in a plain name or index addresses that
 * child of every node its prefix selects, whether or not the child exists yet (the parent must).
 * Other paths, and matches_schema entries, address the existing nodes they select.
 */
export function resolveDestinations(doc: Doc, target: Target, pointer: string): Resolved {
  const diagnostics: Diagnostic[] = [];
  const matches: Match[] = [];
  for (const [entry, at] of entries(target, pointer)) {
    if (typeof entry !== 'string') {
      matches.push(...matchesSchema(doc, entry, at, diagnostics));
      continue;
    }
    const segments = tryPath(at, entry, diagnostics, () => parseJsonPath(entry));
    if (segments === undefined) continue;
    const found = destinations(doc.root, segments);
    if (found.length === 0) {
      diagnostics.push(
        error(
          'FER1208',
          at,
          `Destination "${entry}" has no existing parent object or array.`,
          'Create the parent first (for example with an earlier append or copy), or fix the path.',
        ),
      );
    }
    matches.push(...found);
  }
  return { matches: dedupe(matches), diagnostics };
}

function destinations(root: unknown, segments: readonly Segment[]): Match[] {
  const last = segments.at(-1);
  const [selector] = last?.selectors ?? [];
  const simple =
    last !== undefined &&
    !last.descendant &&
    last.selectors.length === 1 &&
    selector !== undefined &&
    (selector.kind === 'name' || selector.kind === 'index');
  if (!simple) return evaluateSegments(root, segments);
  const out: Match[] = [];
  for (const parent of evaluateSegments(root, segments.slice(0, -1))) {
    const v = parent.value;
    if (isRecord(v) && selector.kind === 'name') {
      out.push({ path: [...parent.path, selector.name], value: v[selector.name] });
    } else if (Array.isArray(v)) {
      const raw =
        selector.kind === 'index'
          ? selector.index
          : /^\d+$/.test(selector.name)
            ? Number(selector.name)
            : undefined;
      if (raw === undefined) continue;
      const i = raw < 0 ? v.length + raw : raw;
      if (i >= 0 && i <= v.length) out.push({ path: [...parent.path, i], value: v[i] });
    }
  }
  return out;
}

/**
 * Order matches so mutating them one by one never invalidates a later path: descendants before
 * their ancestors and higher array indices before lower ones; sibling keys sort by code unit, so
 * the order is total and deterministic.
 */
export function mutationOrder(matches: readonly Match[]): Match[] {
  const cmp = (a: readonly PathSegment[], b: readonly PathSegment[]): number => {
    const n = Math.min(a.length, b.length);
    for (let i = 0; i < n; i++) {
      const x = a[i];
      const y = b[i];
      if (x === y) continue;
      if (typeof x === 'number' && typeof y === 'number') return y - x;
      return String(x) < String(y) ? -1 : 1;
    }
    return b.length - a.length;
  };
  return [...matches].sort((p, q) => cmp(p.path, q.path));
}

/** The container holding `path`'s last segment, if it still exists. */
export function parentOf(doc: Doc, path: readonly PathSegment[]): unknown {
  let node = doc.root;
  for (const seg of path.slice(0, -1)) {
    if (Array.isArray(node) && typeof seg === 'number') node = node[seg];
    else if (isRecord(node) && typeof seg === 'string') node = node[seg];
    else return undefined;
  }
  return node;
}

export function getAt(doc: Doc, path: readonly PathSegment[]): unknown {
  if (path.length === 0) return doc.root;
  const parent = parentOf(doc, path);
  const key = path.at(-1);
  if (Array.isArray(parent) && typeof key === 'number') return parent[key];
  if (isRecord(parent) && typeof key === 'string') return parent[key];
  return undefined;
}

/** Set `path` to `value`; a no-op if its parent no longer exists. */
export function setAt(doc: Doc, path: readonly PathSegment[], value: unknown): void {
  if (path.length === 0) {
    doc.root = value;
    return;
  }
  const parent = parentOf(doc, path);
  const key = path.at(-1);
  if (Array.isArray(parent) && typeof key === 'number') parent[key] = value;
  else if (isRecord(parent) && typeof key === 'string') parent[key] = value;
}

/** Delete `path` (splicing arrays); a no-op if it no longer exists. */
export function deleteAt(doc: Doc, path: readonly PathSegment[]): void {
  const parent = parentOf(doc, path);
  const key = path.at(-1);
  if (Array.isArray(parent) && typeof key === 'number') parent.splice(key, 1);
  else if (isRecord(parent) && typeof key === 'string') delete parent[key];
}

export function isPrefix(prefix: readonly PathSegment[], path: readonly PathSegment[]): boolean {
  return prefix.length <= path.length && prefix.every((s, i) => s === path[i]);
}

export function describePath(path: readonly PathSegment[]): string {
  return `$${path.map((s) => (typeof s === 'number' ? `[${s}]` : `[${JSON.stringify(s)}]`)).join('')}`;
}
