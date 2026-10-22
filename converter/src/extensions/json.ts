/** Untyped JSON readers shared by the extension builders. Pure. */

export type Json = Record<string, unknown>;

export const isObject = (v: unknown): v is Json =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

/** Typed field readers (keep biome's useLiteralKeys and tsc's index-signature rule apart). */
export const obj = (o: Json, k: string): Json => {
  const v = o[k];
  return isObject(v) ? v : {};
};
export const arr = (o: Json, k: string): unknown[] => {
  const v = o[k];
  return Array.isArray(v) ? v : [];
};
/** Any member, untyped. */
export const at = (o: Json, k: string): unknown => o[k];
export const str = (o: Json, k: string): string | undefined => {
  const v = o[k];
  return typeof v === 'string' ? v : undefined;
};
export const num = (o: Json, k: string): number | undefined => {
  const v = o[k];
  return typeof v === 'number' && Number.isFinite(v) ? v : undefined;
};
export const bool = (o: Json, k: string): boolean | undefined => {
  const v = o[k];
  return typeof v === 'boolean' ? v : undefined;
};

/** Resolve a local `#/...` $ref chain (cycle-safe); other nodes are returned as is. */
export function deref(spec: Json, node: unknown, seen: ReadonlySet<string> = new Set()): unknown {
  const ref = isObject(node) ? str(node, '$ref') : undefined;
  if (ref === undefined) return node;
  if (!ref.startsWith('#/') || seen.has(ref)) return undefined;
  let cur: unknown = spec;
  for (const raw of ref.slice(2).split('/')) {
    cur = isObject(cur) ? cur[raw.replaceAll('~1', '/').replaceAll('~0', '~')] : undefined;
  }
  return deref(spec, cur, new Set([...seen, ref]));
}

/** The schema of `name` among a schema's properties, following $ref and allOf. */
export function propertySchema(
  spec: Json,
  schema: unknown,
  name: string,
  depth = 0,
): unknown | undefined {
  const s = deref(spec, schema);
  if (!isObject(s) || depth > 32) return undefined;
  const props = obj(s, 'properties');
  if (Object.hasOwn(props, name)) return props[name];
  for (const part of arr(s, 'allOf')) {
    const found = propertySchema(spec, part, name, depth + 1);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** Property names of a schema, following $ref and allOf. */
export function propertyNames(spec: Json, schema: unknown, depth = 0): Set<string> {
  const out = new Set<string>();
  const s = deref(spec, schema);
  if (!isObject(s) || depth > 32) return out;
  for (const k of Object.keys(obj(s, 'properties'))) out.add(k);
  for (const part of arr(s, 'allOf')) {
    for (const k of propertyNames(spec, part, depth + 1)) out.add(k);
  }
  return out;
}

/** Whether a schema is an array, following $ref and allOf. */
export function isArraySchema(spec: Json, schema: unknown, depth = 0): boolean {
  const s = deref(spec, schema);
  if (!isObject(s) || depth > 32) return false;
  const type = at(s, 'type');
  if (type === 'array' || (Array.isArray(type) && type.includes('array'))) return true;
  if (at(s, 'items') !== undefined) return true;
  return arr(s, 'allOf').some((part) => isArraySchema(spec, part, depth + 1));
}

/** The element schema of an array schema, following $ref and allOf. */
export function itemsSchema(spec: Json, schema: unknown, depth = 0): unknown | undefined {
  const s = deref(spec, schema);
  if (!isObject(s) || depth > 32) return undefined;
  if (at(s, 'items') !== undefined) return at(s, 'items');
  for (const part of arr(s, 'allOf')) {
    const found = itemsSchema(spec, part, depth + 1);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** The lowest 2xx response of an operation, dereferenced, or undefined. */
export function successResponse(spec: Json, op: Json): Json | undefined {
  const responses = obj(op, 'responses');
  const code = Object.keys(responses)
    .filter((c) => c.startsWith('2'))
    .sort()[0];
  const resp = code === undefined ? undefined : deref(spec, responses[code]);
  return isObject(resp) ? resp : undefined;
}
