/** api.md: every resource's types and methods, as the oracle SDKs list them. */
import type { FilesMethod, FilesResource, ProjectFilesInput } from './input.ts';

/** `from <module> import a, b`, parenthesized one per line past `width` columns. */
export function importLine(module: string, names: readonly string[], width: number): string {
  const line = `from ${module} import ${names.join(', ')}`;
  if (line.length <= width) return line;
  return `from ${module} import (\n${names.map((n) => `    ${n},\n`).join('')})`;
}

const filePath = (pkg: string, module: string): string =>
  `src/${pkg}/${module.replace(/\./g, '/')}.py`;

function signature(pkg: string, m: FilesMethod): string {
  const path = m.params.filter((p) => p.location === 'path').map((p) => p.pyName);
  const args: string[] = [];
  if (path.length > 0) args.push(path[path.length - 1] as string);
  if (path.length > 1) args.push('\\*', ...path.slice(0, -1));
  if (m.paramsModule !== undefined) {
    args.push(`\\*\\*<a href="${filePath(pkg, m.paramsModule)}">params</a>`);
  }
  return args.join(', ');
}

function methodLine(pkg: string, accessor: string, r: FilesResource, m: FilesMethod): string {
  const ret =
    m.response === undefined
      ? 'None'
      : `<a href="./${filePath(pkg, m.response.module)}">${m.response.name}</a>`;
  return `- <code title="${m.httpMethod} ${m.path}">client.${accessor}.<a href="./${filePath(pkg, r.module)}">${m.name}</a>(${signature(pkg, m)}) -> ${ret}</code>`;
}

function section(pkg: string, r: FilesResource, parent: string, depth: number): string[] {
  const accessor = parent === '' ? r.name : `${parent}.${r.name}`;
  const out = [`${'#'.repeat(depth)} ${r.title}`];
  if (r.types.length > 0) {
    out.push(
      'Types:',
      `\`\`\`python\n${importLine(`${pkg}.${r.typesModule}`, r.types, 120)}\n\`\`\``,
    );
  }
  if (r.methods.length > 0) {
    out.push('Methods:', r.methods.map((m) => methodLine(pkg, accessor, r, m)).join('\n'));
  }
  const blocks = [out.join('\n\n')];
  for (const sub of r.subresources) blocks.push(...section(pkg, sub, accessor, depth + 1));
  return blocks;
}

export function emitApiMd(input: ProjectFilesInput): string {
  const pkg = input.packageName;
  const blocks: string[] = [];
  if (input.sharedTypes.length > 0) {
    blocks.push(
      `# Shared Types\n\n\`\`\`python\n${importLine(`${pkg}.types`, input.sharedTypes, 120)}\n\`\`\``,
    );
  }
  for (const r of input.resources) blocks.push(...section(pkg, r, '', 1));
  return `${blocks.join('\n\n')}\n`;
}
