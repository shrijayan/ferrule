//! The generated files of `Core/`: `ClientOptions.cs` (the client settings and options),
//! `EnvironmentUrl.cs` (the environments) and `ModelBase.cs` (the JSON options, which list a
//! converter for every enum).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::doc;
use crate::emit::client::opts;
use crate::emit::model::sort_usings;
use crate::emit::{Gen, template};
use crate::naming::{Casings, pascal, words};
use crate::tree::{Decl, Kind};
use crate::types::EnumValue;

/// The default environment's property on `EnvironmentUrl` and its words: `Production`.
fn default_env(g: &Gen<'_>) -> (String, String) {
    let casings = Casings(&g.client.casings);
    let Some(s) = g.client.settings.as_ref() else {
        return ("Production".to_owned(), "production".to_owned());
    };
    let env = s
        .default_environment
        .as_ref()
        .and_then(|d| s.environments.iter().find(|e| &e.name.written == d))
        .or_else(|| s.environments.first());
    env.map_or_else(
        || ("Production".to_owned(), "production".to_owned()),
        |e| (pascal(&words(&e.name), casings), words(&e.name).join(" ")),
    )
}

/// `TimeSpan.FromMinutes(1)` for 60 seconds.
fn timeout(seconds: f64) -> String {
    if seconds > 0.0 && seconds % 60.0 == 0.0 {
        format!("TimeSpan.FromMinutes({})", seconds / 60.0)
    } else {
        format!("TimeSpan.FromSeconds({seconds})")
    }
}

/// `Core/ClientOptions.cs`.
#[must_use]
pub fn client_options(g: &Gen<'_>) -> (String, String) {
    let t = g.target;
    let (env, env_words) = default_env(g);
    let settings = g.client.settings.as_ref();
    let mut out = template(include_str!("client_options.cs.txt"))
        .replace("__ENV__", &t.env_prefix)
        .replace("__NS__", &t.ns)
        .replace("__CLIENT__", &t.client)
        .replace("__DEFAULT_ENV__", &env)
        .replace("__DEFAULT_ENV_WORDS__", &env_words)
        .replace("__MAX_RETRIES__", &settings.map_or(2, |s| s.max_retries).to_string())
        .replace("__TIMEOUT__", &timeout(settings.map_or(60.0, |s| s.timeout_seconds)));
    for o in opts(g) {
        let field = format!("_{}", crate::naming::decapitalize(&o.name));
        let summary = doc::summary(&o.description, 4, doc::Place::Opt, &[]);
        let read = o
            .env
            .as_ref()
            .map(|e| format!("Environment.GetEnvironmentVariable(\"{e}\")"));
        let value = match (read, &o.default, o.nullable) {
            (Some(r), Some(d), _) => format!("{r} ?? {d}"),
            (Some(r), None, true) => r,
            (Some(r), None, false) => format!(
                "{r}\n        ?? throw new {c}InvalidDataException(\n            string.Format(\"{{0}} cannot be null\", nameof({n})),\n            new ArgumentNullException(nameof({n}))\n        )",
                c = t.client,
                n = o.name
            ),
            (None, Some(d), _) => d.clone(),
            (None, None, _) => "null".to_owned(),
        };
        let _ = write!(
            out,
            "\n{summary}    Lazy<{ty}> {field} = new(() =>\n        {value}\n    );\n\n{summary}    public {ty} {name}\n    {{\n        readonly get {{ return {field}.Value; }}\n        set {{ {field} = new(() => value); }}\n    }}\n",
            ty = o.ty,
            name = o.name
        );
    }
    out.push_str("}\n");
    (format!("src/{}/Core/ClientOptions.cs", t.ns), out)
}

/// `Core/EnvironmentUrl.cs`.
#[must_use]
pub fn environment_url(g: &Gen<'_>) -> (String, String) {
    let casings = Casings(&g.client.casings);
    let mut out = format!(
        "namespace {}.Core;\n\npublic static class EnvironmentUrl\n{{\n",
        g.target.ns
    );
    let envs = g.client.settings.iter().flat_map(|s| &s.environments);
    for (i, e) in envs.enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(
            out,
            "    public static readonly string {} = \"{}\";",
            pascal(&words(&e.name), casings),
            e.url
        );
    }
    out.push_str("}\n");
    (format!("src/{}/Core/EnvironmentUrl.cs", g.target.ns), out)
}

/// Every enum of the SDK in the order `ModelBase` lists their converters: shared models first,
/// then each resource's named models, then its methods' responses and params.
fn enums(g: &Gen<'_>) -> Vec<(Vec<String>, String, EnumValue)> {
    let mut by_ns: Vec<Vec<String>> = vec![Vec::new()];
    for s in &g.services {
        by_ns.push(s.model_ns.clone());
    }
    let mut out = Vec::new();
    for ns in by_ns {
        let models = g.files.files.iter().filter(|f| f.ns == ns && f.kind == Kind::Model);
        let mut methods: Vec<&crate::tree::ModelFile> = g
            .files
            .files
            .iter()
            .filter(|f| f.ns == ns && f.kind != Kind::Model)
            .collect();
        // Within a method, its response comes before its params.
        methods.sort_by_key(|f| match f.kind {
            Kind::Response(s, m) => (s, m, 0),
            Kind::Params(s, m) => (s, m, 1),
            Kind::Model => (0, 0, 2),
        });
        for f in models.chain(methods) {
            for d in &f.decls {
                if let Decl::Enum(e) = d
                    && e.value != EnumValue::Double
                {
                    out.push((f.ns.clone(), e.name.clone(), e.value));
                }
            }
        }
    }
    out
}

/// `Core/ModelBase.cs`.
#[must_use]
pub fn model_base(g: &Gen<'_>) -> (String, String) {
    let t = g.target;
    let list = enums(g);
    // A namespace is aliased when one of its enums has the name of an enum listed before it.
    let mut first: HashMap<String, Vec<String>> = HashMap::new();
    let mut clashing: HashSet<Vec<String>> = HashSet::new();
    for (ns, name, _) in &list {
        match first.get(name) {
            Some(owner) if owner != ns && !ns.is_empty() => {
                clashing.insert(ns.clone());
            }
            Some(_) => {}
            None => {
                first.insert(name.clone(), ns.clone());
            }
        }
    }
    let mut aliases: HashMap<Vec<String>, String> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut usings = vec![format!("{}.Exceptions", t.ns), "System.Text.Json".to_owned()];
    let mut converters = String::new();
    for (ns, name, value) in &list {
        let spelled = if clashing.contains(ns) {
            let alias = aliases
                .entry(ns.clone())
                .or_insert_with(|| {
                    let leaf = ns.last().cloned().unwrap_or_default();
                    let picked = if taken.contains(&leaf) && ns.len() > 1 {
                        format!("{}{leaf}", ns[ns.len() - 2])
                    } else {
                        leaf
                    };
                    taken.insert(picked.clone());
                    picked
                })
                .clone();
            format!("{alias}::{name}")
        } else {
            usings.push(g.models_ns(ns));
            name.clone()
        };
        let _ = writeln!(
            converters,
            "            new ApiEnumConverter<{}, {spelled}>(),",
            value.raw()
        );
    }
    for (ns, alias) in &aliases {
        usings.push(format!("{alias} = {}", g.models_ns(ns)));
    }
    let body = template(include_str!("model_base.cs.txt"))
        .replace("__CONVERTERS__", &converters)
        .replace("__NS__", &t.ns)
        .replace("__CLIENT__", &t.client);
    (
        format!("src/{}/Core/ModelBase.cs", t.ns),
        format!("{}\n{body}", sort_usings(&usings)),
    )
}
