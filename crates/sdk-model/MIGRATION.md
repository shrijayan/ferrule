# Moving the Python plugin onto `ferrule-sdk-model`

The Python plugin keeps its own adapter and `model.rs` for now (ADR 0017). This file maps every
field of its `SdkModel` to the neutral model, so the move is mechanical: the Python plugin gets a
`from_neutral` step (namer plus type mapping) in place of `src/adapter/`, and its emitters stay as
they are.

The mapping is executable: `tests/python_cross_check.rs` derives the Python `SdkClient` from the
neutral `Client` with exactly these rules and asserts it equals what the Python adapter builds,
for every annotated fixture of the Python plugin (`tests/fixtures/*.json` and every
`annotated.json` under its tests), the generator-era fixture under each `x-ferrule-style`, and
every cached config oracle the Python adapter accepts. Move that derivation into
`crates/python-plugin`, delete `src/adapter/`, and the goldens prove the rest.

Names below: `n::` is `ferrule_sdk_model::model`, `py::` is `ferrule_python_plugin::model`.
`snake_case`, `pascal_case`, `split_words`, `class_segment_with`, `singular` are the Python
plugin's `naming` functions, which stay in the Python plugin as its namer.

## Names

| Python                                   | Neutral source                                                        |
| ---------------------------------------- | --------------------------------------------------------------------- |
| `SdkField.py_name`, `SdkParam.py_name`   | `snake_case(field.name.words)`                                        |
| `SdkField.wire_name`, `SdkParam.wire_name` | `field.wire_name`                                                   |
| `SdkMethod.name`                         | `method.name.written` (validated as an identifier by the adapter)     |
| `SdkRef.name`                            | `type_ref.name.written` (the config's class spelling)                 |
| `SdkRef.module`                          | `type_ref.path`, each segment `py_key(written)`, joined with `.`      |
| `SdkResource.path`                       | `resource.path`, each `py_key(written)`                               |
| `SdkResource.class_segment`              | `class_segment_with(split_words(path.last), casings)`                 |
| `SdkResource.singular_class_segment`     | `class_segment_with(singular(split_words(path.last)), casings)`       |
| `SdkPagination.page`                     | `pascal_case(split_words(pagination.page.written))`                   |
| `SdkClientSettings.name`                 | `settings.name.written`                                               |
| `SdkClientSettings.class_name`, `SdkClient.client_class` | `client_class_name_with(settings.name.written, casings)` |
| `SdkClientOpt.name`                      | `snake_case(split_words(opt.name.written))` (`X-Session-Token` -> `x_session_token`) |
| `SdkEnvironment.name`                    | `environment.name.written`                                            |
| `SdkMethod.body_file_paths` segments     | `snake_case(field.name.words)`, `<array>` for every item of an array  |

`py_key(s)` keeps `s` when it matches `^[a-z_][a-z0-9_]*$`, else `snake_case(split_words(s))`
(`trip-details` -> `trip_details`, `assistantMessage` -> `assistant_message`). `casings` is
`client.casings` as `(word, Casing)` pairs.

## Types (`py::SdkType` from `n::Type`)

| Neutral                                   | Python                                                       |
| ----------------------------------------- | ------------------------------------------------------------ |
| `Scalar` `unknown`, `binary`              | `Unknown`                                                    |
| `Scalar` `string`, `uuid`, `bytes`        | `Primitive(String)`                                          |
| `Scalar` `int`, `int8` .. `uint64`        | `Primitive(Integer)`                                         |
| `Scalar` `float`, `double`                | `Primitive(Number)`                                          |
| `Scalar` `null`, `boolean`, `date`, `date_time` | `Primitive(Null / Boolean / Date / Datetime)`          |
| `Enum { members }`                        | `Literal(members[].value)`                                   |
| `Array`, `Map`                            | `Array`, `Map`                                               |
| `Object` with no fields and no bases      | `Unknown`                                                    |
| `Object(o)`                               | `Object { bases, fields, description, hint: o.origin, extra: o.extra }` |
| `Union(u)`                                | `Union { variants: flat(u.variants[].ty), discriminator: u.discriminator.wire_name }` |
| `Nullable(t)`                             | `Union { variants: flat([t]) + [Null], discriminator: None }` |
| `Ref(r)`                                  | `Ref(SdkRef)` as above                                       |

`flat` maps each member and splices in the variants of a member that maps to a union, dropping
its discriminator, as Stainless does (ogx `Output`). So `Nullable(Union(u))` is `u`'s variants
plus `Null`, discriminated or not.

The one case this cannot reproduce: an `allOf` whose every member gives no shape reads as an
empty `Object` in Python but as "any value" after the mapping. No fixture or oracle has one.

## Declarations, methods, resources, client

- `SdkModelDecl { ref, description }`: `ModelDecl.ref`, `.description`. `ty`/`alias`: map
  `ModelDecl.ty`; an object is the class, `Unknown` an empty class, anything else the alias.
- `SdkClient.models`: `client.models` mapped, then stably sorted by Python module (UTF-16 order,
  `js::cmp`); the neutral order is by the written path.
- `SdkMethod`: `pointer`, `path`, `summary`, `description`, `order`, `deprecated` as they are;
  `http_method` from `HttpMethod` (Python rejects anything but get, post, put, patch, delete with
  FER201); `path_params`, `query_params`, `header_params` from `Param.field` (with its
  `deprecated`); `body` and `body_media_type` from `Body.ty` (mapped) and `Body.media_type`;
  `response` from `Response.ty`; `example`/`minimum`/`maximum` stay `None` (Forge gap G18).
- The body shapes, read on the mapped body type `b` with the mapped declarations at hand:
  - `body_variants`, `body_discriminator`: when `b` (or, for `Ref`, the declaration's alias) is
    a `Union` of two or more variants that are all `Object`s, one `SdkBodyVariant { object,
    params }` per variant and the union's discriminator; else empty and `None`.
  - `body_params`: with variants, their params merged (each wire name once, in first-seen order,
    required only where every variant requires it); else the fields of the object of `b` (an
    `Object`, the one non-null variant of a two-variant union with `Null`, or the class a `Ref`
    declares). Body params carry `deprecated: false`, as the Python adapter has them.
  - `body_whole`: `b` is a `Map` or `Array`. `body_params` is then the one param `body`
    (`required: Body.required`, `description: Body.schema_description`, `ty: b`).
  - `body_file_paths`: for a `multipart/form-data` body (any case), each field of
    `client.object_of(Body.ty)` that is `Scalar` `binary` (`[name]`) or an `Array` of it
    (`[name, "<array>"]`); empty otherwise.
- `SdkPagination`: `scheme`, `param_location`, `item` mapped; `request`, `response`, `headers`
  from `Purpose { field, purpose }` pairs.
- `SdkStreaming`: `format`, `param_discriminator`, `event` mapped.
- `SdkResource`: `order`, `standalone`, methods and subresources mapped. `SdkClient.resource_tree`
  is `client.resources` mapped; `SdkClient.resources` is that tree without the resources that
  have no methods, themselves or below.
- `SdkClient.package`, `base_url`, `client_methods`: as they are. `env_prefix`:
  `settings.env_prefix`.
- `SdkClient.style`: today's `SdkStyle`, then for each `Client.style` switch with target
  `python`, the flag of its key (`sentinel` -> `omit`, `resources` -> `lazy_resources`, `paths`
  -> `path_template`, `multipart` -> `deepcopy_with_paths`) set to whether its value is today's
  spelling (`omit`, `lazy`, `path_template`, `deepcopy_with_paths`).
- `SdkClientSettings`: every other member as it is; `security_schemes` and `default_headers`
  from their named structs to pairs.

## What the neutral model carries for Python

Every Python field above is derived; nothing is skipped. Three facts the Python adapter reads
from Forge directly are carried by the neutral model for it:

- `Body.schema_description`: Forge's content description (the body schema's own, else that of
  the schema it refers to), apart from `Body.description`, which describes the body as a whole.
- `Object.origin` on the items of an inlined array schema of inline objects, which Stainless
  names after the array schema (knock-mgmt `HeadersRequestTemplateHeadersArray`).
- `Client.style`: `x-ferrule-style` (ADR 0018). It is not an API fact but a property of one
  target's runtime (`--runtime-from` reads the Python runtime), so each switch is kept as written,
  `{ target, key, value }` with target `python`; the Python namer interprets it and other targets
  ignore it. The neutral reader validates it as the Python one does (FER101).

Union body variants and multipart file paths need no field of their own: they are in `Body.ty`
(its union's variants and discriminator, its `binary` fields), and only their Python shape (the
merged params, the paths by Python name) is derived.

## What the neutral model adds (Python ignores it today)

`Client.version`, `Client.description`; `Field.deprecated` on body fields, `Field.read_only`,
`Field.default`; `Param.exploded`; `Body.description`; `Response.status`,
`Response.media_type`; `Union.exclusive`, variant names and
discriminator values, `Discriminator.mapping`; enum member names; integer and float sizes,
`uuid`, `bytes`, `binary`; HTTP methods beyond the five; `ModelDecl.deprecated`;
`ClientSettings.default_environment`, `initial_retry_delay_seconds`, `max_retry_delay_seconds`.

## Steps

1. Add `ferrule-sdk-model` as a dependency of `crates/python-plugin`; move the derivation from
   `tests/python_cross_check.rs` into `src/from_neutral.rs`.
2. Point `plugin.rs` at `ferrule_sdk_model::forge_to_model` plus `from_neutral`; run
   `mise run golden` (byte-identical) and the plugin's tests.
3. Delete `src/adapter/`, `src/diagnostics.rs` (use `ferrule_sdk_model::diagnostics`) and
   `src/js.rs`'s duplicates of `entries`/`cmp`/`is_space`; the plugin's boundary test then checks
   that no file imports Forge at all.
4. Move the Python adapter tests that check Python spellings (`py_name`, modules) next to
   `from_neutral`; the rest already exist here.
