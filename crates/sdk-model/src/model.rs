//! The language-neutral SDK model (ADR 0002, ADR 0017).
//!
//! It describes the SDK, not the spec, and never in a language's spelling: every named thing keeps
//! its [`Name`] (words plus the spelling it was written with) and its wire name, and each
//! language's namer spells identifiers from those and [`Client::casings`]. Named types are
//! [`TypeRef`]s, identified by schema name and stainless.yml position, not by a file path.
//!
//! Inline objects are still anonymous, as Stainless names them by position; named models are
//! declared once in [`Client::models`]. Lists that come from JSON objects keep the order a
//! JavaScript `Object.entries` would give, as the converter and the plugins rely on.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A name as words, so that every language can spell it in its own casing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Name {
    /// The spelling the spec or config wrote it with: a wire name, a stainless.yml key, or the
    /// config's own spelling of a model name (`RouteIDs`). Namers that must reproduce a Stainless
    /// spelling exactly start from this.
    pub written: String,
    /// The lower-case words, split as Forge's `Name` splits them: `XMLDocument` is `xml document`,
    /// `getByID` is `get by id`.
    pub words: Vec<String>,
}

/// A type with no parts: Forge's full scalar set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scalar {
    /// Any value.
    Unknown,
    Null,
    Boolean,
    String,
    /// A string with format `uuid`.
    Uuid,
    /// An `integer` with no size.
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    /// Format `float`.
    Float,
    /// Format `double`, or a `number` with no format.
    Double,
    /// A string with format `date-time`.
    DateTime,
    /// A string with format `date`.
    Date,
    /// Base64 text (format `byte`).
    Bytes,
    /// Raw bytes (format `binary`): a file upload or an octet-stream body.
    Binary,
}

impl Scalar {
    /// Reports whether the scalar is an integer of any size.
    #[must_use]
    pub fn is_integer(self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::Int8
                | Self::Int16
                | Self::Int32
                | Self::Int64
                | Self::Uint8
                | Self::Uint16
                | Self::Uint32
                | Self::Uint64
        )
    }
}

/// A literal value of an enum or `const`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// One value of an enum, with a name for the languages that declare enum members.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumMember {
    pub value: Literal,
    /// The value's words (`in_progress` -> `in progress`); empty for a value with no letters or
    /// digits. Forge does not expose `x-enum-varnames`, so this is always derived from the value.
    pub name: Name,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    Scalar {
        scalar: Scalar,
    },
    /// One of the values, in the order the spec lists them; never null.
    Enum {
        members: Vec<EnumMember>,
    },
    Array {
        items: Box<Type>,
    },
    /// An object with no fields whose other keys have this type.
    Map {
        values: Box<Type>,
    },
    Object(Object),
    Union(Union),
    /// A named model, declared in [`Client::models`].
    Ref(TypeRef),
    /// `inner` or null. Distinct from a field that may be absent ([`Field::required`]).
    Nullable {
        inner: Box<Type>,
    },
}

impl Type {
    #[must_use]
    pub fn scalar(scalar: Scalar) -> Self {
        Self::Scalar { scalar }
    }

    /// The `kind` tag, for diagnostics.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Scalar { .. } => "scalar",
            Self::Enum { .. } => "enum",
            Self::Array { .. } => "array",
            Self::Map { .. } => "map",
            Self::Object(_) => "object",
            Self::Union(_) => "union",
            Self::Ref(_) => "ref",
            Self::Nullable { .. } => "nullable",
        }
    }
}

/// A named model: stable identity plus the name each language spells.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeRef {
    /// The `components.schemas` key it was declared from, such as `Reference`.
    pub schema: String,
    /// Where stainless.yml declares it: resource keys (`shared` for `$shared`), then the `models`
    /// key, as written (`["shared", "references"]`). Languages map this to a file of their own.
    pub path: Vec<Name>,
    /// The model's name: the words of its `models` key, written as the config spells the class
    /// (`custom_casings` applied), such as `References`.
    pub name: Name,
}

/// An object type. Inline ones are named by the emitters from their position.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Object {
    /// Named models this one extends (`allOf` with a named model).
    pub bases: Vec<TypeRef>,
    pub fields: Vec<Field>,
    /// The type of keys other than the fields, when the schema allows them.
    pub extra: Option<Box<Type>>,
    /// The description of the schema or property it was written under.
    pub description: Option<String>,
    /// The `components.schemas` key it was inlined from, which Stainless uses to name variants.
    pub origin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// The words of `x-fern-property-name` or the wire name; `written` is the wire name.
    pub name: Name,
    /// The property name as sent.
    pub wire_name: String,
    pub required: bool,
    pub deprecated: bool,
    pub read_only: bool,
    pub description: String,
    /// The schema's `default`, if any.
    pub default: Option<Value>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Union {
    pub variants: Vec<Variant>,
    pub discriminator: Option<Discriminator>,
    /// `oneOf` (a value is at most one variant) rather than `anyOf`.
    pub exclusive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    /// The model's name for a named variant (`written` is its schema name), else the words of its
    /// discriminator value, else `None`.
    pub name: Option<Name>,
    /// A discriminator value that selects this variant; [`Discriminator::mapping`] has them all.
    pub discriminator_value: Option<String>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discriminator {
    /// The discriminating property as sent.
    pub wire_name: String,
    pub name: Name,
    /// Every value the discriminator selects a variant by, by variant then value; a mapping may
    /// give one variant several values.
    pub mapping: Vec<DiscriminatorCase>,
}

/// One discriminator value and the variant it selects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscriminatorCase {
    pub value: String,
    /// The index of the variant in [`Union::variants`].
    pub variant: usize,
}

/// A named model declared once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDecl {
    pub r#ref: TypeRef,
    pub description: String,
    pub deprecated: bool,
    /// The schema's type: an object for a class, anything else for an alias.
    pub ty: Type,
}

/// A path, query or header parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    #[serde(flatten)]
    pub field: Field,
    /// For a query parameter: whether each array item is its own parameter (`explode`).
    pub exploded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Body {
    /// Such as `application/json` or `multipart/form-data`.
    pub media_type: String,
    pub required: bool,
    /// The description of the body as a whole.
    pub description: String,
    /// The description of the body's schema in this media type (its own, else that of the schema
    /// it refers to): what Stainless documents a map or array body argument with.
    pub schema_description: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Such as `200` or `2XX`.
    pub status: String,
    pub media_type: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Put,
    Post,
    Delete,
    Options,
    Head,
    Patch,
    Trace,
    Query,
    Connect,
}

impl HttpMethod {
    /// Lower case: `get`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Put => "put",
            Self::Post => "post",
            Self::Delete => "delete",
            Self::Options => "options",
            Self::Head => "head",
            Self::Patch => "patch",
            Self::Trace => "trace",
            Self::Query => "query",
            Self::Connect => "connect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaginationScheme {
    Cursor,
    CursorId,
    CursorUrl,
    Offset,
    PageNumber,
    FakePage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamLocation {
    Query,
    Body,
}

/// A field path mapped to what it is for, such as `after` -> `next_cursor_param`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Purpose {
    /// A wire field path: dotted, with `[]` stepping into array items.
    pub field: String,
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pagination {
    pub scheme: PaginationScheme,
    /// The page type's name, such as `cursor_page`; each language prefixes and spells it.
    pub page: Name,
    /// Request fields, in config order.
    pub request: Vec<Purpose>,
    /// Response fields; the one with purpose `items` holds the page items.
    pub response: Vec<Purpose>,
    pub param_location: ParamLocation,
    /// Response headers, for `from_header` properties.
    pub headers: Vec<Purpose>,
    /// The type of one item.
    pub item: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamFormat {
    Sse,
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Streaming {
    pub format: StreamFormat,
    /// The body param that switches streaming on, or `None` when the method always streams.
    pub param_discriminator: Option<String>,
    /// The type of one event: the named event model, else the 2xx response.
    pub event: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Method {
    /// From `x-ferrule-method-name`, such as `list`.
    pub name: Name,
    /// JSON pointer of the operation, for diagnostics.
    pub pointer: String,
    pub http_method: HttpMethod,
    pub path: String,
    pub summary: String,
    pub description: String,
    /// Position in its resource's `methods` (stainless.yml order).
    pub order: usize,
    pub deprecated: bool,
    /// In the order the path template names them.
    pub path_params: Vec<Param>,
    pub query_params: Vec<Param>,
    pub header_params: Vec<Param>,
    /// The JSON body, else the first content of the body.
    pub body: Option<Body>,
    /// The first 2xx response with JSON content.
    pub response: Option<Response>,
    pub pagination: Option<Pagination>,
    pub streaming: Option<Streaming>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// The stainless.yml key, such as `agencies_with_coverage`.
    pub name: Name,
    /// The keys from the client down, this one last.
    pub path: Vec<Name>,
    /// Position among its siblings in stainless.yml.
    pub order: usize,
    pub methods: Vec<Method>,
    pub subresources: Vec<Resource>,
    /// `standalone_api: true`.
    pub standalone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    /// `x-ferrule-package.name`: today the Python target's package name.
    pub name: String,
    /// GitHub `owner/repo` of the SDK, or empty.
    pub repo: String,
}

/// How `custom_casings` spells one word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Casing {
    /// `initialism: true` upper-cases it (`qa` -> `QA`); `false` stops treating it as one.
    Initialism(bool),
    /// The spelling as written, first letter included: `LumaAI`, `v1`.
    Pascal(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomCasing {
    /// The lower-case word.
    pub word: String,
    pub casing: Casing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptType {
    String,
    Number,
    Integer,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRole {
    Value,
    Username,
    Password,
    ClientId,
    ClientSecret,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Auth {
    pub security_scheme: String,
    pub role: AuthRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendLocation {
    Header,
    Query,
    Body,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Send {
    pub location: SendLocation,
    /// The wire name it is sent as.
    pub name: String,
}

/// One `client_settings.opts` entry: a client constructor option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientOpt {
    /// The stainless.yml key, such as `api_key`.
    pub name: Name,
    pub ty: OptType,
    pub nullable: bool,
    /// `None` when the config gives no default.
    pub default: Option<Value>,
    /// The environment variable read when the option is not given.
    pub read_env: Option<String>,
    pub description: Option<String>,
    pub auth: Option<Auth>,
    /// Where the client sends it on every request, if anywhere.
    pub send: Option<Send>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpAuthScheme {
    Bearer,
    Digest,
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyLocation {
    Header,
    Query,
    Cookie,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SecurityScheme {
    Http {
        scheme: HttpAuthScheme,
        keyword: Option<String>,
    },
    ApiKey {
        location: ApiKeyLocation,
        name: String,
    },
    #[serde(rename = "oauth2")]
    OAuth2,
    OpenIdConnect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedSecurityScheme {
    pub name: String,
    pub scheme: SecurityScheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrayFormat {
    Comma,
    Repeat,
    Indices,
    Brackets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NestedFormat {
    Brackets,
    Dots,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// The stainless.yml key, such as `production`.
    pub name: Name,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub name: String,
    pub value: String,
}

/// Client settings from `x-ferrule-client` (ADR 0003).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettings {
    /// `organization.name` (or `default_client_name`), such as `onebusaway-sdk`; namers spell the
    /// client class from it.
    pub name: Name,
    /// Prefix of `<PREFIX>_BASE_URL` and friends, such as `ONEBUSAWAY_SDK`.
    pub env_prefix: String,
    pub opts: Vec<ClientOpt>,
    /// In config order; the first is the default unless `default_environment` says otherwise.
    pub environments: Vec<Environment>,
    pub default_environment: Option<String>,
    pub security_schemes: Vec<NamedSecurityScheme>,
    /// Alternatives, each a set of scheme names that apply together.
    pub security: Vec<Vec<String>>,
    pub timeout_seconds: f64,
    pub max_retries: u32,
    pub initial_retry_delay_seconds: Option<f64>,
    pub max_retry_delay_seconds: Option<f64>,
    pub idempotency_header: Option<String>,
    pub default_headers: Vec<Header>,
    pub array_format: ArrayFormat,
    pub nested_format: NestedFormat,
}

/// The whole SDK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Client {
    pub package: Package,
    /// `info.version`; empty where Forge does not expose it (one group).
    pub version: String,
    /// `info.title`, `summary` and `description`; empty where Forge does not expose it.
    pub description: String,
    pub base_url: String,
    /// `None` in a fixture without `x-ferrule-client`.
    pub settings: Option<ClientSettings>,
    /// `custom_casings` by lower-case word, in config order.
    pub casings: Vec<CustomCasing>,
    /// Top-level resources in stainless.yml order, including the standalone APIs that only
    /// declare models (and their parents), which have no methods.
    pub resources: Vec<Resource>,
    /// `$client` methods, on the client itself.
    pub client_methods: Vec<Method>,
    /// Every named model the methods reference, sorted by stainless.yml path.
    pub models: Vec<ModelDecl>,
}

impl Client {
    /// The declaration of `r`, if it was declared.
    #[must_use]
    pub fn model(&self, r: &TypeRef) -> Option<&ModelDecl> {
        self.models.iter().find(|d| d.r#ref == *r)
    }

    /// The object `ty` is, following a reference to a declared model.
    #[must_use]
    pub fn object_of<'a>(&'a self, ty: &'a Type) -> Option<&'a Object> {
        match ty {
            Type::Object(o) => Some(o),
            Type::Ref(r) => match &self.model(r)?.ty {
                Type::Object(o) => Some(o),
                _ => None,
            },
            _ => None,
        }
    }
}
