/** `types/<resource>_<method>_params.py`: the TypedDict of a method's query parameters. */
import { FerruleError } from '../diagnostics.ts';
import type { SdkMethod, SdkResource } from '../model/index.ts';
import { PythonPrinter } from '../printer.ts';
import { toAnnotation } from '../type-mapper.ts';
import { HEADER, methodRef, relImport, sortFields } from './common.ts';
import { allList, docstring } from './models.ts';

export function emitParamsFile(resource: SdkResource, method: SdkMethod): string {
  const ref = methodRef(resource, method, 'Params');
  const here = `types.${ref.module}`;
  const p = new PythonPrinter([HEADER]);
  p.importFrom('__future__', 'annotations');
  p.importFrom('typing_extensions', 'TypedDict');
  const ctx = {
    objectName: (): string => {
      throw new FerruleError(
        'FER201',
        method.pointer,
        'an object-typed query parameter is not generated yet',
        'leave the operation out of the spike',
      );
    },
    refName: (r: { name: string; module: string }): string => {
      p.importFrom(relImport(here, `types.${r.module}`), r.name);
      return r.name;
    },
  };
  for (const l of allList([ref.name])) p.line(l);
  p.line().line();
  p.block(`class ${ref.name}(TypedDict, total=False):`, () => {
    sortFields(method.queryParams).forEach((q, i) => {
      if (q.wireName !== q.pyName) {
        throw new FerruleError(
          'FER201',
          method.pointer,
          `query parameter ${q.wireName} needs a PropertyInfo alias, which is not generated yet`,
          'leave the operation out of the spike',
        );
      }
      let ann = toAnnotation(q.type, p, ctx);
      if (q.required) {
        p.importFrom('typing_extensions', 'Required');
        ann = `Required[${ann}]`;
      }
      if (i > 0) p.line();
      p.line(`${q.pyName}: ${ann}`);
      for (const l of docstring(q.description, '')) p.line(l);
    });
  });
  return p.toString();
}
