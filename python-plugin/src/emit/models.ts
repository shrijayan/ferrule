/** Pydantic model files under `types/`: one root model and the inline objects it contains. */
import type { SdkField, SdkObject, SdkRef, SdkType } from '../model/index.ts';
import { classSegment, singular, splitWords } from '../naming.ts';
import { LINE_LENGTH, PythonPrinter } from '../printer.ts';
import { type AnnotationContext, isNullable, toAnnotation } from '../type-mapper.ts';
import { HEADER, relImport, sortFields, wrap } from './common.ts';

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

/** Inline objects directly in `type`, and whether each sits in an array. */
function inlineObjects(type: SdkType, inArray = false): [SdkObject, boolean][] {
  switch (type.kind) {
    case 'object':
      return [[type, inArray]];
    case 'array':
      return inlineObjects(type.items, true);
    case 'map':
      return inlineObjects(type.values, inArray);
    case 'union':
      return type.variants.flatMap((v) => inlineObjects(v, inArray));
    default:
      return [];
  }
}

interface Named {
  readonly names: Map<SdkObject, string>;
  /** Pre-order, for `__all__`. */
  readonly preorder: SdkObject[];
  /** Post-order, the order classes are defined in. */
  readonly postorder: SdkObject[];
}

function nameObjects(file: ModelFile): Named {
  const named: Named = { names: new Map(), preorder: [], postorder: [] };
  const visit = (obj: SdkObject, name: string, isRoot: boolean): void => {
    named.names.set(obj, name);
    named.preorder.push(obj);
    const prefix = isRoot && !file.prefixChildren ? '' : name;
    for (const field of sortFields(obj.fields)) {
      for (const [child, inArray] of inlineObjects(field.type)) {
        if (named.names.has(child)) continue;
        const words = splitWords(field.wireName);
        visit(child, prefix + classSegment(inArray ? singular(words) : words), false);
      }
    }
    named.postorder.push(obj);
  };
  visit(file.type, file.ref.name, true);
  return named;
}

function fieldLines(
  p: PythonPrinter,
  field: SdkField,
  ctx: AnnotationContext,
  indent: string,
): string[] {
  let ann = toAnnotation(field.type, p, ctx);
  if (!field.required && !isNullable(field.type)) {
    p.importFrom('typing', 'Optional');
    ann = `Optional[${ann}]`;
  }
  const alias = field.pyName !== field.wireName;
  if (alias) p.importFrom('pydantic', 'Field as FieldInfo');
  const args = [
    ...(alias ? [`alias=${JSON.stringify(field.wireName)}`] : []),
    ...(field.required ? [] : ['default=None']),
  ].join(', ');
  const head = `${indent}${field.pyName}: ${ann}`;
  let lines: string[];
  if (!alias) lines = [field.required ? head : `${head} = None`];
  else if (`${head} = FieldInfo(${args})`.length <= LINE_LENGTH) {
    lines = [`${head} = FieldInfo(${args})`];
  } else lines = [`${head} = FieldInfo(`, `${indent}    ${args}`, `${indent})`];
  return [...lines, ...docstring(field.description, indent)];
}

/** A docstring: one line when it fits, otherwise wrapped with the quotes on their own line. */
export function docstring(text: string, indent: string): string[] {
  const body = wrap(text.trim());
  if (body.length === 0) return [];
  const one = `${indent}"""${body.join(' ')}"""`;
  if (body.length === 1 && one.length <= LINE_LENGTH) return [one];
  const [first, ...rest] = body;
  return [`${indent}"""${first}`, ...rest.map((l) => `${indent}${l}`), `${indent}"""`];
}

/** The `__all__` list, wrapped one name per line when it doesn't fit. */
export function allList(names: readonly string[]): string[] {
  const one = `__all__ = [${names.map((n) => JSON.stringify(n)).join(', ')}]`;
  if (one.length <= LINE_LENGTH) return [one];
  return ['__all__ = [', ...names.map((n) => `    ${JSON.stringify(n)},`), ']'];
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
  };
  const classes: string[][] = named.postorder.map((obj) => {
    const bases = obj.bases.map((b) => ctx.refName(b));
    if (bases.length === 0) {
      p.importFrom(relImport(here, '_models'), 'BaseModel');
      bases.push('BaseModel');
    }
    const fields = sortFields(obj.fields).map((f) => fieldLines(p, f, ctx, '    '));
    const body = fields.length === 0 ? [['    pass']] : fields;
    return [
      `class ${named.names.get(obj)}(${bases.join(', ')}):`,
      ...body.flatMap((f, i) => (i === 0 ? f : ['', ...f])),
    ];
  });
  for (const line of allList(named.preorder.map((o) => named.names.get(o) ?? ''))) p.line(line);
  for (const cls of classes) {
    p.line().line();
    for (const line of cls) p.line(line);
  }
  return p.toString();
}
