//! Test helpers for the project-file emitters: the camelCase JSON of the TypeScript
//! `ProjectFilesInput` mapped to the Rust struct, and the TypeScript emitter run under node so
//! the port can be compared with it byte for byte.

use std::path::{Path, PathBuf};

use ferrule_python_plugin::emit::files::input::{
    Auth, AuthOpt, AuthScheme, ExampleArg, ExampleRequest, FilesMethod, FilesParam, FilesResource, Organization,
    ParamLocation, ProjectFilesInput, PyName, PythonEdition, QueryFormat, Stats,
};
use ferrule_python_plugin::model::{
    ArrayFormat, HttpVerb, NestedFormat, SdkField, SdkLiteral, SdkObject, SdkPrimitive, SdkRef, SdkType,
};
use serde_json::Value;

/// The repository root.
pub(crate) fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn get<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or_else(|| panic!("missing `{key}` in {v}"))
}

fn string(v: &Value, key: &str) -> String {
    get(v, key)
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
        .to_owned()
}

fn opt_string(v: &Value, key: &str) -> Option<String> {
    v.get(key).map(|_| string(v, key))
}

fn boolean(v: &Value, key: &str) -> bool {
    get(v, key)
        .as_bool()
        .unwrap_or_else(|| panic!("`{key}` is not a boolean in {v}"))
}

fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    get(v, key)
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is not an array in {v}"))
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    array(v, key)
        .iter()
        .map(|s| s.as_str().expect("a string").to_owned())
        .collect()
}

fn verb(v: &Value, key: &str) -> HttpVerb {
    match string(v, key).as_str() {
        "get" => HttpVerb::Get,
        "post" => HttpVerb::Post,
        "put" => HttpVerb::Put,
        "patch" => HttpVerb::Patch,
        "delete" => HttpVerb::Delete,
        other => panic!("unknown verb {other}"),
    }
}

fn sdk_ref(v: &Value) -> SdkRef {
    SdkRef {
        name: string(v, "name"),
        module: string(v, "module"),
    }
}

/// An `SdkType` from its TypeScript JSON (`{ kind: ... }`).
pub(crate) fn sdk_type(v: &Value) -> SdkType {
    match string(v, "kind").as_str() {
        "primitive" => SdkType::Primitive(match string(v, "name").as_str() {
            "string" => SdkPrimitive::String,
            "integer" => SdkPrimitive::Integer,
            "number" => SdkPrimitive::Number,
            "boolean" => SdkPrimitive::Boolean,
            "null" => SdkPrimitive::Null,
            "datetime" => SdkPrimitive::Datetime,
            "date" => SdkPrimitive::Date,
            other => panic!("unknown primitive {other}"),
        }),
        "unknown" => SdkType::Unknown,
        "array" => SdkType::Array(Box::new(sdk_type(get(v, "items")))),
        "map" => SdkType::Map(Box::new(sdk_type(get(v, "values")))),
        "union" => SdkType::Union {
            variants: array(v, "variants").iter().map(sdk_type).collect(),
            discriminator: opt_string(v, "discriminator"),
        },
        "literal" => SdkType::Literal(
            array(v, "values")
                .iter()
                .map(|l| match l {
                    Value::String(s) => SdkLiteral::String(s.clone()),
                    Value::Bool(b) => SdkLiteral::Boolean(*b),
                    Value::Number(n) => SdkLiteral::Number(n.as_f64().expect("a number")),
                    other => panic!("bad literal {other}"),
                })
                .collect(),
        ),
        "ref" => SdkType::Ref(sdk_ref(v)),
        "object" => SdkType::Object(SdkObject {
            bases: array(v, "bases").iter().map(sdk_ref).collect(),
            fields: array(v, "fields")
                .iter()
                .map(|f| SdkField {
                    wire_name: string(f, "wireName"),
                    py_name: string(f, "pyName"),
                    required: boolean(f, "required"),
                    description: string(f, "description"),
                    ty: sdk_type(get(f, "type")),
                })
                .collect(),
            description: opt_string(v, "description"),
            hint: opt_string(v, "hint"),
            extra: None,
        }),
        other => panic!("unknown type kind {other}"),
    }
}

fn param(v: &Value) -> FilesParam {
    FilesParam {
        py_name: string(v, "pyName"),
        wire_name: string(v, "wireName"),
        required: boolean(v, "required"),
        location: match string(v, "location").as_str() {
            "path" => ParamLocation::Path,
            "query" => ParamLocation::Query,
            "body" => ParamLocation::Body,
            "header" => ParamLocation::Header,
            other => panic!("unknown location {other}"),
        },
        ty: sdk_type(get(v, "type")),
        example: v.get("example").cloned(),
        minimum: v.get("minimum").map(|n| n.as_f64().expect("a number")),
        maximum: v.get("maximum").map(|n| n.as_f64().expect("a number")),
    }
}

fn method(v: &Value) -> FilesMethod {
    FilesMethod {
        name: string(v, "name"),
        http_method: verb(v, "httpMethod"),
        path: string(v, "path"),
        params: array(v, "params").iter().map(param).collect(),
        params_module: opt_string(v, "paramsModule"),
        response: v.get("response").map(|r| PyName {
            name: string(r, "name"),
            module: string(r, "module"),
        }),
        stream_switch: opt_string(v, "streamSwitch"),
    }
}

fn resource(v: &Value) -> FilesResource {
    FilesResource {
        name: string(v, "name"),
        title: string(v, "title"),
        variable: string(v, "variable"),
        module: string(v, "module"),
        types: strings(v, "types"),
        types_module: string(v, "typesModule"),
        methods: array(v, "methods").iter().map(method).collect(),
        subresources: array(v, "subresources").iter().map(resource).collect(),
        standalone_link: None,
    }
}

/// An example request of the TypeScript input: `args` is rendered keyword arguments
/// (`a="x", b=1`, each required); the README assigns the result to the last accessor segment and
/// prints nothing.
fn example_request(v: &Value) -> ExampleRequest {
    let accessor = string(v, "accessor");
    let var = accessor.rsplit('.').next().unwrap_or(&accessor).to_owned();
    let args = string(v, "args");
    ExampleRequest {
        method: string(v, "method"),
        args: args
            .split(", ")
            .filter(|a| !a.is_empty())
            .map(|a| {
                let (name, value) = a.split_once('=').expect("name=value");
                ExampleArg {
                    name: name.to_owned(),
                    value: value.to_owned(),
                    readme_value: value.to_owned(),
                    required: true,
                }
            })
            .collect(),
        http_method: verb(v, "httpMethod"),
        path: string(v, "path"),
        result: var.clone(),
        parsed: var,
        print_property: None,
        parsed_property: None,
        paginated: false,
        accessor,
    }
}

/// A `ProjectFilesInput` from the JSON of the TypeScript one.
pub(crate) fn input(v: &Value) -> ProjectFilesInput {
    assert_eq!(string(v, "edition"), "python.2025-10-08");
    let org = get(v, "organization");
    let auth = get(v, "auth");
    assert_eq!(string(auth, "kind"), "api_key_query");
    let query = get(v, "query");
    let stats = get(v, "stats");
    ProjectFilesInput {
        edition: PythonEdition::Python20251008,
        package_name: string(v, "packageName"),
        distribution_name: string(v, "distributionName"),
        client_class: string(v, "clientClass"),
        env_prefix: string(v, "envPrefix"),
        repo: string(v, "repo"),
        version: string(v, "version"),
        copyright_year: u32::try_from(get(v, "copyrightYear").as_u64().expect("a year")).expect("a year"),
        organization: Organization {
            name: string(org, "name"),
            title: string(org, "title"),
            contact: string(org, "contact"),
            docs_url: string(org, "docsUrl"),
            security_contact: string(org, "contact"),
        },
        readme_title: string(v, "readmeTitle"),
        readme_attribution: true,
        mock_tests: true,
        // `api_key_query`: the `api_key` option, sent as a query parameter.
        auth: Auth {
            scheme: AuthScheme::Query(string(auth, "param")),
            opts: vec![AuthOpt {
                name: "api_key".to_owned(),
                env: Some(string(auth, "env")),
                value: "My API Key".to_owned(),
                label: "API Key".to_owned(),
                has_default: false,
            }],
        },
        query: QueryFormat {
            array_format: match string(query, "arrayFormat").as_str() {
                "comma" => ArrayFormat::Comma,
                "repeat" => ArrayFormat::Repeat,
                "indices" => ArrayFormat::Indices,
                "brackets" => ArrayFormat::Brackets,
                other => panic!("unknown array format {other}"),
            },
            object_format: match string(query, "objectFormat").as_str() {
                "brackets" => NestedFormat::Brackets,
                "dots" => NestedFormat::Dots,
                other => panic!("unknown object format {other}"),
            },
        },
        stats: Stats {
            openapi_spec_url: string(stats, "openapiSpecUrl"),
            openapi_spec_hash: string(stats, "openapiSpecHash"),
            config_hash: string(stats, "configHash"),
        },
        readme_example: example_request(get(v, "readmeExample")),
        retry_probe: example_request(get(v, "retryProbe")),
        shared_types: strings(v, "sharedTypes"),
        resources: array(v, "resources").iter().map(resource).collect(),
    }
}

/// The OneBusAway input the TypeScript tests use, as JSON.
pub(crate) fn onebusaway_json() -> Value {
    let path = repo().join("crates/python-plugin/tests/files/onebusaway-input.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid JSON")
}
