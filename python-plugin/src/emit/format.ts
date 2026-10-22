/**
 * Line splitting the way ruff format (black style) splits the bracketed expressions the emitters
 * print: a statement stays on one line when it fits in 120 columns and nothing inside it has a
 * magic trailing comma; otherwise its last bracket is split, the contents go on one indented line
 * when they fit there, and otherwise one item per line with a trailing comma, each split the same
 * way. The plugin is sandboxed, so this replaces running ruff (Forge gaps G8, G15).
 */
import { LINE_LENGTH, type PythonPrinter } from '../printer.ts';

/** An atom (printed as is) or a bracketed list: `open` + items joined by `, ` + `close`. */
export type Expr = string | Bracket;

export interface Bracket {
  /** Everything up to and including the opening bracket, such as `self._get(`. */
  readonly open: string;
  readonly items: readonly Expr[];
  /** The closing bracket, such as `)`. */
  readonly close: string;
  /** A trailing comma after the last item, which keeps the bracket split. */
  readonly magic?: boolean;
}

export function call(open: string, items: readonly Expr[], magic = false, close = ')'): Bracket {
  return { open, items, close, magic };
}

function hasMagic(e: Expr): boolean {
  return typeof e !== 'string' && ((e.magic ?? false) || e.items.some(hasMagic));
}

/** The expression on one line. */
export function flat(e: Expr): string {
  if (typeof e === 'string') return e;
  return `${e.open}${e.items.map(flat).join(', ')}${e.close}`;
}

const fits = (line: string): boolean => line.length <= LINE_LENGTH;

/** The lines of `prefix + e + suffix` at `indent`, split as ruff would. */
export function formatExpr(e: Expr, indent: string, prefix = '', suffix = ''): string[] {
  if (typeof e === 'string') return [`${indent}${prefix}${e}${suffix}`];
  const one = `${indent}${prefix}${flat(e)}${suffix}`;
  if (!hasMagic(e) && fits(one)) return [one];
  const inner = `${indent}    `;
  const joined = `${inner}${e.items.map(flat).join(', ')}`;
  const body =
    !hasMagic(e) && e.items.length > 0 && fits(joined)
      ? [joined]
      : e.items.length === 1 && typeof e.items[0] === 'string' && !e.magic
        ? [`${inner}${e.items[0]}`]
        : e.items.flatMap((item) => formatExpr(item, inner, '', ','));
  return [`${indent}${prefix}${e.open}`, ...body, `${indent}${e.close}${suffix}`];
}

/** Prints `prefix + e + suffix` at the printer's indentation. */
export function printExpr(p: PythonPrinter, e: Expr, prefix = '', suffix = ''): void {
  for (const l of formatExpr(e, p.indentation, prefix, suffix)) p.raw(l);
}
