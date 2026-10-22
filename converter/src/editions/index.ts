/** Stainless config editions Ferrule understands, oldest first. */
export const EDITIONS = ['2025-10-10'] as const;

export type Edition = (typeof EDITIONS)[number];

export const DEFAULT_EDITION: Edition = '2025-10-10';

export function isEdition(value: unknown): value is Edition {
  return typeof value === 'string' && (EDITIONS as readonly string[]).includes(value);
}
