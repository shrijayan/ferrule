/** The one function mapping SdkType to a Python annotation (CLAUDE.md: type mapping lives here). */

import { pyString } from './emit/common.ts';
import type { SdkObject, SdkRef, SdkType } from './model/index.ts';
import type { PythonPrinter } from './printer.ts';

/** How the file being printed spells inline objects and named models. */
export interface AnnotationContext {
  /** The class name the file gives an inline object. */
  objectName(obj: SdkObject): string;
  /** The name a named model has in this file; imports it if needed. */
  refName(ref: SdkRef): string;
  /** Imports `name` from the package module `module` (such as `_types`). */
  runtime?(module: string, name: string): void;
  /**
   * Request types (method arguments, TypedDict params) rather than response models: arrays are
   * `Iterable` (`SequenceNotStr` for strings), dates also accept `str`.
   */
  readonly input?: boolean;
  /** Names the file's fields use, which a `from datetime import` would clash with. */
  readonly shadowed?: ReadonlySet<string>;
  /** The alias the file declares for a union, if any. */
  unionName?(union: SdkType): string | undefined;
}

const PRIMITIVES = {
  string: 'str',
  integer: 'int',
  number: 'float',
  boolean: 'bool',
  null: 'None',
} as const;

function pyLiteral(v: string | number | boolean): string {
  if (typeof v === 'string') return pyString(v);
  if (typeof v === 'boolean') return v ? 'True' : 'False';
  return String(v);
}

const isNull = (t: SdkType): boolean => t.kind === 'primitive' && t.name === 'null';

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
        let name: string = type.name;
        if (ctx.shadowed?.has(type.name)) {
          // A field of the same name would shadow the class: `import datetime`, `datetime.date`.
          printer.importModule('datetime');
          name = `datetime.${type.name}`;
        } else printer.importFrom('datetime', type.name);
        if (!ctx.input) return name;
        printer.importFrom('typing', 'Union');
        return `Union[str, ${name}]`;
      }
      return PRIMITIVES[type.name];
    case 'unknown':
      return 'object';
    case 'array':
      if (!ctx.input) {
        printer.importFrom('typing', 'List');
        return `List[${ann(type.items)}]`;
      }
      if (type.items.kind === 'primitive' && type.items.name === 'string') {
        ctx.runtime?.('_types', 'SequenceNotStr');
        return 'SequenceNotStr[str]';
      }
      printer.importFrom('typing', 'Iterable');
      return `Iterable[${ann(type.items)}]`;
    case 'map':
      printer.importFrom('typing', 'Dict');
      return `Dict[str, ${ann(type.values)}]`;
    case 'union': {
      const alias = ctx.unionName?.(type);
      if (alias !== undefined) return alias;
      // Forge does not keep member order (gap G17); the oracles list enums before plain types.
      const rank = (t: SdkType): number => (t.kind === 'literal' ? 0 : 1);
      const rest = type.variants.filter((t) => !isNull(t)).sort((a, b) => rank(a) - rank(b));
      const nullable = rest.length !== type.variants.length;
      if (rest.length > 1) {
        printer.importFrom('typing', 'Union');
        return `Union[${[...rest.map(ann), ...(nullable ? ['None'] : [])].join(', ')}]`;
      }
      const inner = rest[0] === undefined ? 'None' : ann(rest[0]);
      if (!nullable) return inner;
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
  return type.kind === 'union' && type.variants.some(isNull);
}
