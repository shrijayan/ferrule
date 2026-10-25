//! `SdkModel`: Ferrule's own, Forge-independent view of an SDK (ADR 0002).
//!
//! It describes the SDK, not the spec: names are already Python spellings, inline objects are
//! still anonymous (the types emitter names them by path, as Stainless does), and named models
//! are references into [`SdkClient::models`].
//!
//! Port of `python-plugin/src/model/index.ts`. Maps that the TypeScript model keeps as objects
//! are `Vec<(key, value)>` here, in the order the TypeScript object would iterate them.

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
    pub ty: SdkObject,
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
    /// The decoded `2XX` JSON response, or `None` for none.
    pub response: Option<SdkType>,
    pub pagination: Option<SdkPagination>,
    pub streaming: Option<SdkStreaming>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SdkResource {
    /// Python names from the client down, such as `["agencies_with_coverage"]`.
    pub path: Vec<String>,
    /// Position among its siblings in stainless.yml.
    pub order: usize,
    pub methods: Vec<SdkMethod>,
    pub subresources: Vec<SdkResource>,
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
}
