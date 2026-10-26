//! What the project-file emitters need, as a narrow type of its own, so they don't depend on the
//! whole SDK model. Everything is already a Python or text spelling.

use serde_json::Value;

use crate::model::{ArrayFormat, HttpVerb, NestedFormat, SdkType};

/// Config editions the templates were extracted for (OneBusAway: Rye, Python 3.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonEdition {
    /// `python.2025-10-08`.
    Python20251008,
}

/// One keyword argument of an [`ExampleRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleArg {
    /// Python keyword, such as `stop_id`.
    pub name: String,
    /// The value as a one-line Python literal, such as `"1_75403"` or `["a", "b"]`.
    pub value: String,
    /// The same value with the trailing commas the oracle READMEs' examples have (in dicts, not
    /// in lists), which ruff then splits one element per line.
    pub readme_value: String,
    pub required: bool,
}

/// One request used as an example in README.md, or as the retry probe in `tests/test_client.py`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleRequest {
    /// Dotted resource accessor from the client, such as `current_time` or `chat.completions`.
    pub accessor: String,
    /// Python method name, such as `retrieve`.
    pub method: String,
    /// Keyword arguments in call order (required, then optional, each by name); empty for none.
    pub args: Vec<ExampleArg>,
    pub http_method: HttpVerb,
    /// The path as respx matches it, such as `/api/where/current-time.json`.
    pub path: String,
    /// Variable the README assigns the result to: the response model's (`embedding_response`),
    /// the resource's for a synthesized response (`current_time`), `page` for a page.
    pub result: String,
    /// Variable the README assigns a parsed raw response to: the resource's (`embedding`).
    pub parsed: String,
    /// The response property the README prints after the call, such as `id` (a page's items for
    /// a page); `None` prints nothing.
    pub print_property: Option<String>,
    /// The property it prints of a parsed raw response (of one item for a page); `None` prints
    /// the whole result.
    pub parsed_property: Option<String>,
    /// A paginated method: the oracles have no connection-leak tests for one.
    pub paginated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    Path,
    Query,
    Body,
    /// A header parameter: the tests pass it after the others.
    Header,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesParam {
    pub py_name: String,
    pub wire_name: String,
    pub required: bool,
    pub location: ParamLocation,
    pub ty: SdkType,
    /// The spec's `example`, when it has one; strings otherwise default to the wire name.
    pub example: Option<Value>,
    /// The schema's `minimum`/`maximum`: a number example is the minimum, else the maximum, else 0.
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    /// A raw-bytes body: its example is bytes.
    pub binary: bool,
}

/// A Python name and the dotted module (under the package) that declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyName {
    pub name: String,
    /// Such as `types.trips_for_location_list_response` or `types.shared.response_wrapper`.
    pub module: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesMethod {
    pub name: String,
    pub http_method: HttpVerb,
    /// The path as written in the spec, such as `/api/where/agency/{agencyID}.json`.
    pub path: String,
    /// Path parameters first, in path order, then the rest.
    pub params: Vec<FilesParam>,
    /// Dotted module of the `*_params.py` file; `None` when the method has no params class.
    pub params_module: Option<String>,
    /// The return type; `None` for `None`.
    pub response: Option<PyName>,
    /// The Python name of the body parameter that switches a param-switched streaming method to
    /// streaming (one of `params`): its tests come in two overloads.
    pub stream_switch: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesResource {
    /// Python attribute name, such as `trip_details`.
    pub name: String,
    /// Class segment, such as `TripDetails` or `StopIDsForAgency` (api.md heading, test class).
    pub title: String,
    /// Local variable the tests and README use for a result, such as `trip_detail`.
    pub variable: String,
    /// Dotted module of the resource class, such as `resources.trip_details`.
    pub module: String,
    /// Names api.md lists under "Types:", in order, from module `types_module`.
    pub types: Vec<String>,
    /// Such as `types` or `types.chat`.
    pub types_module: String,
    pub methods: Vec<FilesMethod>,
    pub subresources: Vec<FilesResource>,
    /// For a standalone API (its own package and api.md), where the root api.md links to.
    pub standalone_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    /// `organization.name`, such as `onebusaway-sdk`.
    pub name: String,
    /// Display name, such as `Onebusaway SDK`.
    pub title: String,
    pub contact: String,
    pub docs_url: String,
    /// `organization.security_contact`, which SECURITY.md names (isaacus); else `contact`.
    pub security_contact: String,
}

/// How the client sends its credentials: the security scheme of its auth options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthScheme {
    /// No option authenticates (opencode).
    None,
    /// `Authorization: Bearer <value>` (lumaai, isaacus, knock-mgmt).
    Bearer,
    /// `Authorization: Basic <username:password>` (imagekit).
    Basic,
    /// An API key in this header, such as `X-API-Key` (papr).
    Header(String),
    /// An API key sent as this query parameter, such as `key` (OneBusAway).
    Query(String),
}

/// A client option that authenticates, as the tests and the README spell it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthOpt {
    /// Python argument, such as `api_key`.
    pub name: String,
    /// The environment variable it reads, such as `ONEBUSAWAY_API_KEY`.
    pub env: Option<String>,
    /// Its value in the tests, such as `My API Key`.
    pub value: String,
    /// How the README names it, such as `API Key`.
    pub label: String,
    /// Whether it has a config default (imagekit's `password`): the README leaves it out.
    pub has_default: bool,
}

/// The client's credentials: the scheme and the options it reads, in config order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    pub scheme: AuthScheme,
    pub opts: Vec<AuthOpt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryFormat {
    pub array_format: ArrayFormat,
    pub object_format: NestedFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub openapi_spec_url: String,
    pub openapi_spec_hash: String,
    pub config_hash: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFilesInput {
    pub edition: PythonEdition,
    /// Python import name, such as `onebusaway`.
    pub package_name: String,
    /// PyPI distribution name, such as `onebusaway`.
    pub distribution_name: String,
    /// Client class, such as `OnebusawaySDK`.
    pub client_class: String,
    /// Prefix of the SDK's own env vars (`_BASE_URL`, `_LOG`), such as `ONEBUSAWAY_SDK`.
    pub env_prefix: String,
    /// GitHub `owner/repo`, such as `OneBusAway/python-sdk`.
    pub repo: String,
    /// Package version, such as `1.30.0`.
    pub version: String,
    /// Year in the LICENSE copyright line (an input: the plugin has no clock).
    pub copyright_year: u32,
    pub organization: Organization,
    /// First heading of README.md without " Python API library", such as `OneBusAway SDK`.
    pub readme_title: String,
    /// `readme.include_stainless_attribution` (default true): README.md says what generated it.
    pub readme_attribution: bool,
    /// Not `settings.disable_mock_tests`: the tests run against a mock server (`scripts/mock`).
    pub mock_tests: bool,
    pub auth: Auth,
    pub query: QueryFormat,
    pub stats: Stats,
    pub readme_example: ExampleRequest,
    /// The README example with its required arguments only.
    pub retry_probe: ExampleRequest,
    /// Names of `types.shared`, for the "Shared Types" section of api.md.
    pub shared_types: Vec<String>,
    pub resources: Vec<FilesResource>,
}
