/** Pydantic model files under `types/`: one root model and the inline objects it contains. */
import type { SdkField, SdkObject, SdkRef, SdkType } from '../model/index.ts';
import { LINE_LENGTH, PythonPrinter } from '../printer.ts';
import { type AnnotationContext, isNullable, toAnnotation } from '../type-mapper.ts';
import { escapeDocstring, HEADER, pyString, relImport, sortFields, wrap } from './common.ts';
import { call, formatExpr } from './format.ts';
import { type Decl, type Named, nameParams } from './params.ts';

export interface ModelFile {
  /** The root class and its module under `types`. */
  readonly ref: SdkRef;
  readonly type: SdkObject;
  /**
   * Whether nested classes carry the root's name: true for types synthesized from a method
   * (`AgenciesWithCoverageListResponseData`), false for models the config names (`Agency`
   * inside `References`).
   */
  readonly prefixChildren: boolean;
}

/** Names the root and its inline objects; see `nameParams` for the rules. */
function nameObjects(file: ModelFile): Named {
  const inner = nameParams(file.type.fields, file.prefixChildren ? file.ref.name : '');
  const root: Decl = { kind: 'class', name: file.ref.name, obj: file.type };
  inner.names.set(file.type, file.ref.name);
  return {
    names: inner.names,
    aliases: inner.aliases,
    preorder: [root, ...inner.preorder],
    postorder: [...inner.postorder, root],
  };
}

function fieldLines(
  p: PythonPrinter,
  field: SdkField,
  ctx: AnnotationContext,
  indent: string,
): string[] {
  let ann = toAnnotation(field.type, p, ctx);
  if (!field.required && !isNullable(field.type)) {
    // A plain union takes `None` as one more member: `Union[A, B, None]`.
    if (field.type.kind === 'union' && ann.startsWith('Union[')) ann = `${ann.slice(0, -1)}, None]`;
    else {
      p.importFrom('typing', 'Optional');
      ann = `Optional[${ann}]`;
    }
  }
  // Optional fields, and required nullable ones (as the oracles print them), default to None.
  const hasDefault = !field.required || isNullable(field.type);
  const alias = field.pyName !== field.wireName;
  if (alias) p.importFrom('pydantic', 'Field as FieldInfo');
  const args = [
    ...(alias ? [`alias=${pyString(field.wireName)}`] : []),
    ...(hasDefault ? ['default=None'] : []),
  ].join(', ');
  const head = `${indent}${field.pyName}: ${ann}`;
  let lines: string[];
  if (!alias) lines = [hasDefault ? `${head} = None` : head];
  else if (`${head} = FieldInfo(${args})`.length <= LINE_LENGTH) {
    lines = [`${head} = FieldInfo(${args})`];
  } else lines = [`${head} = FieldInfo(`, `${indent}    ${args}`, `${indent})`];
  return [...lines, ...docstring(field.description, indent)];
}

/** A docstring: one line when it fits, otherwise wrapped with the quotes on their own line. */
export function docstring(text: string, indent: string): string[] {
  const clean = escapeDocstring(text.trim());
  if (clean === '') return [];
  const ind = (lines: readonly string[]): string[] =>
    lines.map((l) => (l === '' ? '' : `${indent}${l}`));
  const body = wrap(clean, DOC_WIDTH);
  const one = `${indent}"""${body.join(' ')}"""`;
  if (body.length === 1 && one.length <= LINE_LENGTH) return [one];
  // Summary style when the first paragraph's first sentence is short and more text follows.
  const [para = '', ...paras] = clean.split(/\n\s*\n/);
  const flatPara = para.replace(/\s+/g, ' ');
  const m = /^(.+?[.!?])\s+(?=\S)(.*)$/.exec(flatPara);
  const summary = m?.[1] ?? flatPara;
  const restParas = [...(m?.[2] ? [m[2]] : []), ...paras];
  if (restParas.length > 0 && summary.length <= SUMMARY_MAX) {
    const [first = '', ...more] = wrap(summary, DOC_WIDTH);
    const rest = restParas.flatMap((p, i) => [...(i === 0 ? [] : ['']), ...wrap(p, DOC_WIDTH)]);
    return [`${indent}"""${first}`, ...ind(more), '', ...ind(rest), `${indent}"""`];
  }
  const all = [para, ...paras].flatMap((p, i) => [...(i === 0 ? [] : ['']), ...wrap(p, DOC_WIDTH)]);
  return [`${indent}"""`, ...ind(all), `${indent}"""`];
}

/** Wrap width of docstring text (measured on the oracles). */
const DOC_WIDTH = 80;
/** Longest first sentence still printed as a summary line. */
const SUMMARY_MAX = 82;

/** The `__all__` list, wrapped one name per line when it doesn't fit. */
export function allList(names: readonly string[]): string[] {
  const one = `__all__ = [${names.map((n) => pyString(n)).join(', ')}]`;
  if (one.length <= LINE_LENGTH) return [one];
  return ['__all__ = [', ...names.map((n) => `    ${pyString(n)},`), ']'];
}

/** Renders one model file, `types/<module>.py`. */
export function emitModelFile(file: ModelFile): string {
  const here = `types.${file.ref.module}`;
  const p = new PythonPrinter([HEADER]);
  const named = nameObjects(file);
  const ctx: AnnotationContext = {
    objectName: (obj) => named.names.get(obj) ?? '',
    refName: (ref) => {
      if (ref.module !== file.ref.module)
        p.importFrom(relImport(here, `types.${ref.module}`), ref.name);
      return ref.name;
    },
    unionName: (u) => named.aliases.get(u),
    runtime: (module, name) => p.importFrom(relImport(here, module), name),
  };
  const classes: string[][] = named.postorder.map((decl) => {
    if (decl.kind === 'alias') return aliasLines(p, decl, ctx);
    const obj = decl.obj;
    const bases = obj.bases.map((b) => ctx.refName(b));
    if (bases.length === 0) {
      p.importFrom(relImport(here, '_models'), 'BaseModel');
      bases.push('BaseModel');
    }
    const fields = sortFields(obj.fields).map((f) => fieldLines(p, f, ctx, '    '));
    const doc = docstring(obj.description ?? '', '    ');
    const body =
      fields.length === 0 && doc.length === 0
        ? [['    pass']]
        : [...(doc.length > 0 ? [doc] : []), ...fields];
    return [
      `class ${named.names.get(obj)}(${bases.join(', ')}):`,
      ...body.flatMap((f, i) => (i === 0 ? f : ['', ...f])),
    ];
  });
  for (const line of allList(named.preorder.map((d) => d.name))) p.line(line);
  for (const cls of classes) {
    p.line().line();
    for (const line of cls) p.line(line);
  }
  return p.toString();
}

/** A response that is not an object: `Name: TypeAlias = <annotation>`. */
export function emitAliasFile(ref: SdkRef, type: SdkType): string {
  const here = `types.${ref.module}`;
  const p = new PythonPrinter([HEADER]);
  p.importFrom('typing_extensions', 'TypeAlias');
  const ann = toAnnotation(type, p, {
    objectName: () => 'object',
    refName: (r) => {
      p.importFrom(relImport(here, `types.${r.module}`), r.name);
      return r.name;
    },
  });
  for (const l of allList([ref.name])) p.line(l);
  p.line();
  p.line(`${ref.name}: TypeAlias = ${ann}`);
  return p.toString();
}

/** `Name: TypeAlias = Union[...]`, `Annotated` with its discriminator when the spec names one. */
function aliasLines(
  p: PythonPrinter,
  decl: Decl & { kind: 'alias' },
  ctx: AnnotationContext,
): string[] {
  p.importFrom('typing_extensions', 'TypeAlias');
  p.importFrom('typing', 'Union');
  const u = decl.union;
  const variants = (u.kind === 'union' ? u.variants : []).map((v) =>
    toAnnotation(v, p, { ...ctx, unionName: () => undefined }),
  );
  const union = call('Union[', variants, false, ']');
  if (u.kind !== 'union' || u.discriminator === undefined) {
    return formatExpr(union, '', `${decl.name}: TypeAlias = `);
  }
  p.importFrom('typing_extensions', 'Annotated');
  ctx.runtime?.('_utils', 'PropertyInfo');
  const annotated = call(
    'Annotated[',
    [union, `PropertyInfo(discriminator=${pyString(u.discriminator)})`],
    false,
    ']',
  );
  return formatExpr(annotated, '', `${decl.name}: TypeAlias = `);
}
