//! `SdkModel`: Ferrule's own, Forge-independent view of an SDK (ADR 0002).
//!
//! It describes the SDK, not the spec: names are already Python spellings, inline objects are still
//! anonymous (the types emitter names them by path, as Stainless does), and named models are
//! references into [`SdkClient::models`].
//!
//! Maps that the TypeScript model keeps as objects are `Vec<(key, value)>` here, in the order the
//! TypeScript object would iterate them.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SdkPrimitive {
    String,
    Integer,
    Number,
    Boolean,
    Null,
    Datetime,
    Date,
}

impl SdkPrimitive {
    /// The TypeScript spelling, such as `datetime`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Null => "null",
            Self::Datetime => "datetime",
            Self::Date => "date",
        }
    }
}

/// A literal value of an enum or `const`.
#[derive(Debug, Clone, PartialEq)]
pub enum SdkLiteral {
    String(String),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SdkType {
    Primitive(SdkPrimitive),
    Unknown,
    Array(Box<SdkType>),
    Map(Box<SdkType>),
    Union {
        variants: Vec<SdkType>,
        /// The wire name of the discriminating property, if the spec names one.
        discriminator: Option<String>,
    },
    Literal(Vec<SdkLiteral>),
    Ref(SdkRef),
    Object(SdkObject),
}

/// A named model: Python class `name` declared in `types/<module as path>.py`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SdkRef {
    pub name: String,
    /// Dotted module under the `types` package, such as `shared.references`.
    pub module: String,
}

/// An object type. Inline ones are named by the emitter from their position.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SdkObject {
    /// Named models this one extends (`allOf` with a named model).
    pub bases: Vec<SdkRef>,
    pub fields: Vec<SdkField>,
    /// The description of the schema it was inlined from, for a class docstring.
    pub description: Option<String>,
    /// The components.schemas name it was inlined from, which names union variants.
    pub hint: Option<String>,
    /// The type of the values under other keys, for an object with fields that takes them too
    /// (`additionalProperties` beside `properties`); `None` when it takes no other keys.
    pub extra: Option<Box<SdkType>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkField {
    /// The property name as sent.
    pub wire_name: String,
    /// The Python attribute name, such as `lat_span` or `from_`.
    pub py_name: String,
    pub required: bool,
    pub description: String,
    pub ty: SdkType,
}

/// A named model declared in its own module.
#[derive(Debug, Clone, PartialEq)]
pub struct SdkModelDecl {
    pub r#ref: SdkRef,
    pub description: String,
    /// The class, for a model whose schema is an object (empty when `alias` is set).
    pub ty: SdkObject,
    /// The type of a model whose schema is not an object (a union, enum, map, array or scalar):
    /// its module declares `Name: TypeAlias = ...` instead of a class.
    pub alias: Option<SdkType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkParam {
    pub wire_name: String,
    pub py_name: String,
    pub required: bool,
    pub description: String,
    pub ty: SdkType,
    /// Whether the spec marks it deprecated.
    pub deprecated: bool,
    /// The spec's `example`, `minimum` and `maximum` (for README and test examples). Forge does
    /// not expose them yet (docs/forge-gaps.md G18), so the adapter leaves them unset.
    pub example: Option<Value>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationScheme {
    Cursor,
    CursorId,
    CursorUrl,
    Offset,
    PageNumber,
    FakePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    Query,
    Body,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkPagination {
    pub scheme: PaginationScheme,
    /// Page class without the Sync/Async prefix, such as `CursorPage`.
    pub page: String,
    /// Wire query parameter -> purpose, such as `after` -> `next_cursor_param`.
    pub request: Vec<(String, String)>,
    /// Wire response field -> purpose; the field with purpose `items` holds the page items.
    pub response: Vec<(String, String)>,
    /// Where the request fields go.
    pub param_location: ParamLocation,
    /// Response header name -> purpose, for `from_header` properties.
    pub headers: Vec<(String, String)>,
    /// The type of one item.
    pub item: SdkType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFormat {
    Sse,
    Jsonl,
}

/// `x-ferrule-streaming`: the method returns `Stream[event]` (ADR 0003).
#[derive(Debug, Clone, PartialEq)]
pub struct SdkStreaming {
    pub format: StreamFormat,
    /// Body param that switches streaming on (an overload pair), or `None` when it always streams.
    pub param_discriminator: Option<String>,
    /// The type of one event: the named event model, else the 2xx response.
    pub event: Option<SdkType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpVerb {
    /// Lower case, as in the TypeScript model: `get`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
            Self::Patch => "patch",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkMethod {
    /// Python method name, such as `list`.
    pub name: String,
    /// JSON pointer of the operation, for diagnostics.
    pub pointer: String,
    pub http_method: HttpVerb,
    pub path: String,
    pub summary: String,
    pub description: String,
    /// Position in its resource's `methods` (stainless.yml order).
    pub order: usize,
    pub deprecated: bool,
    /// Path parameters in the order they appear in `path`.
    pub path_params: Vec<SdkParam>,
    pub query_params: Vec<SdkParam>,
    pub header_params: Vec<SdkParam>,
    /// The JSON (or multipart) request body; its object fields are `body_params`.
    pub body: Option<SdkType>,
    /// The top-level properties of an object body, as keyword arguments.
    pub body_params: Vec<SdkParam>,
    /// The request body's media type, such as `application/json`.
    pub body_media_type: Option<String>,
    /// A map or array body (honcho `sessions.peers.add/remove`): `body_params` is then the one
    /// keyword argument `body`, sent as the whole request body.
    pub body_whole: bool,
    /// A body that is raw bytes (`format: binary`, such as `application/octet-stream`): `body_params`
    /// is then the one keyword argument `body`, sent as the request content with the body's
    /// media type.
    pub body_binary: bool,
    /// A body that is a union of objects (imagekit `files.update`, knock-mgmt `guides.activate`):
    /// one overload per variant, in spec order. `body_params` then holds every variant's fields
    /// once, required only where every variant requires them.
    pub body_variants: Vec<SdkBodyVariant>,
    /// The wire name of the property that discriminates `body_variants`, if the spec names one.
    pub body_discriminator: Option<String>,
    /// For a `multipart/form-data` body: the paths of its binary fields (Python names, `<array>`
    /// for every item of an array), sent as files (`extract_files(..., paths=[["file"]])`).
    /// Empty for a body sent as JSON.
    pub body_file_paths: Vec<Vec<String>>,
    /// The decoded `2XX` JSON response, or `None` for none: with `unwrap`, the type of that
    /// envelope field.
    pub response: Option<SdkType>,
    /// The response envelope field the method returns (`unwrap_response`), by wire name.
    pub unwrap: Option<String>,
    pub pagination: Option<SdkPagination>,
    pub streaming: Option<SdkStreaming>,
}

/// One object variant of a union request body.
#[derive(Debug, Clone, PartialEq)]
pub struct SdkBodyVariant {
    /// The variant's schema; its `hint` (or discriminator value) names its `TypedDict`.
    pub object: SdkObject,
    /// Its fields as keyword arguments, as `body_params` are for an object body.
    pub params: Vec<SdkParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkResource {
    /// Python names from the client down, such as `["agencies_with_coverage"]`.
    pub path: Vec<String>,
    /// Position among its siblings in stainless.yml.
    pub order: usize,
    pub methods: Vec<SdkMethod>,
    pub subresources: Vec<SdkResource>,
    /// The class-name segment of the last path word, spelled with the config's custom casings
    /// (`QA` for `qa`); `None` derives it from the path with the default rules.
    pub class_segment: Option<String>,
    /// The same for the singular of the last word, which names its method types
    /// (`QACreateParams`); `None` derives it with the default rules.
    pub singular_class_segment: Option<String>,
    /// `standalone_api: true`: a package of its own (`resources/extractions/qa/qa.py`) with its
    /// types in its own directory (`types/extractions/qa/`) and its own `api.md`.
    pub standalone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkPackage {
    /// Python import name, such as `onebusaway`.
    pub name: String,
    /// GitHub `owner/repo` used in README links, such as `OneBusAway/python-sdk`.
    pub repo: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptType {
    String,
    Number,
    Integer,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRole {
    Value,
    Username,
    Password,
    ClientId,
    ClientSecret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkAuth {
    pub security_scheme: String,
    pub role: AuthRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendLocation {
    Header,
    Query,
    Body,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkSend {
    pub location: SendLocation,
    pub name: String,
}

/// One `client_settings.opts` entry: a client constructor argument.
#[derive(Debug, Clone, PartialEq)]
pub struct SdkClientOpt {
    /// Python argument name, such as `api_key`.
    pub name: String,
    pub ty: OptType,
    pub nullable: bool,
    /// `None` when the config gives no default (TypeScript `undefined`).
    pub default: Option<Value>,
    /// Environment variable read when the argument is not given.
    pub read_env: Option<String>,
    pub description: Option<String>,
    pub auth: Option<SdkAuth>,
    /// Where the client sends it on every request, if anywhere.
    pub send: Option<SdkSend>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpAuthScheme {
    Bearer,
    Digest,
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkSecurityScheme {
    Http {
        scheme: HttpAuthScheme,
        keyword: Option<String>,
    },
    ApiKey {
        location: ApiKeyLocation,
        name: String,
    },
    OAuth2,
    OpenIdConnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayFormat {
    Comma,
    Repeat,
    Indices,
    Brackets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedFormat {
    Brackets,
    Dots,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkEnvironment {
    pub name: String,
    pub url: String,
}

/// Client class settings from `x-ferrule-client` (ADR 0003).
#[derive(Debug, Clone, PartialEq)]
pub struct SdkClientSettings {
    /// `organization.name` (or `default_client_name`) as written, such as `onebusaway-sdk`.
    pub name: String,
    /// The client class, such as `OnebusawaySDK`.
    pub class_name: String,
    /// Prefix of `<PREFIX>_BASE_URL` and friends, such as `ONEBUSAWAY_SDK`.
    pub env_prefix: String,
    pub opts: Vec<SdkClientOpt>,
    /// In config order; the first is the default.
    pub environments: Vec<SdkEnvironment>,
    pub security_schemes: Vec<(String, SdkSecurityScheme)>,
    /// Alternatives, each a set of scheme names that apply together.
    pub security: Vec<Vec<String>>,
    pub timeout_seconds: f64,
    pub max_retries: u32,
    pub idempotency_header: Option<String>,
    pub default_headers: Vec<(String, String)>,
    pub array_format: ArrayFormat,
    pub nested_format: NestedFormat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkClient {
    pub package: SdkPackage,
    pub base_url: String,
    /// `None` in a fixture without `x-ferrule-client`.
    pub settings: Option<SdkClientSettings>,
    /// Top-level resources in stainless.yml order.
    pub resources: Vec<SdkResource>,
    /// `$client` methods, emitted on the client class.
    pub client_methods: Vec<SdkMethod>,
    /// The client class, such as `OnebusawaySDK` (`settings.class_name`, for the file emitters).
    pub client_class: Option<String>,
    /// Prefix of the SDK's env vars, such as `ONEBUSAWAY_SDK` (`settings.env_prefix`).
    pub env_prefix: Option<String>,
    /// Every named model the resources reference, sorted by module.
    pub models: Vec<SdkModelDecl>,
    /// `resources` plus the standalone APIs that only declare models (and their parents), in
    /// stainless.yml order: what api.md, the test packages and the empty resource packages list.
    /// Empty in a hand-built model, where `resources` stands in.
    pub resource_tree: Vec<SdkResource>,
    /// What the generator that made the SDK's runtime spelled differently; today's by default.
    pub style: SdkStyle,
}

/// `x-ferrule-style`: the spellings of the Stainless generator an adopted SDK's runtime comes
/// from, where they differ from today's (proposed ADR "older generator styles"). Each field is true for
/// today's spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is an independent switch of x-ferrule-style; the runtimes mix them"
)]
pub struct SdkStyle {
    /// Optional arguments default to `omit` and options to `not_given` (`sentinel: omit`), or
    /// both to `NOT_GIVEN` (`sentinel: NOT_GIVEN`, a `_types.py` without `omit`).
    pub omit: bool,
    /// Client resources are `cached_property`s (`resources: lazy`), or attributes set in
    /// `__init__` (`resources: eager`, a `_client.py` without `cached_property`).
    pub lazy_resources: bool,
    /// Paths go through `path_template` (`paths: path_template`), or are f-strings
    /// (`paths: fstring`, a runtime without `_utils/_path.py`).
    pub path_template: bool,
    /// A multipart body is copied with `_files.deepcopy_with_paths` (`multipart:
    /// deepcopy_with_paths`), or `_utils.deepcopy_minimal` (`multipart: deepcopy_minimal`, a
    /// `_files.py` without it).
    pub deepcopy_with_paths: bool,
}

impl Default for SdkStyle {
    fn default() -> Self {
        Self {
            omit: true,
            lazy_resources: true,
            path_template: true,
            deepcopy_with_paths: true,
        }
    }
}

impl SdkStyle {
    /// The default of an option such as `timeout`: `not_given` or `NOT_GIVEN`.
    #[must_use]
    pub fn not_given(self) -> &'static str {
        if self.omit { "not_given" } else { "NOT_GIVEN" }
    }

    /// The sentinel type and default of an optional argument: `("Omit", "omit")` or
    /// `("NotGiven", "NOT_GIVEN")`.
    #[must_use]
    pub fn optional(self) -> (&'static str, &'static str) {
        if self.omit {
            ("Omit", "omit")
        } else {
            ("NotGiven", "NOT_GIVEN")
        }
    }
}
