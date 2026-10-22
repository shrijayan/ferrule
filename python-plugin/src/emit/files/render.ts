/**
 * The template language of the project files: `%%NAME%%` is replaced by `vars.NAME`, and a line
 * starting with `%%isort%%` is a one-line `from x import a, b` whose names are re-sorted the way
 * the oracle's ruff isort config (`length-sort = true`) orders them once the tokens are filled.
 */

export type TemplateVars = Readonly<Record<string, string>>;

/** Sort key of ruff isort with `length-sort`: length, then CONSTANTS, Classes, variables. */
export function isortCompare(a: string, b: string): number {
  if (a.length !== b.length) return a.length - b.length;
  const rank = (n: string): number => (/^[A-Z0-9_]+$/.test(n) ? 0 : /^[A-Z]/.test(n) ? 1 : 2);
  if (rank(a) !== rank(b)) return rank(a) - rank(b);
  return a < b ? -1 : a > b ? 1 : 0;
}

function sortImportLine(line: string): string {
  const m = /^from (\S+) import (.+)$/.exec(line);
  if (m === null) throw new Error(`%%isort%% on a line that is not a one-line import: ${line}`);
  const names = (m[2] as string).split(', ').sort(isortCompare);
  return `from ${m[1]} import ${names.join(', ')}`;
}

export function render(template: string, vars: TemplateVars): string {
  const filled = template.replace(/%%([A-Za-z_]+)%%/g, (token, name: string) => {
    if (name === 'isort') return token;
    const value = vars[name];
    if (value === undefined) throw new Error(`template token ${token} has no value`);
    return value;
  });
  return filled.replace(/^%%isort%%(.*)$/gm, (_, line: string) => sortImportLine(line));
}
