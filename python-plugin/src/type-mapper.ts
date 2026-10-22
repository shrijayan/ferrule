/** The one function mapping SdkType to a Python annotation (CLAUDE.md: type mapping lives here). */
import type { SdkObject, SdkRef, SdkType } from './model/index.ts';
import type { PythonPrinter } from './printer.ts';

/** How the file being printed spells inline objects and named models. */
export interface AnnotationContext {
  /** The class name the file gives an inline object. */
  objectName(obj: SdkObject): string;
  /** The name a named model has in this file; imports it if needed. */
  refName(ref: SdkRef): string;
}

const PRIMITIVES = {
  string: 'str',
  integer: 'int',
  number: 'float',
  boolean: 'bool',
  null: 'None',
} as const;

function pyLiteral(v: string | number | boolean): string {
  if (typeof v === 'string') return JSON.stringify(v);
  if (typeof v === 'boolean') return v ? 'True' : 'False';
  return String(v);
}

/** Map an SdkType to a Python annotation, registering needed imports on `printer`. */
export function toAnnotation(
  type: SdkType,
  printer: PythonPrinter,
  ctx: AnnotationContext,
): string {
  const ann = (t: SdkType): string => toAnnotation(t, printer, ctx);
  switch (type.kind) {
    case 'primitive':
      if (type.name === 'datetime' || type.name === 'date') {
        printer.importFrom('datetime', type.name);
        return type.name;
      }
      return PRIMITIVES[type.name];
    case 'unknown':
      return 'object';
    case 'array':
      printer.importFrom('typing', 'List');
      return `List[${ann(type.items)}]`;
    case 'map':
      printer.importFrom('typing', 'Dict');
      return `Dict[str, ${ann(type.values)}]`;
    case 'union': {
      const isNull = (t: SdkType): boolean => t.kind === 'primitive' && t.name === 'null';
      const rest = type.variants.filter((t) => !isNull(t));
      let inner: string;
      if (rest.length === 1 && rest[0] !== undefined) inner = ann(rest[0]);
      else {
        printer.importFrom('typing', 'Union');
        inner = `Union[${rest.map(ann).join(', ')}]`;
      }
      if (rest.length === type.variants.length) return inner;
      printer.importFrom('typing', 'Optional');
      return `Optional[${inner}]`;
    }
    case 'literal':
      printer.importFrom('typing_extensions', 'Literal');
      return `Literal[${type.values.map(pyLiteral).join(', ')}]`;
    case 'ref':
      return ctx.refName(type);
    case 'object':
      return ctx.objectName(type);
  }
}

/** True for `T | null`, which a field types as `Optional[T]` already. */
export function isNullable(type: SdkType): boolean {
  return (
    type.kind === 'union' && type.variants.some((t) => t.kind === 'primitive' && t.name === 'null')
  );
}
