/** Diagnostics with addresses: a FER code, a pointer into the spec, and a suggested fix. */

export type FerCode =
  /** An `x-ferrule-*` extension has the wrong shape. */
  | 'FER101'
  /** An `x-ferrule-*` extension the plugin does not know. */
  | 'FER102'
  /** A required `x-ferrule-*` extension is missing. */
  | 'FER103'
  /** The spec uses something the plugin cannot generate yet. */
  | 'FER201';

export class FerruleError extends Error {
  constructor(
    readonly code: FerCode,
    readonly pointer: string,
    readonly detail: string,
    readonly fix: string,
  ) {
    super(`${code} at ${pointer}: ${detail}. Fix: ${fix}`);
    this.name = 'FerruleError';
  }
}

/** JSON pointer of an operation: `#/paths/~1items/get`. */
export function operationPointer(path: string, verb: string): string {
  return `#/paths/${path.replace(/~/g, '~0').replace(/\//g, '~1')}/${verb.toLowerCase()}`;
}
