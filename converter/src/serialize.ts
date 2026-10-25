/**
 * Set an own enumerable key. Plain assignment of `__proto__` would set the prototype and drop the
 * key, and spec JSON may contain any key.
 */
export function setOwn<T>(obj: Record<string, T>, key: string, value: T): void {
  Object.defineProperty(obj, key, { value, enumerable: true, writable: true, configurable: true });
}

/** Recursively sort object keys so equal values serialize to identical bytes. */
export function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (typeof value !== 'object' || value === null) return value;
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(value).sort()) {
    setOwn(out, key, sortKeys((value as Record<string, unknown>)[key]));
  }
  return out;
}

/** Deterministic JSON: sorted keys, 2-space indent, trailing newline. */
export function stableStringify(value: unknown): string {
  return `${JSON.stringify(sortKeys(value), null, 2)}\n`;
}
