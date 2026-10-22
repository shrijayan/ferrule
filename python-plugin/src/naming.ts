/** Names are carried as word lists; these render them in Python conventions. */

const PY_KEYWORDS = new Set([
  'False',
  'None',
  'True',
  'and',
  'as',
  'assert',
  'async',
  'await',
  'break',
  'class',
  'continue',
  'def',
  'del',
  'elif',
  'else',
  'except',
  'finally',
  'for',
  'from',
  'global',
  'if',
  'import',
  'in',
  'is',
  'lambda',
  'nonlocal',
  'not',
  'or',
  'pass',
  'raise',
  'return',
  'try',
  'while',
  'with',
  'yield',
]);

function clean(words: readonly string[]): string[] {
  return words.map((w) => w.replace(/[^A-Za-z0-9]/g, '')).filter((w) => w.length > 0);
}

function guard(name: string): string {
  const safe = /^[0-9]/.test(name) ? `_${name}` : name;
  return PY_KEYWORDS.has(safe) ? `${safe}_` : safe;
}

/** `['list', 'Users']` -> `list_users` */
export function snakeCase(words: readonly string[]): string {
  return guard(
    clean(words)
      .map((w) => w.toLowerCase())
      .join('_'),
  );
}

/** `['user', 'ID']` -> `UserID`; existing inner capitals are preserved. */
export function pascalCase(words: readonly string[]): string {
  return guard(
    clean(words)
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(''),
  );
}

/** Words Stainless spells upper-case inside class names, such as `SituationURL`. */
const ACRONYMS = new Set([
  'id',
  'ids',
  'url',
  'uri',
  'api',
  'http',
  'https',
  'ip',
  'json',
  'uuid',
  'sdk',
]);

/** A class-name segment: PascalCase with acronyms upper-cased; `ids` becomes `IDs`. */
export function classSegment(words: readonly string[]): string {
  return clean(words)
    .map((w) => {
      const lower = w.toLowerCase();
      if (!ACRONYMS.has(lower)) return w.charAt(0).toUpperCase() + w.slice(1);
      return lower.endsWith('s') && lower !== 'https'
        ? `${lower.slice(0, -1).toUpperCase()}s`
        : lower.toUpperCase();
    })
    .join('');
}

/** The singular of the last word, as Stainless names array item classes: `agencies` -> `agency`. */
export function singular(words: readonly string[]): string[] {
  const out = [...words];
  const last = out.pop();
  if (last === undefined) return out;
  let s = last;
  if (/ies$/i.test(last)) s = `${last.slice(0, -3)}y`;
  else if (/(ss|us|is)$/i.test(last)) s = last;
  else if (/s$/i.test(last)) s = last.slice(0, -1);
  return [...out, s];
}

/** Split an identifier like `listUsers`, `list_users` or `HTTPServer` into words. */
export function splitWords(identifier: string): string[] {
  return identifier
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .split(/[^A-Za-z0-9]+/)
    .filter((w) => w.length > 0);
}
