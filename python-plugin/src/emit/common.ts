/** Helpers shared by the emitters. */
import type {
  SdkField,
  SdkMethod,
  SdkParam,
  SdkRef,
  SdkResource,
  SdkType,
} from '../model/index.ts';
import { classSegment, singular, snakeCase, splitWords } from '../naming.ts';

/** First line of every generated file (no vendor branding, per the plan). */
export const HEADER =
  '# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.';

/**
 * The relative import of `target` from module `from`, both dotted paths from the package root:
 * `relImport('types.foo', '_models')` is `.._models`.
 */
export function relImport(from: string, target: string): string {
  const pkg = from.split('.').slice(0, -1);
  const tgt = target.split('.');
  let common = 0;
  while (common < pkg.length && common < tgt.length - 1 && pkg[common] === tgt[common]) common++;
  return `.${'.'.repeat(pkg.length - common)}${tgt.slice(common).join('.')}`;
}

/** Greedy word wrap at `width` columns. */
export function wrap(text: string, width = 80): string[] {
  const lines: string[] = [];
  for (const para of text.split('\n')) {
    let cur = '';
    for (const word of para.split(/\s+/).filter((w) => w.length > 0)) {
      if (cur === '') cur = word;
      else if (cur.length + 1 + word.length <= width) cur += ` ${word}`;
      else {
        lines.push(cur);
        cur = word;
      }
    }
    lines.push(cur);
  }
  while (lines.length > 0 && lines[lines.length - 1] === '') lines.pop();
  return lines;
}

/**
 * Field order in the oracle SDKs: required before optional; within each, `id` first, then by
 * wire name ignoring case.
 */
export function sortFields<T extends SdkField | SdkParam>(fields: readonly T[]): T[] {
  const key = (f: T): string => f.wireName.toLowerCase();
  return [...fields].sort((a, b) => {
    if (a.required !== b.required) return a.required ? -1 : 1;
    if ((a.wireName === 'id') !== (b.wireName === 'id')) return a.wireName === 'id' ? -1 : 1;
    return key(a) < key(b) ? -1 : key(a) > key(b) ? 1 : 0;
  });
}

/** `AgenciesWithCoverage` for `['agencies_with_coverage']`. */
export function resourceClassBase(resource: SdkResource): string {
  return classSegment(splitWords(resource.path[resource.path.length - 1] ?? ''));
}

/**
 * A synthesized type named after its method, with the resource's last word singular as the
 * oracles have it: `AgenciesWithCoverageListResponse`, `CardProgramListParams`.
 */
export function methodRef(resource: SdkResource, method: SdkMethod, suffix: string): SdkRef {
  const words = singular(splitWords(resource.path[resource.path.length - 1] ?? ''));
  return {
    kind: 'ref',
    name: `${classSegment(words)}${classSegment(splitWords(method.name))}${suffix}`,
    module: `${snakeCase(words)}_${method.name}_${suffix.toLowerCase()}`,
  };
}

/**
 * What a method returns: for a paginated method, one item; otherwise the response. A named
 * model stays a ref; an inline object becomes a synthesized `<Resource><Method>Response`.
 */
export function returnModel(
  resource: SdkResource,
  method: SdkMethod,
): { readonly ref: SdkRef; readonly inline: SdkType | undefined } | undefined {
  const t = method.pagination?.item ?? method.response;
  if (t === undefined) return undefined;
  if (t.kind === 'ref') return { ref: t, inline: undefined };
  return { ref: methodRef(resource, method, 'Response'), inline: t };
}
