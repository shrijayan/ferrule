/**
 * `x-ferrule-pagination` (ADR 0003): normalizes the `pagination` schemes of stainless.yml and
 * matches them against operations. Pure.
 */
import { type Diagnostic, toJsonPointer } from '../diagnostic.ts';
import {
  arr,
  at,
  bool,
  deref,
  isArraySchema,
  isObject,
  itemsSchema,
  type Json,
  obj,
  propertyNames,
  propertySchema,
  str,
  successResponse,
} from './json.ts';

/** Every pagination `type` the public stainless.yml schema allows. */
export const PAGINATION_SCHEMES = [
  'cursor',
  'cursor_id',
  'cursor_url',
  'fake_page',
  'offset',
  'page_number',
] as const;

export type PaginationScheme = (typeof PAGINATION_SCHEMES)[number];

/**
 * `x-ferrule-pagination`. Field maps run spec field path -> purpose. A nested response field is a
 * dotted path (`meta.next_cursor`); `[]` steps into array items (`data.[].id`).
 */
export interface FerrulePagination {
  readonly scheme: PaginationScheme;
  readonly page: string;
  readonly paramLocation: 'query' | 'body';
  readonly continueOnEmptyItems: boolean;
  readonly request: Record<string, string>;
  readonly response: Record<string, string>;
  /** Response header name -> purpose, for properties read `from_header`. Only when present. */
  readonly headers?: Record<string, string>;
}

/** Name given to the single-object `pagination:` form, which has no `name`. */
export const UNNAMED_PAGE = 'page';

const isScheme = (v: unknown): v is PaginationScheme =>
  (PAGINATION_SCHEMES as readonly unknown[]).includes(v);

function purposeOf(def: Json): { purpose?: string; fromHeader?: string } {
  const prop = obj(def, 'x-stainless-pagination-property');
  const purpose = str(prop, 'purpose');
  const fromHeader = str(prop, 'from_header');
  return {
    ...(purpose === undefined ? {} : { purpose }),
    ...(fromHeader === undefined ? {} : { fromHeader }),
  };
}

interface Flat {
  readonly body: Record<string, string>;
  readonly headers: Record<string, string>;
}

const emptyFlat = (): Flat => ({ body: {}, headers: {} });

function merge(into: Flat, from: Flat): void {
  Object.assign(into.body, from.body);
  Object.assign(into.headers, from.headers);
}

/**
 * Flatten configured fields. A field is listed when it has a purpose or has no annotated
 * descendants; the fields under an annotated object or array item are listed by path.
 * Returns whether any field carries `x-stainless-pagination-property`.
 */
function flatten(fields: Json, prefix: string, out: Flat, depth = 0): boolean {
  let annotated = false;
  for (const [name, raw] of Object.entries(fields)) {
    const def = isObject(raw) ? raw : {};
    const path = prefix === '' ? name : `${prefix}.${name}`;
    const { purpose, fromHeader } = purposeOf(def);
    const children = emptyFlat();
    const childAnnotated =
      depth < 16 &&
      [
        flatten(obj(def, 'properties'), path, children, depth + 1),
        flatten(obj(obj(def, 'items'), 'properties'), `${path}.[]`, children, depth + 1),
      ].includes(true);
    if (childAnnotated) merge(out, children);
    if (fromHeader !== undefined) out.headers[fromHeader] = purpose ?? name;
    else if (purpose !== undefined || !childAnnotated) out.body[path] = purpose ?? name;
    if (purpose !== undefined || fromHeader !== undefined || childAnnotated) annotated = true;
  }
  return annotated;
}

/** A configured scheme, ready to match. */
export interface Scheme {
  readonly extension: FerrulePagination;
  /** Top-level request field names. */
  readonly requestFields: readonly string[];
}

function toScheme(raw: Json, name: string, type: PaginationScheme): Scheme {
  const request = emptyFlat();
  const response = emptyFlat();
  flatten(obj(raw, 'request'), '', request);
  flatten(obj(raw, 'response'), '', response);
  const headers = { ...request.headers, ...response.headers };
  return {
    extension: {
      scheme: type,
      page: name,
      paramLocation: str(raw, 'param_location') === 'body' ? 'body' : 'query',
      continueOnEmptyItems: bool(raw, 'continue_on_empty_items') ?? false,
      request: request.body,
      response: response.body,
      ...(Object.keys(headers).length === 0 ? {} : { headers }),
    },
    requestFields: Object.keys(obj(raw, 'request')),
  };
}

/** Normalize `pagination` (a list, or the single-object form) into schemes, in config order. */
export function readSchemes(config: Json): { schemes: Scheme[]; diagnostics: Diagnostic[] } {
  const raw = at(config, 'pagination');
  const list = Array.isArray(raw) ? raw : isObject(raw) ? [raw] : [];
  const single = !Array.isArray(raw);
  const schemes: Scheme[] = [];
  const diagnostics: Diagnostic[] = [];
  list.forEach((entry, i) => {
    const p = isObject(entry) ? entry : {};
    const pointer = toJsonPointer(single ? ['pagination'] : ['pagination', i]);
    const type = at(p, 'type');
    if (!isScheme(type)) {
      diagnostics.push({
        code: 'FER1501',
        severity: 'error',
        message: `Pagination type ${JSON.stringify(type)} is not one of ${PAGINATION_SCHEMES.join(', ')}.`,
        pointer: `${pointer}/type`,
        suggestion: `Set type to one of: ${PAGINATION_SCHEMES.join(', ')}.`,
      });
      return;
    }
    schemes.push(toScheme(p, str(p, 'name') ?? UNNAMED_PAGE, type));
  });
  return { schemes, diagnostics };
}

/** Walk a dotted field path (`[]` = array items) through a schema; undefined when absent. */
function resolvePath(spec: Json, schema: unknown, path: string): unknown | undefined {
  let cur: unknown = schema;
  for (const seg of path.split('.')) {
    cur = seg === '[]' ? itemsSchema(spec, cur) : propertySchema(spec, cur, seg);
    if (cur === undefined) return undefined;
  }
  return cur;
}

/** The request and response shape of an operation that pagination is matched against. */
export interface OperationShape {
  readonly query: ReadonlySet<string>;
  readonly body: ReadonlySet<string>;
  readonly responseSchema: unknown;
}

function jsonSchema(content: Json): unknown {
  return at(obj(content, 'application/json'), 'schema');
}

export function operationShape(spec: Json, pathItem: Json, op: Json): OperationShape {
  const query = new Set<string>();
  for (const p of [...arr(pathItem, 'parameters'), ...arr(op, 'parameters')]) {
    const param = deref(spec, p);
    const name = isObject(param) && str(param, 'in') === 'query' ? str(param, 'name') : undefined;
    if (name !== undefined) query.add(name);
  }
  const requestBody = deref(spec, at(op, 'requestBody'));
  const body = isObject(requestBody)
    ? propertyNames(spec, jsonSchema(obj(requestBody, 'content')))
    : new Set<string>();
  const resp = successResponse(spec, op);
  return {
    query,
    body,
    responseSchema: resp === undefined ? undefined : jsonSchema(obj(resp, 'content')),
  };
}

/**
 * Stainless's matching rule: every request field is a param in the scheme's location, and every
 * response field path exists in the 2xx JSON response, with the `items` field an array.
 */
export function matches(spec: Json, scheme: Scheme, shape: OperationShape): boolean {
  const params = scheme.extension.paramLocation === 'body' ? shape.body : shape.query;
  if (!scheme.requestFields.every((k) => params.has(k))) return false;
  return Object.entries(scheme.extension.response).every(([path, purpose]) => {
    const field = resolvePath(spec, shape.responseSchema, path);
    return field !== undefined && (purpose !== 'items' || isArraySchema(spec, field));
  });
}

const specificity = (s: Scheme): number =>
  s.requestFields.length + Object.keys(s.extension.response).length;

/** The most specific matching scheme (most fields), earliest in config order on ties. */
export function bestMatch(
  spec: Json,
  schemes: readonly Scheme[],
  shape: OperationShape,
): Scheme | undefined {
  let best: Scheme | undefined;
  for (const s of schemes) {
    if (matches(spec, s, shape) && (best === undefined || specificity(s) > specificity(best))) {
      best = s;
    }
  }
  return best;
}
