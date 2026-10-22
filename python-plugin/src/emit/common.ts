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

/**
 * A Python string literal for any text. JSON's escaping (quotes, backslashes, control characters
 * and lone surrogates as `\uXXXX`) is valid in a Python `"..."` literal, so no text can end it.
 */
export function pyString(text: string): string {
  return JSON.stringify(text);
}

/** An f-string literal holding `text` verbatim: braces are doubled so nothing interpolates. */
export function pyFString(text: string): string {
  return `f${pyString(text).replaceAll('{', '{{').replaceAll('}', '}}')}`;
}

/**
 * Makes spec text safe inside a `"""` docstring: backslashes doubled (as Stainless does) and
 * quotes that could close the docstring escaped, so a spec can't inject code into the SDK.
 */
export function escapeDocstring(text: string): string {
  return text.replaceAll('\\', '\\\\').replaceAll('"""', '\\"\\"\\"').replace(/"$/, '\\"');
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
 * The dotted module of a resource: `resources.stop`, or `resources.generations.generations` for
 * a resource with subresources, which becomes a package.
 */
export function resourceModule(resource: SdkResource): string {
  const base = ['resources', ...resource.path];
  if (resource.subresources.length > 0) base.push(resource.path[resource.path.length - 1] ?? '');
  return base.join('.');
}

/** Standard methods come first in this order; the rest follow by name (as the oracles order). */
const STANDARD = ['create', 'retrieve', 'update', 'list', 'delete'];

export function stainlessMethodOrder(methods: readonly SdkMethod[]): SdkMethod[] {
  const rank = (m: SdkMethod): number => {
    const i = STANDARD.indexOf(m.name);
    return i < 0 ? STANDARD.length : i;
  };
  return [...methods].sort(
    (a, b) => rank(a) - rank(b) || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0),
  );
}

/**
 * A synthesized type named after its method, with the resource's last word singular as the
 * oracles have it: `AgenciesWithCoverageListResponse`, `CardProgramListParams`. Types of a
 * subresource sit in its parent's directory: `types/generations/video_create_params.py`.
 * `$client` methods (no resource) are named after the method alone.
 */
export function methodRef(
  resource: SdkResource | undefined,
  method: SdkMethod,
  suffix: string,
): SdkRef {
  const words =
    resource === undefined
      ? []
      : singular(splitWords(resource.path[resource.path.length - 1] ?? ''));
  const dir = resource === undefined ? [] : resource.path.slice(0, -1);
  const leaf = [snakeCase(words), method.name, suffix.toLowerCase()].filter((w) => w !== '');
  return {
    kind: 'ref',
    name: `${classSegment(words)}${classSegment(splitWords(method.name))}${suffix}`,
    module: [...dir, leaf.join('_')].join('.'),
  };
}

/**
 * What a method returns: for a paginated method, one item; for a streaming one, one event;
 * otherwise the response. A named model stays a ref; an inline object becomes a synthesized
 * `<Resource><Method>Response`.
 */
export function returnModel(
  resource: SdkResource | undefined,
  method: SdkMethod,
): { readonly ref: SdkRef; readonly inline: SdkType | undefined } | undefined {
  const t = method.pagination?.item ?? method.streaming?.event ?? method.response;
  if (t === undefined) return undefined;
  if (t.kind === 'ref') return { ref: t, inline: undefined };
  return { ref: methodRef(resource, method, 'Response'), inline: t };
}

/** The keyword arguments of a method (query and body), in the oracle order. */
export function methodParams(method: SdkMethod): SdkParam[] {
  return sortFields([...method.queryParams, ...method.bodyParams]);
}

const EXTRA_ARGS: readonly [string, string][] = [
  ['extra_headers', 'Send extra headers'],
  ['extra_query', 'Add additional query parameters to the request'],
  ['extra_body', 'Add additional JSON properties to the request'],
  ['timeout', 'Override the client-level default timeout for this request, in seconds'],
];

/**
 * A method docstring: the description (else the summary), then, when the method takes any
 * parameter, an `Args:` section listing the documented keyword arguments and the extras.
 */
export function argsDocstring(method: SdkMethod, args: readonly SdkParam[] | undefined): string[] {
  const text = (method.description || method.summary).trim();
  const body = wrap(escapeDocstring(text));
  if (args === undefined) {
    if (body.length === 0) return [];
    if (body.length === 1) return [`"""${body[0]}"""`];
    return ['"""', ...body, '"""'];
  }
  const all: [string, string][] = [
    ...args
      .filter((a) => a.description.trim() !== '')
      .map((a): [string, string] => [a.pyName, escapeDocstring(a.description.trim())]),
    ...EXTRA_ARGS,
  ];
  const argLines = all.flatMap(([name, desc], i) => {
    const [first = '', ...rest] = wrap(desc);
    return [...(i === 0 ? [] : ['']), `  ${name}: ${first}`, ...rest.map((l) => `      ${l}`)];
  });
  return ['"""', ...body, ...(body.length > 0 ? [''] : []), 'Args:', ...argLines, '"""'];
}
