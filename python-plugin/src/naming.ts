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

/** Split an identifier like `listUsers`, `list_users` or `HTTPServer` into words. */
export function splitWords(identifier: string): string[] {
  return identifier
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .split(/[^A-Za-z0-9]+/)
    .filter((w) => w.length > 0);
}
