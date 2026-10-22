import type { SdkType } from './model/index.ts';
import { pascalCase } from './naming.ts';
import type { PythonPrinter } from './printer.ts';

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
export function toAnnotation(type: SdkType, printer: PythonPrinter): string {
  switch (type.kind) {
    case 'primitive':
      return PRIMITIVES[type.name];
    case 'unknown':
      return 'object';
    case 'array':
      printer.importFrom('typing', 'List');
      return `List[${toAnnotation(type.items, printer)}]`;
    case 'map':
      printer.importFrom('typing', 'Dict');
      return `Dict[str, ${toAnnotation(type.values, printer)}]`;
    case 'optional':
      printer.importFrom('typing', 'Optional');
      return `Optional[${toAnnotation(type.inner, printer)}]`;
    case 'union':
      printer.importFrom('typing', 'Union');
      return `Union[${type.variants.map((v) => toAnnotation(v, printer)).join(', ')}]`;
    case 'literal':
      printer.importFrom('typing_extensions', 'Literal');
      return `Literal[${type.values.map(pyLiteral).join(', ')}]`;
    case 'ref': {
      const name = pascalCase(type.name);
      printer.importFrom(type.module, name);
      return name;
    }
  }
}
