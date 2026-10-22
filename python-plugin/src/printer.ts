/** Python source printer with indentation and deterministic import collection. */

export class PythonPrinter {
  private readonly lines: string[] = [];
  private depth = 0;
  private readonly plainImports = new Set<string>();
  private readonly fromImports = new Map<string, Set<string>>();

  constructor(private readonly indentUnit = '    ') {}

  /** `import module` */
  importModule(module: string): this {
    this.plainImports.add(module);
    return this;
  }

  /** `from module import name` */
  importFrom(module: string, name: string): this {
    const names = this.fromImports.get(module) ?? new Set<string>();
    names.add(name);
    this.fromImports.set(module, names);
    return this;
  }

  line(text = ''): this {
    this.lines.push(text === '' ? '' : this.indentUnit.repeat(this.depth) + text);
    return this;
  }

  /** Print `header` then run `body` one level deeper. */
  block(header: string, body: (p: this) => void): this {
    this.line(header);
    this.depth += 1;
    const before = this.lines.length;
    body(this);
    if (this.lines.length === before) this.line('pass');
    this.depth -= 1;
    return this;
  }

  /** Render imports (sorted) followed by the body, ending with exactly one newline. */
  toString(): string {
    const byName = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);
    const header = [
      ...[...this.plainImports].sort(byName).map((m) => `import ${m}`),
      ...[...this.fromImports.keys()]
        .sort(byName)
        .map(
          (m) => `from ${m} import ${[...(this.fromImports.get(m) ?? [])].sort(byName).join(', ')}`,
        ),
    ];
    const body = header.length > 0 ? [...header, '', ...this.lines] : this.lines;
    return `${body.join('\n').trimEnd()}\n`;
  }
}
