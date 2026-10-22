/** Reads `x-ferrule-*` extensions (pagination, streaming, client) from a spec node. */

export type FerruleExtensions = Readonly<Record<string, unknown>>;

export function readExtensions(node: Readonly<Record<string, unknown>>): FerruleExtensions {
  return Object.fromEntries(
    Object.entries(node)
      .filter(([k]) => k.startsWith('x-ferrule-'))
      .map(([k, v]) => [k.slice('x-ferrule-'.length), v]),
  );
}
