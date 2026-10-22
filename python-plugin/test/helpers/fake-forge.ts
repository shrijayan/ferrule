/**
 * Builds an object shaped like Forge's plugin interface from an OpenAPI JSON document, so the
 * adapter and emitters can run in-process. It covers what the fixtures use, following what the
 * pinned Forge hands plugins (checked against it by the CLI golden test); it is test-only.
 */
import type {
  Field,
  Forge,
  Group,
  Json,
  Location,
  Method,
  Model,
  Name,
  Response,
  Type,
} from '@cloudflare/forge/plugin';

type Schema = { readonly [key: string]: Json };
type Doc = {
  info: { version: string; title: string };
  servers?: { url: string }[];
  paths: Record<string, Record<string, Schema>>;
  components?: { schemas?: Record<string, Schema> };
};

export function name(text: string): Name {
  const words = text
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .split(/[^A-Za-z0-9]+/)
    .filter((w) => w.length > 0)
    .map((w) => w.toLowerCase());
  const cap = (w: string): string => w.charAt(0).toUpperCase() + w.slice(1);
  const n: Name = {
    words,
    isEmpty: words.length === 0,
    camel: () => words.map((w, i) => (i === 0 ? w : cap(w))).join(''),
    pascal: () => words.map(cap).join(''),
    snake: () => words.join('_'),
    kebab: () => words.join('-'),
    equals: (o) => o.words.join(' ') === words.join(' '),
    toString: () => words.join(' '),
  };
  return n;
}

/** A string member, or an empty string. */
function text(s: Schema, key: string): string {
  const v = s[key];
  return typeof v === 'string' ? v : '';
}

/** Reads a member without tripping noPropertyAccessFromIndexSignature or useLiteralKeys. */
function at(s: Schema, key: string): Json | undefined {
  return s[key];
}

const NULL: Type = { kind: 'scalar', scalar: 'null' };

export function fakeForge(doc: Doc): Forge {
  const schemas = doc.components?.schemas ?? {};
  const models = new Map<string, Model>();

  const typeOf = (s: Schema): Type => {
    const t = baseType(s);
    return at(s, 'nullable') === true
      ? { kind: 'union', members: [t, NULL], exclusive: true, discriminator: undefined }
      : t;
  };

  const modelOf = (schemaName: string): Model => {
    const existing = models.get(schemaName);
    if (existing !== undefined) return existing;
    const schema = schemas[schemaName] ?? {};
    let type: Type | undefined;
    const m: Model = {
      schemaName,
      name: name(schemaName),
      group: undefined,
      description: text(schema, 'description'),
      deprecated: false,
      get type(): Type {
        type ??= typeOf(schema);
        return type;
      },
      toString: () => schemaName,
    };
    models.set(schemaName, m);
    return m;
  };

  const fieldsOf = (s: Schema): Field[] => {
    const props = (at(s, 'properties') ?? {}) as Record<string, Schema>;
    const required = (at(s, 'required') ?? []) as string[];
    return Object.entries(props).map(([wire, p]) => ({
      wireName: wire,
      name: name(wire),
      required: required.includes(wire),
      description: text(p, 'description'),
      deprecated: false,
      readOnly: false,
      sensitive: false,
      default: at(p, 'default'),
      type: typeOf(p),
    }));
  };

  function baseType(s: Schema): Type {
    const ref = text(s, '$ref');
    if (ref !== '') {
      return { kind: 'model', model: modelOf(ref.split('/').pop() ?? '') };
    }
    if (Array.isArray(at(s, 'allOf'))) {
      return { kind: 'intersection', members: (at(s, 'allOf') as Schema[]).map(typeOf) };
    }
    const alts = (at(s, 'oneOf') ?? at(s, 'anyOf')) as Schema[] | undefined;
    if (Array.isArray(alts)) {
      return {
        kind: 'union',
        members: alts.map(typeOf),
        exclusive: at(s, 'oneOf') !== undefined,
        discriminator: undefined,
      };
    }
    if (Array.isArray(at(s, 'enum'))) {
      return { kind: 'enum', values: at(s, 'enum') as (string | number | boolean)[] };
    }
    switch (at(s, 'type')) {
      case 'array':
        return {
          kind: 'array',
          elem: typeOf((at(s, 'items') ?? {}) as Schema),
          minItems: undefined,
          maxItems: undefined,
        };
      case 'object': {
        const extra = at(s, 'additionalProperties');
        return {
          kind: 'object',
          fields: fieldsOf(s),
          extra:
            typeof extra === 'object' && extra !== null && !Array.isArray(extra)
              ? typeOf(extra as Schema)
              : undefined,
        };
      }
      case 'integer':
        return {
          kind: 'scalar',
          scalar: 'number',
          number:
            at(s, 'format') === 'int64' ? 'int64' : at(s, 'format') === 'int32' ? 'int32' : 'int',
        };
      case 'number':
        return {
          kind: 'scalar',
          scalar: 'number',
          number: at(s, 'format') === 'float' ? 'float' : 'double',
        };
      case 'boolean':
        return { kind: 'scalar', scalar: 'boolean' };
      case 'string':
        if (at(s, 'format') === 'date-time') return { kind: 'scalar', scalar: 'date-time' };
        if (at(s, 'format') === 'date') return { kind: 'scalar', scalar: 'date' };
        if (at(s, 'format') === 'uuid') return { kind: 'scalar', scalar: 'uuid' };
        if (at(s, 'format') === 'binary') return { kind: 'scalar', scalar: 'binary' };
        return { kind: 'scalar', scalar: 'string' };
      default:
        return { kind: 'scalar', scalar: 'unknown' };
    }
  }

  const byGroup = new Map<string, Method[]>();
  const groups: Group[] = [];
  const groupOf = (key: string): Group => {
    const existing = groups.find((g) => g.name.snake() === key);
    if (existing !== undefined) return existing;
    const methods: Method[] = [];
    byGroup.set(key, methods);
    const g: Group = {
      name: name(key),
      description: '',
      path: [name(key)],
      parent: undefined,
      children: [],
      descendants: [],
      methods,
      models: [],
    };
    groups.push(g);
    return g;
  };

  for (const [path, ops] of Object.entries(doc.paths)) {
    for (const [verb, op] of Object.entries(ops)) {
      const groupName = at(op, 'x-fern-sdk-group-name');
      if (!Array.isArray(groupName)) continue;
      const group = groupOf(groupName.join('_'));
      const locations = new Map<Field, Location>();
      const paramFields: Field[] = [];
      for (const p of (at(op, 'parameters') ?? []) as Schema[]) {
        const where = at(p, 'in');
        if (where !== 'query' && where !== 'path') continue;
        const field: Field = {
          wireName: String(at(p, 'name')),
          name: name(String(at(p, 'name'))),
          required: at(p, 'required') === true,
          description: text(p, 'description'),
          deprecated: false,
          readOnly: false,
          sensitive: false,
          default: undefined,
          type: typeOf((at(p, 'schema') ?? {}) as Schema),
        };
        paramFields.push(field);
        locations.set(field, where === 'path' ? { in: 'path' } : { in: 'query', exploded: true });
      }
      const params: Type | undefined =
        paramFields.length > 0
          ? { kind: 'object', fields: paramFields, extra: undefined }
          : undefined;
      const jsonOf = (holder: Schema | undefined): Type | undefined => {
        const content = (holder === undefined ? undefined : at(holder, 'content')) as
          | Record<string, Schema>
          | undefined;
        const media = content?.['application/json'];
        const schema = media === undefined ? undefined : at(media, 'schema');
        return schema === undefined ? undefined : typeOf(schema as Schema);
      };
      const bodyType = jsonOf(at(op, 'requestBody') as Schema | undefined);
      const responses: Response[] = Object.entries(
        (at(op, 'responses') ?? {}) as Record<string, Schema>,
      )
        .filter(([status]) => status.startsWith('2'))
        .map(([status, r]) => {
          const t = jsonOf(r);
          const content =
            t === undefined
              ? undefined
              : { mediaType: 'application/json', isJson: true, description: '', type: t };
          return {
            status,
            description: String(at(r, 'description') ?? ''),
            contents: content === undefined ? [] : [content],
            json: content,
          };
        });
      const extensions = Object.entries(op)
        .filter(([k]) => k.startsWith('x-') && !k.startsWith('x-fern-'))
        .map(([k, v]) => [k, v] as const);
      const bodyContent =
        bodyType === undefined
          ? undefined
          : { mediaType: 'application/json', isJson: true, description: '', type: bodyType };
      const method: Method = {
        name: name(String(at(op, 'x-fern-sdk-method-name') ?? verb)),
        group,
        httpMethod: verb.toUpperCase(),
        path,
        summary: String(at(op, 'summary') ?? ''),
        description: String(at(op, 'description') ?? ''),
        deprecated: false,
        availability: undefined,
        hidden: false,
        confirmation: undefined,
        request:
          bodyType === undefined
            ? params
            : params === undefined
              ? bodyType
              : { kind: 'intersection', members: [params, bodyType] },
        headers: undefined,
        body:
          bodyContent === undefined
            ? undefined
            : { required: true, description: '', contents: [bodyContent], json: bodyContent },
        responses,
        errors: [],
        extensions,
        extension: (key) => extensions.find(([k]) => k === key)?.[1],
        location: (field) => locations.get(field),
      };
      byGroup.get(group.name.snake())?.push(method);
    }
  }

  return {
    version: doc.info.version,
    description: doc.info.title,
    baseUrl: doc.servers?.[0]?.url ?? '',
    groups,
    models: [],
  };
}
