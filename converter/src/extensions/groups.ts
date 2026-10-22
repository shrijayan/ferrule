/**
 * Forge group names for stainless.yml resource paths (ADR 0003, `$client` amendment). Pure.
 *
 * Forge drops an operation whose `x-fern-sdk-group-name` is empty, and splits names into words
 * (`$client` becomes the group `client`, merging with a real `client` resource). Root client
 * methods therefore go to a reserved group; the plugin reads `x-ferrule-resource` instead.
 */

/** The reserved Forge group of `$client` methods; Forge sees the words `ferrule client`. */
export const CLIENT_GROUP = '$ferrule_client';

/** The words Forge's `Name::new` splits a name into (letters/digits runs and case changes). */
export function forgeWords(name: string): string[] {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z])([A-Z][a-z])/g, '$1 $2')
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((w) => w !== '');
}

/** `x-fern-sdk-group-name` for a resource path: `$client` maps to `CLIENT_GROUP`. */
export function forgeGroupPath(path: readonly string[]): string[] {
  return path.map((seg, i) => (i === 0 && seg === '$client' ? CLIENT_GROUP : seg));
}

/** Whether a top-level resource name would land in the same Forge group as `$client`. */
export function collidesWithClientGroup(name: string): boolean {
  return name !== '$client' && forgeWords(name).join(' ') === forgeWords(CLIENT_GROUP).join(' ');
}
