/**
 * `types/<resource>_<method>_params.py`: the TypedDict of a method's query and body parameters,
 * followed by the TypedDicts (and union aliases) of the inline objects inside them.
 */
import type {
  SdkField,
  SdkMethod,
  SdkObject,
  SdkParam,
  SdkResource,
  SdkType,
} from '../model/index.ts';
import { classSegment, pascalCase, singular, splitWords } from '../naming.ts';
import { PythonPrinter } from '../printer.ts';
import { type AnnotationContext, toAnnotation } from '../type-mapper.ts';
import { HEADER, methodParams, methodRef, relImport, sortFields } from './common.ts';
import { call, printExpr } from './format.ts';
import { allList, docstring } from './models.ts';

/** A declaration in a params file: a TypedDict, or a union alias of TypedDicts. */
export type Decl =
  | { readonly kind: 'class'; readonly name: string; readonly obj: SdkObject }
  | { readonly kind: 'alias'; readonly name: string; readonly union: SdkType };

export interface Named {
  /** Inline object -> class name. */
  readonly names: Map<SdkObject, string>;
  /** Union of objects -> alias name. */
  readonly aliases: Map<SdkType, string>;
  /** Pre-order, for `__all__`. */
  readonly preorder: Decl[];
  /** Post-order, the order they are declared in. */
  readonly postorder: Decl[];
}

function objectVariants(t: SdkType): SdkObject[] | undefined {
  if (t.kind !== 'union') return undefined;
  const objects = t.variants.filter((v): v is SdkObject => v.kind === 'object');
  return objects.length > 1 ? objects : undefined;
}

/**
 * Names the inline objects of a method's parameters: a field's object is named after the field
 * (singular inside an array), prefixed by its parent's name below the top level; the variants
 * of a union of objects are named after the schemas they come from.
 */
export function nameParams(params: readonly (SdkParam | SdkField)[], rootPrefix = ''): Named {
  const named: Named = { names: new Map(), aliases: new Map(), preorder: [], postorder: [] };
  const visitType = (t: SdkType, base: string, inArray: boolean): void => {
    switch (t.kind) {
      case 'object':
        visitObject(t, base);
        return;
      case 'array':
        visitType(t.items, base, true);
        return;
      case 'map':
        visitType(t.values, base, inArray);
        return;
      case 'union': {
        const variants = objectVariants(t);
        if (variants === undefined) {
          for (const v of t.variants) visitType(v, base, inArray);
          return;
        }
        const alias: Decl = { kind: 'alias', name: base, union: t };
        named.aliases.set(t, base);
        named.preorder.push(alias);
        variants.forEach((v, i) => {
          const hint = v.hint === undefined ? `Variant${i}` : pascalCase(splitWords(v.hint));
          visitObject(v, `${base}${hint}`);
        });
        named.postorder.push(alias);
        return;
      }
      default:
        return;
    }
  };
  const visitObject = (obj: SdkObject, name: string): void => {
    if (named.names.has(obj)) return;
    named.names.set(obj, name);
    const decl: Decl = { kind: 'class', name, obj };
    named.preorder.push(decl);
    for (const f of sortFields(obj.fields)) visitField(f, name);
    named.postorder.push(decl);
  };
  const visitField = (f: SdkParam | SdkField, prefix: string): void => {
    const words = splitWords(f.wireName);
    const inArray = f.type.kind === 'array';
    visitType(f.type, prefix + classSegment(inArray ? singular(words) : words), inArray);
  };
  for (const p of sortFields(params)) visitField(p, rootPrefix);
  return named;
}

/** The annotation context of a params file or of method arguments using its names. */
export function paramsContext(
  named: Named,
  qualify: string,
  refName: AnnotationContext['refName'],
  runtime: (module: string, name: string) => void,
): AnnotationContext {
  return {
    input: true,
    objectName: (obj) => `${qualify}${named.names.get(obj) ?? 'object'}`,
    refName,
    runtime,
    unionName: (u) => {
      const n = named.aliases.get(u);
      return n === undefined ? undefined : `${qualify}${n}`;
    },
  };
}

function isDate(t: SdkType): boolean {
  return t.kind === 'primitive' && (t.name === 'datetime' || t.name === 'date');
}

function fieldAnnotation(p: PythonPrinter, f: SdkParam, ctx: AnnotationContext): string {
  let ann = toAnnotation(f.type, p, ctx);
  const info: string[] = [];
  if (f.pyName !== f.wireName) info.push(`alias=${JSON.stringify(f.wireName)}`);
  if (isDate(f.type)) info.push('format="iso8601"');
  if (info.length > 0) {
    p.importFrom('typing_extensions', 'Annotated');
    ctx.runtime?.('_utils', 'PropertyInfo');
    ann = `Annotated[${ann}, PropertyInfo(${info.join(', ')})]`;
  }
  if (f.required) {
    p.importFrom('typing_extensions', 'Required');
    ann = `Required[${ann}]`;
  }
  return ann;
}

export function emitParamsFile(resource: SdkResource | undefined, method: SdkMethod): string {
  const ref = methodRef(resource, method, 'Params');
  const here = `types.${ref.module}`;
  const p = new PythonPrinter([HEADER]);
  p.importFrom('__future__', 'annotations');
  p.importFrom('typing_extensions', 'TypedDict');
  const params = methodParams(method);
  const named = nameParams(params);
  const ctx: AnnotationContext = {
    ...paramsContext(
      named,
      '',
      (r) => {
        p.importFrom(relImport(here, `types.${r.module}`), r.name);
        return r.name;
      },
      (module, name) => p.importFrom(relImport(here, module), name),
    ),
    shadowed: new Set(params.map((q) => q.pyName)),
  };
  const fieldsOf = (fields: readonly SdkParam[]): void => {
    sortFields(fields).forEach((f, i) => {
      if (i > 0) p.line();
      p.line(`${f.pyName}: ${fieldAnnotation(p, f, ctx)}`);
      for (const l of docstring(f.description, '')) p.line(l);
    });
  };
  for (const l of allList([ref.name, ...named.preorder.map((d) => d.name)])) p.line(l);
  p.line().line();
  p.block(`class ${ref.name}(TypedDict, total=False):`, () => fieldsOf(params));
  for (const decl of named.postorder) {
    p.line().line();
    if (decl.kind === 'alias') {
      p.importFrom('typing_extensions', 'TypeAlias');
      p.importFrom('typing', 'Union');
      const variants = (decl.union.kind === 'union' ? decl.union.variants : []).map((v) =>
        toAnnotation(v, p, { ...ctx, unionName: () => undefined }),
      );
      printExpr(p, call('Union[', variants, false, ']'), `${decl.name}: TypeAlias = `);
      continue;
    }
    p.block(`class ${decl.name}(TypedDict, total=False):`, () => {
      const doc = docstring(decl.obj.description ?? '', '');
      for (const l of doc) p.line(l);
      if (doc.length > 0 && decl.obj.fields.length > 0) p.line();
      fieldsOf(decl.obj.fields);
    });
  }
  return p.toString();
}
