/** A FER code such as `FER0001`. */
export type FerCode = `FER${number}`;

export type Severity = 'error' | 'warning' | 'info';

/** Every error carries a FER code, a JSON pointer into the spec or config, and a suggested fix. */
export interface Diagnostic {
  readonly code: FerCode;
  readonly severity: Severity;
  readonly message: string;
  /** RFC 6901 JSON pointer, e.g. `/resources/users/methods/list`. */
  readonly pointer: string;
  readonly suggestion: string;
}

/** Encode path segments as an RFC 6901 JSON pointer. */
export function toJsonPointer(segments: readonly (string | number)[]): string {
  return segments.map((s) => `/${String(s).replaceAll('~', '~0').replaceAll('/', '~1')}`).join('');
}
