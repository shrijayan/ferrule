//! Maps [`SdkClient`] to the project-file emitters' input ([`super::files::input`]). Values the
//! model cannot know yet come in [`ProjectMeta`]: `x-ferrule-config` does not carry them (proposed
//! ADR 0003 amendment), so the plugin passes deterministic placeholders until it does.

use serde_json::{Map, Value};

use super::common::{
    example_type, header_params, is_binary_body, leaf, method_params, method_ref, resource_class_base, resource_module,
    resource_tree, return_model, sort_fields, stainless_method_order, stream_switch, types_dir,
};
use super::files::input::{
    Auth, AuthOpt, AuthScheme, ExampleRequest, FilesMethod, FilesParam, FilesResource, Organization, ParamLocation,
    ProjectFilesInput, PyName, PythonEdition, QueryFormat, Stats,
};
use super::files_example::examples;
use crate::model::{
    ApiKeyLocation, HttpAuthScheme, SdkClient, SdkClientSettings, SdkMethod, SdkModelDecl, SdkParam, SdkResource,
    SdkSecurityScheme, SendLocation,
};
use crate::naming::{singular, snake_case, split_words};

/// The organization fields of [`ProjectMeta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaOrganization {
    pub title: String,
    pub contact: String,
    pub docs_url: String,
    /// `organization.security_contact`; `None` names `contact` in SECURITY.md.
    pub security_contact: Option<String>,
}

/// `readme.example_requests.default` of the config: `endpoint` such as `post /embeddings`, and
/// the request's parameters by wire name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadmeRequest {
    pub endpoint: String,
    pub params: Map<String, Value>,
}

/// What the project files need that the annotated spec does not carry yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMeta {
    pub version: String,
    pub copyright_year: u32,
    pub distribution_name: Option<String>,
    pub organization: Option<MetaOrganization>,
    pub readme_title: Option<String>,
    pub stats: Option<Stats>,
    /// A ready README example; it takes precedence over `readme_request`.
    pub readme_example: Option<ExampleRequest>,
    /// `readme.example_requests.default`; else the first method without required parameters,
    /// else the first method with its required parameters from the spec's examples.
    pub readme_request: Option<ReadmeRequest>,
    /// `readme.include_stainless_attribution` (default true).
    pub readme_attribution: bool,
    /// Not `settings.disable_mock_tests` (default false).
    pub mock_tests: bool,
}

/// Placeholders the plugin uses until `x-ferrule-config` carries the real values.
#[must_use]
pub fn placeholder_meta() -> ProjectMeta {
    ProjectMeta {
        version: "0.0.1".to_owned(),
        copyright_year: 2026,
        distribution_name: None,
        organization: None,
        readme_title: None,
        stats: None,
        readme_example: None,
        readme_request: None,
        readme_attribution: true,
        mock_tests: true,
    }
}

/// `API Key` for `api_key`: the option's words capitalized, `API` in capitals.
fn auth_label(name: &str) -> String {
    split_words(name)
        .iter()
        .map(|w| {
            if w == "api" {
                "API".to_owned()
            } else {
                let mut c = w.chars();
                c.next()
                    .map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The client's credentials: the options that authenticate (`auth`, or sent as a query
/// parameter), and how the first one is sent.
fn auth(s: &SdkClientSettings) -> Auth {
    let opts: Vec<AuthOpt> = s
        .opts
        .iter()
        .filter(|o| o.auth.is_some() || o.send.as_ref().is_some_and(|x| x.location == SendLocation::Query))
        .map(|o| {
            let label = auth_label(&o.name);
            AuthOpt {
                name: o.name.clone(),
                env: o.read_env.clone(),
                value: format!("My {label}"),
                label,
                has_default: o.default.is_some(),
                nullable: o.nullable,
            }
        })
        .collect();
    let first = s
        .opts
        .iter()
        .find(|o| o.auth.is_some() || o.send.as_ref().is_some_and(|x| x.location == SendLocation::Query));
    let scheme = first.map_or(AuthScheme::None, |o| {
        if let Some(send) = o.send.as_ref().filter(|x| x.location == SendLocation::Query) {
            return AuthScheme::Query(send.name.clone());
        }
        let scheme = o
            .auth
            .as_ref()
            .and_then(|a| s.security_schemes.iter().find(|(name, _)| *name == a.security_scheme))
            .map(|(_, scheme)| scheme);
        match scheme {
            Some(SdkSecurityScheme::Http {
                scheme: HttpAuthScheme::Basic,
                ..
            }) => AuthScheme::Basic,
            Some(SdkSecurityScheme::ApiKey {
                location: ApiKeyLocation::Header,
                name,
            }) => AuthScheme::Header(name.clone()),
            Some(SdkSecurityScheme::ApiKey {
                location: ApiKeyLocation::Query,
                name,
            }) => AuthScheme::Query(name.clone()),
            Some(SdkSecurityScheme::Http { .. }) => AuthScheme::Bearer,
            _ => AuthScheme::None,
        }
    });
    Auth { scheme, opts }
}

fn files_param(models: &[SdkModelDecl], p: &SdkParam, location: ParamLocation, binary: bool) -> FilesParam {
    FilesParam {
        py_name: p.py_name.clone(),
        wire_name: p.wire_name.clone(),
        required: p.required,
        location,
        ty: example_type(&p.ty, models),
        example: p.example.clone(),
        minimum: p.minimum,
        maximum: p.maximum,
        binary,
    }
}

fn files_method(models: &[SdkModelDecl], resource: &SdkResource, m: &SdkMethod) -> FilesMethod {
    let mut params: Vec<FilesParam> = m
        .path_params
        .iter()
        .map(|p| files_param(models, p, ParamLocation::Path, false))
        .collect();
    params.extend(
        sort_fields(m.query_params.iter().chain(&m.body_params))
            .into_iter()
            .map(|p| {
                let in_query = m.query_params.iter().any(|q| std::ptr::eq(q, p));
                files_param(
                    models,
                    p,
                    if in_query {
                        ParamLocation::Query
                    } else {
                        ParamLocation::Body
                    },
                    is_binary_body(m, p),
                )
            }),
    );
    params.extend(
        header_params(m)
            .into_iter()
            .map(|p| files_param(models, p, ParamLocation::Header, false)),
    );
    let ret = return_model(Some(resource), m);
    FilesMethod {
        name: m.name.clone(),
        http_method: m.http_method,
        path: m.path.clone(),
        params,
        params_module: if method_params(m).is_empty() {
            None
        } else {
            Some(format!("types.{}", method_ref(Some(resource), m, "Params").module))
        },
        response: ret.map(|r| PyName {
            name: r.r#ref.name,
            module: format!("types.{}", r.r#ref.module),
        }),
        stream_switch: stream_switch(m).map(|q| q.py_name.clone()),
        page: m.pagination.as_ref().map(|p| p.page.clone()),
        streaming: m.streaming.is_some() && stream_switch(m).is_none(),
        skip_test: m.skip_test.clone(),
    }
}

/// Where the root api.md links a standalone API: `api.md` in the directory of its parent's
/// module, as the oracles have it (`src/isaacus/resources/extractions/api.md` for
/// `extractions.qa`, `src/isaacus/resources/api.md` for `ilgs.v1`, whose parent has no class).
fn standalone_link(client: &SdkClient, parent: Option<&SdkResource>) -> String {
    let module = parent.map_or_else(|| "resources.__init__".to_owned(), resource_module);
    let dir = module.rsplit_once('.').map_or("", |(d, _)| d);
    format!("src/{}/{}/api.md", client.package.name, dir.replace('.', "/"))
}

fn files_resource(client: &SdkClient, r: &SdkResource, parent: Option<&SdkResource>) -> FilesResource {
    let name = leaf(r).to_owned();
    let methods = stainless_method_order(&r.methods);
    let dir = types_dir(r).join(".");
    // A standalone API's directory is its own, so its named models are listed too.
    let mut types: Vec<String> = Vec::new();
    if r.standalone {
        let mut named: Vec<&str> = client
            .models
            .iter()
            .filter(|d| d.r#ref.module.rsplit_once('.').is_some_and(|(d, _)| d == dir))
            .map(|d| d.r#ref.name.as_str())
            .collect();
        named.sort_unstable();
        types.extend(named.into_iter().map(str::to_owned));
    }
    types.extend(methods.iter().filter_map(|m| {
        return_model(Some(r), m)
            .filter(|ret| ret.inline.is_some())
            .map(|ret| ret.r#ref.name)
    }));
    let mut types_module = vec!["types"];
    types_module.extend(types_dir(r).iter().map(String::as_str));
    FilesResource {
        title: resource_class_base(r),
        variable: snake_case(&singular(&split_words(&name))),
        module: resource_module(r),
        types,
        types_module: types_module.join("."),
        methods: methods.iter().map(|m| files_method(&client.models, r, m)).collect(),
        subresources: r
            .subresources
            .iter()
            .map(|sub| files_resource(client, sub, Some(r)))
            .collect(),
        standalone_link: r.standalone.then(|| standalone_link(client, parent)),
        name,
    }
}

/// The project-file input, or `None` for a client without settings or without a method.
///
/// The templates are today's generator's, so a runtime without `omit` (`sentinel: NOT_GIVEN`)
/// gets none: knock-python's `tests/test_transform.py` imports `NOT_GIVEN` and tests
/// `PYDANTIC_V2`, where the template imports `omit`/`not_given` and tests `PYDANTIC_V1`.
#[must_use]
pub fn to_project_files_input(client: &SdkClient, meta: &ProjectMeta) -> Option<ProjectFilesInput> {
    if !client.style.omit {
        return None;
    }
    let s = client.settings.as_ref()?;
    let (example, probe) = examples(client, meta.readme_example.as_ref(), meta.readme_request.as_ref())?;
    let org = meta.organization.as_ref();
    let contact = org.map_or_else(String::new, |o| o.contact.clone());
    Some(ProjectFilesInput {
        edition: PythonEdition::Python20251008,
        package_name: client.package.name.clone(),
        distribution_name: meta
            .distribution_name
            .clone()
            .unwrap_or_else(|| client.package.name.clone()),
        client_class: s.class_name.clone(),
        env_prefix: s.env_prefix.clone(),
        repo: client.package.repo.clone(),
        version: meta.version.clone(),
        copyright_year: meta.copyright_year,
        organization: Organization {
            name: s.name.clone(),
            title: org.map_or_else(|| s.class_name.clone(), |o| o.title.clone()),
            security_contact: org
                .and_then(|o| o.security_contact.clone())
                .unwrap_or_else(|| contact.clone()),
            contact,
            docs_url: org.map_or_else(String::new, |o| o.docs_url.clone()),
        },
        readme_title: meta.readme_title.clone().unwrap_or_else(|| s.class_name.clone()),
        readme_attribution: meta.readme_attribution,
        mock_tests: meta.mock_tests,
        auth: auth(s),
        query: QueryFormat {
            array_format: s.array_format,
            object_format: s.nested_format,
        },
        stats: meta.stats.clone().unwrap_or(Stats {
            openapi_spec_url: String::new(),
            openapi_spec_hash: String::new(),
            config_hash: String::new(),
        }),
        retry_probe: probe,
        readme_example: example,
        shared_types: client
            .models
            .iter()
            .filter(|d| d.r#ref.module.starts_with("shared."))
            .map(|d| d.r#ref.name.clone())
            .collect(),
        resources: resource_tree(client)
            .iter()
            .map(|r| files_resource(client, r, None))
            .collect(),
    })
}
