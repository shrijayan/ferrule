/**
 * Python source printer with indentation and deterministic imports, sorted the way the oracle
 * SDKs' ruff/isort settings sort them: sections (future, stdlib, third party, local), plain
 * imports before `from` imports, modules and names by length then text, names grouped as
 * constants, classes, then functions and variables.
 */

/** Ruff's line length in Stainless-generated SDKs. */
export const LINE_LENGTH = 120;

/** Modules the oracle SDKs sort into the standard-library section. */
const STDLIB = new Set([
  'typing',
  'typing_extensions',
  'datetime',
  'os',
  'json',
  'inspect',
  'warnings',
  'functools',
]);

type Section = 0 | 1 | 2 | 3;

function sectionOf(module: string): Section {
  if (module === '__future__') return 0;
  if (module.startsWith('.')) return 3;
  return STDLIB.has(module.split('.')[0] ?? module) ? 1 : 2;
}

function byLength(a: string, b: string): number {
  return a.length - b.length || (a < b ? -1 : a > b ? 1 : 0);
}

function nameRank(name: string): number {
  const base = name.split(' ')[0] ?? name;
  if (base.length > 1 && base === base.toUpperCase() && /[A-Z]/.test(base)) return 0;
  if (/^[A-Z]/.test(base)) return 1;
  return 2;
}

function byNameType(a: string, b: string): number {
  return nameRank(a) - nameRank(b) || byLength(a, b);
}

export class PythonPrinter {
  private readonly lines: string[] = [];
  private depth = 0;
  private readonly plainImports = new Set<string>();
  private readonly fromImports = new Map<string, Set<string>>();
  private readonly exploded = new Set<string>();

  /** Prints `from module import (...)` one name per line (a magic trailing comma). */
  explode(module: string): this {
    this.exploded.add(module);
    return this;
  }

  constructor(
    private readonly header: readonly string[] = [],
    private readonly indentUnit = '    ',
  ) {}

  /** `import module` */
  importModule(module: string): this {
    this.plainImports.add(module);
    return this;
  }

  /** `from module import name`; `name` may be `Field as FieldInfo`. */
  importFrom(module: string, name: string): this {
    const names = this.fromImports.get(module) ?? new Set<string>();
    names.add(name);
    this.fromImports.set(module, names);
    return this;
  }

  /** One line at the current indentation; an empty string prints a blank line. */
  line(text = ''): this {
    this.lines.push(text === '' ? '' : this.indentUnit.repeat(this.depth) + text);
    return this;
  }

  /** The current indentation. */
  get indentation(): string {
    return this.indentUnit.repeat(this.depth);
  }

  /** A line printed as is, already indented. */
  raw(text: string): this {
    this.lines.push(text);
    return this;
  }

  /** Print `header` then run `body` one level deeper. */
  block(header: string, body: (p: this) => void): this {
    this.line(header);
    this.indented(body);
    return this;
  }

  /** Run `body` one level deeper; prints `pass` if it printed nothing. */
  indented(body: (p: this) => void): this {
    this.depth += 1;
    const before = this.lines.length;
    body(this);
    if (this.lines.length === before) this.line('pass');
    this.depth -= 1;
    return this;
  }

  private importLines(indent = ''): string[] {
    const sections: string[][] = [[], [], [], []];
    for (const m of [...this.plainImports].sort(byLength)) {
      sections[sectionOf(m)]?.push(`${indent}import ${m}`);
    }
    for (const m of [...this.fromImports.keys()].sort(byLength)) {
      const names = [...(this.fromImports.get(m) ?? [])].sort(byNameType);
      const one = `${indent}from ${m} import ${names.join(', ')}`;
      const text =
        one.length <= LINE_LENGTH && !this.exploded.has(m)
          ? one
          : [
              `${indent}from ${m} import (`,
              ...names.map((n) => `${indent}    ${n},`),
              `${indent})`,
            ].join('\n');
      sections[sectionOf(m)]?.push(text);
    }
    return sections.filter((s) => s.length > 0).flatMap((s) => [...s, '']);
  }

  /** The registered imports alone, at `indent`, for an `if TYPE_CHECKING:` block. */
  importBlock(indent: string): string[] {
    const lines = this.importLines(indent).join('\n').split('\n');
    while (lines.length > 0 && lines[lines.length - 1] === '') lines.pop();
    return lines;
  }

  /** Header, imports by section, then the body, ending with exactly one newline. */
  toString(): string {
    const head = this.header.length > 0 ? [...this.header, ''] : [];
    const all = [...head, ...this.importLines(), ...this.lines];
    return `${all.join('\n').trim()}\n`;
  }
}
