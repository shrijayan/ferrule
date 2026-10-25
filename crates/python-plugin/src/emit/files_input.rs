//! Maps [`SdkClient`] to the project-file emitters' input ([`super::files::input`]). Values the
//! model cannot know yet come in [`ProjectMeta`]: `x-ferrule-config` does not carry them (proposed
//! ADR 0003 amendment), so the plugin passes deterministic placeholders until it does.

use super::common::{
    leaf, method_params, method_ref, resource_class_base, resource_module, resource_tree, return_model, sort_fields,
    stainless_method_order, types_dir,
};
use super::files::input::{
    ApiKeyQueryAuth, ExampleRequest, FilesMethod, FilesParam, FilesResource, Organization, ParamLocation,
    ProjectFilesInput, PyName, PythonEdition, QueryFormat, Stats,
};
use crate::model::{SdkClient, SdkMethod, SdkParam, SdkResource, SendLocation};
use crate::naming::{singular, snake_case, split_words};

/// The organization fields of [`ProjectMeta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaOrganization {
    pub title: String,
    pub contact: String,
    pub docs_url: String,
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
    /// `readme.example_requests.default`; else the first method without required parameters.
    pub readme_example: Option<ExampleRequest>,
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
    }
}

fn files_param(p: &SdkParam, location: ParamLocation) -> FilesParam {
    FilesParam {
        py_name: p.py_name.clone(),
        wire_name: p.wire_name.clone(),
        required: p.required,
        location,
        ty: p.ty.clone(),
        example: p.example.clone(),
        minimum: p.minimum,
        maximum: p.maximum,
    }
}

fn files_method(resource: &SdkResource, m: &SdkMethod) -> FilesMethod {
    let mut params: Vec<FilesParam> = m
        .path_params
        .iter()
        .map(|p| files_param(p, ParamLocation::Path))
        .collect();
    params.extend(
        sort_fields(m.query_params.iter().chain(&m.body_params))
            .into_iter()
            .map(|p| {
                let in_query = m.query_params.iter().any(|q| std::ptr::eq(q, p));
                files_param(
                    p,
                    if in_query {
                        ParamLocation::Query
                    } else {
                        ParamLocation::Body
                    },
                )
            }),
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
        methods: methods.iter().map(|m| files_method(r, m)).collect(),
        subresources: r
            .subresources
            .iter()
            .map(|sub| files_resource(client, sub, Some(r)))
            .collect(),
        standalone_link: r.standalone.then(|| standalone_link(client, parent)),
        name,
    }
}

fn first_example(resources: &[SdkResource]) -> Option<ExampleRequest> {
    for r in resources {
        for m in stainless_method_order(&r.methods) {
            if m.path_params.is_empty() && method_params(m).iter().all(|p| !p.required) {
                return Some(ExampleRequest {
                    accessor: r.path.join("."),
                    method: m.name.clone(),
                    args: String::new(),
                    http_method: m.http_method,
                    path: m.path.clone(),
                });
            }
        }
        if let Some(nested) = first_example(&r.subresources) {
            return Some(nested);
        }
    }
    None
}

/// The project-file input, or `None` for a client the file templates cannot describe yet.
#[must_use]
pub fn to_project_files_input(client: &SdkClient, meta: &ProjectMeta) -> Option<ProjectFilesInput> {
    let s = client.settings.as_ref()?;
    let key = s
        .opts
        .iter()
        .find(|o| o.send.as_ref().is_some_and(|x| x.location == SendLocation::Query) && o.read_env.is_some())?;
    let send = key.send.as_ref()?;
    let env = key.read_env.as_ref()?;
    let example = meta
        .readme_example
        .clone()
        .or_else(|| first_example(&client.resources))?;
    let org = meta.organization.as_ref();
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
            contact: org.map_or_else(String::new, |o| o.contact.clone()),
            docs_url: org.map_or_else(String::new, |o| o.docs_url.clone()),
        },
        readme_title: meta.readme_title.clone().unwrap_or_else(|| s.class_name.clone()),
        auth: ApiKeyQueryAuth {
            env: env.clone(),
            param: send.name.clone(),
        },
        query: QueryFormat {
            array_format: s.array_format,
            object_format: s.nested_format,
        },
        stats: meta.stats.clone().unwrap_or(Stats {
            openapi_spec_url: String::new(),
            openapi_spec_hash: String::new(),
            config_hash: String::new(),
        }),
        retry_probe: example.clone(),
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
