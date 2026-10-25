//! What the project-file emitters need, as a narrow type of its own, so they don't depend on the
//! whole SDK model. Port of `python-plugin/src/emit/files/input.ts`. Everything is already a
//! Python or text spelling.

use serde_json::Value;

use crate::model::{ArrayFormat, HttpVerb, NestedFormat, SdkType};

/// Config editions the templates were extracted for (OneBusAway: Rye, Python 3.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonEdition {
    /// `python.2025-10-08`.
    Python20251008,
}

/// One request used as an example in README.md, or as the retry probe in `tests/test_client.py`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleRequest {
    /// Dotted resource accessor from the client, such as `current_time` or `chat.completions`.
    pub accessor: String,
    /// Python method name, such as `retrieve`.
    pub method: String,
    /// Python keyword arguments, already rendered, such as `stop_id="1_75403"`; empty for none.
    pub args: String,
    pub http_method: HttpVerb,
    /// The path as respx matches it, such as `/api/where/current-time.json`.
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    Path,
    Query,
    Body,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    /// `organization.name`, such as `onebusaway-sdk`.
    pub name: String,
    /// Display name, such as `Onebusaway SDK`.
    pub title: String,
    pub contact: String,
    pub docs_url: String,
}

/// The API key client option: env var it reads and the query parameter it is sent as
/// (`kind: 'api_key_query'`, the only kind today).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyQueryAuth {
    pub env: String,
    pub param: String,
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
    pub auth: ApiKeyQueryAuth,
    pub query: QueryFormat,
    pub stats: Stats,
    pub readme_example: ExampleRequest,
    pub retry_probe: ExampleRequest,
    /// Names of `types.shared`, for the "Shared Types" section of api.md.
    pub shared_types: Vec<String>,
    pub resources: Vec<FilesResource>,
}
