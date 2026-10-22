import type { Diagnostic } from '../diagnostic.ts';
import type { Config } from '../schema/config.ts';
import { closest } from '../schema/suggest.ts';

/**
 * Config editions. An edition pins the defaults Stainless applies to keys a config leaves out,
 * so a config keeps generating the same SDK when newer defaults ship. Every value below is cited
 * from a description in the public schema (converter/schema/stainless-config.schema.json); an
 * edition the schema lists without a description changes nothing Ferrule can observe and simply
 * inherits from the edition before it.
 */
export interface EditionDefaults {
  /**
   * OpenAPI normalizer version. `/properties/edition/anyOf/0` (2026-05-06): "Switches the
   * default normalizer to v2." There is no config key for it, so it is exposed through
   * `applyEdition(...).defaults` only.
   */
  readonly normalizer: 'v1' | 'v2';
}

/** Top-level `edition` values, oldest first, with what each changes. */
export const EDITIONS = {
  /** `/properties/edition/anyOf/4`: "the very first edition". */
  '2025-10-08': { normalizer: 'v1' },
  /** Listed without a description: no observable change. */
  '2025-10-10': { normalizer: 'v1' },
  /** Listed without a description: no observable change. */
  '2026-01-30': { normalizer: 'v1' },
  /** Listed without a description: no observable change. */
  '2026-02-23': { normalizer: 'v1' },
  /** `/properties/edition/anyOf/0`: "Switches the default normalizer to v2." */
  '2026-05-06': { normalizer: 'v2' },
} as const satisfies Record<string, EditionDefaults>;

export type Edition = keyof typeof EDITIONS;

export const EDITION_NAMES = Object.keys(EDITIONS) as Edition[];

/** Python target defaults that depend on `targets.python.edition`. */
export interface PythonEditionDefaults {
  /**
   * `targets.python.options.use_uv`. `/properties/targets/properties/python/properties/options/
   * properties/use_uv`: "In the python.2025-11-20 edition we switched the default package manger
   * from Rye ... to uv". Before it, generated SDKs use Rye (requirements.lock, as in the
   * OneBusAway oracle).
   */
  readonly useUv: boolean;
}

/** `targets.python.edition` values, oldest first. */
export const PYTHON_EDITIONS = {
  /** `/properties/targets/properties/python/properties/edition/anyOf/1`: "the very first edition". */
  'python.2025-10-08': { useUv: false },
  /** Switches the package manager from Rye to uv (see `useUv`). */
  'python.2025-11-20': { useUv: true },
} as const satisfies Record<string, PythonEditionDefaults>;

export type PythonEdition = keyof typeof PYTHON_EDITIONS;

/**
 * The edition `x-ferrule-config.edition` reports when the config has none.
 * Kept at its Phase 1 value so existing snapshots do not move; see `applyEdition` for the
 * defaults actually applied to a config without an edition (those of the first edition).
 */
export const DEFAULT_EDITION: Edition = '2025-10-10';

/** A config without `edition` predates editions, so it gets the first edition's defaults. */
export const IMPLICIT_EDITION: Edition = '2025-10-08';
export const IMPLICIT_PYTHON_EDITION: PythonEdition = 'python.2025-10-08';

export function isEdition(value: unknown): value is Edition {
  return typeof value === 'string' && Object.hasOwn(EDITIONS, value);
}

export function isPythonEdition(value: unknown): value is PythonEdition {
  return typeof value === 'string' && Object.hasOwn(PYTHON_EDITIONS, value);
}

export interface ResolvedDefaults extends EditionDefaults, PythonEditionDefaults {}

export interface EditionResult {
  /** A copy of the input with edition-dependent defaults written where the config left them out. */
  readonly config: Config;
  /** The edition whose defaults were applied (the implicit one when absent or unknown). */
  readonly edition: Edition;
  readonly pythonEdition: PythonEdition;
  /** Every edition-dependent default, including ones with no config key (`normalizer`). */
  readonly defaults: ResolvedDefaults;
  readonly diagnostics: readonly Diagnostic[];
}

/**
 * Fill edition-dependent defaults into a copy of `config`. Pure; the input is not mutated and
 * keys the config sets explicitly always win. Unknown editions are FER1101 (top level) and
 * FER1102 (`targets.python.edition`); their defaults fall back to the implicit edition.
 */
export function applyEdition(config: Config): EditionResult {
  const diagnostics: Diagnostic[] = [];
  const edition = pick(config.edition, isEdition, IMPLICIT_EDITION, () =>
    diagnostics.push(unknown('FER1101', '/edition', config.edition, EDITION_NAMES)),
  );
  const python = asRecord(config.targets?.python);
  const pythonEdition = pick(
    field(python, 'edition'),
    isPythonEdition,
    IMPLICIT_PYTHON_EDITION,
    () =>
      diagnostics.push(
        unknown(
          'FER1102',
          '/targets/python/edition',
          field(python, 'edition'),
          Object.keys(PYTHON_EDITIONS),
        ),
      ),
  );
  const defaults: ResolvedDefaults = { ...EDITIONS[edition], ...PYTHON_EDITIONS[pythonEdition] };

  const copy = structuredClone(config);
  const py = asRecord(copy.targets?.python) ?? {};
  const options = asRecord(field(py, 'options')) ?? {};
  const out: Config = {
    ...copy,
    targets: {
      ...copy.targets,
      python: {
        ...py,
        options: { ...options, use_uv: field(options, 'use_uv') ?? defaults.useUv },
      },
    },
  };
  return { config: out, edition, pythonEdition, defaults, diagnostics };
}

function pick<T extends string>(
  value: unknown,
  known: (v: unknown) => v is T,
  fallback: T,
  onUnknown: () => void,
): T {
  if (value === undefined) return fallback;
  if (known(value)) return value;
  onUnknown();
  return fallback;
}

function unknown(
  code: 'FER1101' | 'FER1102',
  pointer: string,
  value: unknown,
  names: readonly string[],
): Diagnostic {
  const near = typeof value === 'string' ? closest(value, names) : undefined;
  return {
    code,
    severity: 'error',
    message: `unknown edition ${JSON.stringify(value)}; Ferrule knows ${names.join(', ')}`,
    pointer,
    suggestion: near
      ? `Did you mean "${near}"?`
      : `Use one of ${names.join(', ')}, or upgrade Ferrule if this edition is newer.`,
  };
}

function asRecord(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

/** Read a key without tripping biome's useLiteralKeys against tsc's index-signature rule. */
function field(record: Record<string, unknown> | undefined, key: string): unknown {
  return record?.[key];
}
