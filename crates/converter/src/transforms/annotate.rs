//! Annotate the spec from the config: Fern grouping extensions and `x-ferrule-*` (ADR 0003).

use super::targets::{get_at, get_at_mut};
use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::editions::apply_edition;
use crate::extensions::casings::{Casings, read_casings};
use crate::extensions::client::build_client;
use crate::extensions::groups::{CLIENT_GROUP, collides_with_client_group, forge_group_path};
use crate::extensions::json::{empty, obj, obj_of};
use crate::extensions::pagination::{Scheme, best_match, operation_shape, read_schemes};
use crate::extensions::streaming::{StreamingInput, build_streaming, read_on_event};
use crate::value::{Map, Value, is_js_space};
use std::collections::HashSet;

struct Ctx<'c> {
    config: &'c Value,
    diagnostics: Vec<Diagnostic>,
    /// Schema name -> `{ name, module }`.
    models: Map,
    /// Each annotated operation, in annotation order, with the extensions of every method bound
    /// to it (more than one becomes `x-forge-aliases`).
    bindings: Vec<(Vec<Seg>, Vec<Map>)>,
    schemes: Vec<Scheme>,
    on_event: Vec<Value>,
    casings: Casings,
    /// `{ path, order }` of every `standalone_api: true` resource, in config order.
    standalone: Vec<Value>,
}

fn resource_pointer(path: &[String]) -> Vec<Seg> {
    let mut out = vec![Seg::from("resources")];
    for (i, seg) in path.iter().enumerate() {
        if i > 0 {
            out.push(Seg::from("subresources"));
        }
        out.push(Seg::from(seg.as_str()));
    }
    out
}

fn method_pointer(path: &[String], name: &str) -> String {
    let mut p = resource_pointer(path);
    p.push(Seg::from("methods"));
    p.push(Seg::from(name));
    to_json_pointer(&p)
}

/// Follow a local `$ref` chain from `start`, returning where the final node lives.
fn deref_path(spec: &Value, start: Vec<Seg>) -> Option<Vec<Seg>> {
    let mut path = start;
    let mut seen = HashSet::new();
    loop {
        let node = get_at(spec, &path)?;
        let Some(Value::String(r)) = node.get("$ref") else {
            return Some(path);
        };
        if !node.is_object() {
            return Some(path);
        }
        if !r.starts_with("#/") || !seen.insert(r.clone()) {
            return None;
        }
        path = r[2..]
            .split('/')
            .map(|raw| Seg::Key(raw.replace("~1", "/").replace("~0", "~")))
            .collect();
    }
}

/// `^\s*(\w+)\s+(\S+)\s*$`: the verb and route of `"get /path"`.
fn parse_endpoint(endpoint: &str) -> Option<(String, String)> {
    let t = endpoint.trim_matches(is_js_space);
    let verb_end = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(t.len());
    if verb_end == 0 {
        return None;
    }
    let (verb, rest) = t.split_at(verb_end);
    let route = rest.trim_start_matches(is_js_space);
    if route.len() == rest.len() || route.is_empty() || route.contains(is_js_space) {
        return None;
    }
    Some((verb.to_owned(), route.to_owned()))
}

/// The path item and operation an endpoint names, as paths into the spec.
fn find_operation(spec: &Value, endpoint: &str) -> Option<(Vec<Seg>, Vec<Seg>)> {
    let (verb, route) = parse_endpoint(endpoint)?;
    let paths = spec.get("paths").filter(|p| p.is_object())?;
    paths.get(&route)?;
    let item_path = deref_path(spec, vec![Seg::from("paths"), Seg::from(route.as_str())])?;
    let item = get_at(spec, &item_path)?;
    if !item.is_object() {
        return None;
    }
    let mut op_path = item_path.clone();
    op_path.push(Seg::Key(verb.to_lowercase()));
    get_at(spec, &op_path).filter(|op| op.is_object())?;
    Some((item_path, op_path))
}

/// A method: the `"get /path"` shorthand or the object form.
fn endpoint_of(method: &Value) -> Option<&str> {
    match method {
        Value::String(s) => Some(s),
        Value::Object(m) => m.get("endpoint").and_then(Value::as_str),
        _ => None,
    }
}

impl<'c> Ctx<'c> {
    fn new(config: &'c Value) -> Self {
        let (schemes, mut diagnostics) = read_schemes(config);
        let (casings, ds) = read_casings(config);
        diagnostics.extend(ds);
        Ctx {
            config,
            diagnostics,
            models: Map::new(),
            bindings: Vec::new(),
            schemes,
            on_event: read_on_event(config),
            casings,
            standalone: Vec::new(),
        }
    }
}

impl Ctx<'_> {
    fn annotate_method(
        &mut self,
        spec: &mut Value,
        path: &[String],
        name: &str,
        method: &Value,
        resource_order: &[usize],
        method_order: usize,
    ) {
        let pointer = method_pointer(path, name);
        // Methods without an endpoint (`type: webhook_unwrap`) are SDK-side helpers, not operations.
        let Some(endpoint) = endpoint_of(method) else { return };
        let Some((item_path, op_path)) = find_operation(spec, endpoint) else {
            self.diagnostics.push(Diagnostic::error(
                "FER1001",
                pointer,
                format!(
                    "Endpoint \"{endpoint}\" of method {}.{name} is not in the OpenAPI spec.",
                    path.join(".")
                ),
                "Use \"<verb> <path>\" naming an operation in the spec, or remove the method.",
            ));
            return;
        };
        let pagination = {
            let item = get_at(spec, &item_path).and_then(Value::as_object).unwrap_or(empty());
            let op = get_at(spec, &op_path).and_then(Value::as_object).unwrap_or(empty());
            self.resolve_pagination(spec, &pointer, name, method, item, op)
        };
        let streaming = match method {
            Value::Object(m) => {
                let op = get_at(spec, &op_path).and_then(Value::as_object).unwrap_or(empty());
                let (s, ds) = build_streaming(&StreamingInput {
                    spec,
                    config: self.config,
                    op,
                    streaming: m.get("streaming"),
                    pointer: &pointer,
                    on_event: &self.on_event,
                });
                Some((s, ds))
            }
            _ => None,
        };
        if !matches!(get_at(spec, &op_path), Some(Value::Object(_))) {
            return;
        }
        let strings = |v: Vec<String>| Value::Array(v.into_iter().map(Value::String).collect());
        #[expect(clippy::cast_precision_loss, reason = "orders are small indices")]
        let numbers = |v: &[usize]| Value::Array(v.iter().map(|n| Value::Number(*n as f64)).collect());
        let mut def = Map::new();
        def.insert("x-fern-sdk-group-name", strings(forge_group_path(path)));
        def.insert("x-fern-sdk-method-name", Value::from(name));
        let mut resource = Map::new();
        resource.insert("path", strings(path.to_vec()));
        resource.insert("order", numbers(resource_order));
        def.insert("x-ferrule-resource", Value::Object(resource));
        def.insert("x-ferrule-method-name", Value::from(name));
        #[expect(clippy::cast_precision_loss, reason = "orders are small indices")]
        def.insert("x-ferrule-method-order", Value::Number(method_order as f64));
        if let Some(p) = pagination {
            def.insert("x-ferrule-pagination", p);
        }
        if let Some((s, ds)) = streaming {
            self.diagnostics.extend(ds);
            if let Some(s) = s {
                def.insert("x-ferrule-streaming", s);
            }
        }
        match self.bindings.iter_mut().find(|(p, _)| *p == op_path) {
            Some((_, defs)) => defs.push(def),
            None => self.bindings.push((op_path, vec![def])),
        }
    }

    fn resolve_pagination(
        &mut self,
        spec: &Value,
        pointer: &str,
        name: &str,
        method: &Value,
        item: &Map,
        op: &Map,
    ) -> Option<Value> {
        let paginated = match method {
            Value::Object(m) => m.get("paginated"),
            _ => None,
        };
        if paginated == Some(&Value::Bool(false)) {
            return None;
        }
        if let Some(Value::String(wanted)) = paginated {
            if let Some(named) = self.schemes.iter().find(|s| s.page() == wanted) {
                return Some(Value::Object(named.extension.clone()));
            }
            let defined: Vec<&str> = self.schemes.iter().map(Scheme::page).collect();
            let list = defined.join(", ");
            self.diagnostics.push(Diagnostic::error(
                "FER1002",
                format!("{pointer}/paginated"),
                format!("Pagination \"{wanted}\" is not in the top-level pagination list."),
                format!(
                    "Add a pagination entry named \"{wanted}\", or use one of: {}.",
                    if list.is_empty() { "(none defined)" } else { &list }
                ),
            ));
            return None;
        }
        let marked = paginated == Some(&Value::Bool(true));
        if !marked && name != "list" && !name.starts_with("list_") {
            return None;
        }
        let shape = operation_shape(spec, item, op);
        if let Some(scheme) = best_match(spec, &self.schemes, &shape) {
            return Some(Value::Object(scheme.extension.clone()));
        }
        if marked {
            self.diagnostics.push(Diagnostic::error(
                "FER1003",
                format!("{pointer}/paginated"),
                "Method is marked paginated but matches no configured pagination scheme.",
                "Name the scheme with `paginated: <name>`, or align the operation fields with a pagination entry.",
            ));
        }
        None
    }

    /// Models of a resource: `$shared` ones live in `types/shared/`, others under their resource path.
    fn collect_models(&mut self, spec: &Value, path: &[String], resource: &Value) {
        let resource = obj_of(Some(resource));
        for (key, model) in obj(resource, "models") {
            let mut p = resource_pointer(path);
            p.push(Seg::from("models"));
            p.push(Seg::from(key.as_str()));
            let pointer = to_json_pointer(&p);
            let mut dotted = path.to_vec();
            dotted.push(key.clone());
            let model = match model_uri(model) {
                ModelUri::Schema(uri) => uri,
                ModelUri::NotForPython => continue,
                ModelUri::Unsupported => {
                    self.diagnostics.push(Diagnostic::warning(
                        "FER1004",
                        pointer,
                        format!(
                            "Model {} uses an object form that is not supported yet.",
                            dotted.join(".")
                        ),
                        "Write the model as a schema name, e.g. `references: Reference`, or as \
                         `{ openapi_uri: \"#/components/schemas/<Name>\" }`.",
                    ));
                    continue;
                }
            };
            let schema = model.strip_prefix("#/components/schemas/").unwrap_or(model);
            let schemas = spec
                .get("components")
                .map(|c| obj_of(Some(c)))
                .map_or(empty(), |c| obj(c, "schemas"));
            if !schemas.contains_key(schema) {
                self.diagnostics.push(Diagnostic::warning(
                    "FER1005",
                    pointer,
                    format!(
                        "Model {} names schema \"{schema}\", which is not in components.schemas.",
                        dotted.join(".")
                    ),
                    "Name a schema under components.schemas in the OpenAPI spec.",
                ));
                continue;
            }
            // types/<parent dirs>/<key>.py: a resource's models sit in its parent's directory, a
            // standalone API's in its own.
            let mut dirs: Vec<String> = if path.first().is_some_and(|p| p == "$shared") {
                vec!["shared".to_owned()]
            } else if is_standalone(resource) {
                path.to_vec()
            } else {
                path[..path.len() - 1].to_vec()
            };
            dirs.push(key.clone());
            let mut entry = Map::new();
            entry.insert("name", Value::String(self.casings.class_name(key)));
            entry.insert("module", Value::String(dirs.join(".")));
            self.models.insert(schema.to_owned(), Value::Object(entry));
        }
        for (name, sub) in obj(resource, "subresources") {
            let mut p = path.to_vec();
            p.push(name.clone());
            self.collect_models(spec, &p, sub);
        }
    }

    fn walk(&mut self, spec: &mut Value, path: &[String], order: &[usize], resource: &Value) {
        let resource = obj_of(Some(resource));
        if is_standalone(resource) && !path.iter().any(|p| p.starts_with('$')) {
            let strings = Value::Array(path.iter().map(|p| Value::from(p.as_str())).collect());
            #[expect(clippy::cast_precision_loss, reason = "orders are small indices")]
            let numbers = Value::Array(order.iter().map(|n| Value::Number(*n as f64)).collect());
            let mut entry = Map::new();
            entry.insert("path", strings);
            entry.insert("order", numbers);
            self.standalone.push(Value::Object(entry));
        }
        for (i, (name, method)) in obj(resource, "methods").iter().enumerate() {
            self.annotate_method(spec, path, name, method, order, i);
        }
        for (i, (name, sub)) in obj(resource, "subresources").iter().enumerate() {
            let mut p = path.to_vec();
            p.push(name.clone());
            let mut o = order.to_vec();
            o.push(i);
            self.walk(spec, &p, &o, sub);
        }
    }
}

enum ModelUri<'a> {
    /// `Name` or `#/components/schemas/Name`.
    Schema(&'a str),
    /// The object form with `only:` or `skip:` leaving Python out.
    NotForPython,
    /// `type: alias`, an object without `openapi_uri`, or a URI into a sub-schema or a query.
    Unsupported,
}

/// A `models:` value: the schema-name shorthand, or `{ type?: model, openapi_uri, only?, skip? }`.
fn model_uri(model: &Value) -> ModelUri<'_> {
    let m = match model {
        Value::String(s) => return ModelUri::Schema(s),
        Value::Object(m) => m,
        _ => return ModelUri::Unsupported,
    };
    let lists_python = |v: &Value| match v {
        Value::Array(langs) => langs.iter().any(|l| l.as_str() == Some("python")),
        _ => false,
    };
    let only_others = m.get("only").is_some_and(|only| !lists_python(only));
    let skipped = m
        .get("skip")
        .is_some_and(|skip| *skip == Value::Bool(true) || lists_python(skip));
    if only_others || skipped {
        return ModelUri::NotForPython;
    }
    if m.get("type").is_some_and(|t| t.as_str() != Some("model")) {
        return ModelUri::Unsupported;
    }
    // A component name (FER1005 if it is not one); a path below it or a JSONPath query is not.
    match m.get("openapi_uri").and_then(Value::as_str) {
        Some(uri) => {
            let name = uri.strip_prefix("#/components/schemas/").unwrap_or(uri);
            if name.is_empty() || name.contains('/') || name.starts_with(['$', '#']) {
                ModelUri::Unsupported
            } else {
                ModelUri::Schema(uri)
            }
        }
        None => ModelUri::Unsupported,
    }
}

/// `standalone_api: true`: the resource is its own package, with its types in its own directory.
fn is_standalone(resource: &Map) -> bool {
    resource.get("standalone_api") == Some(&Value::Bool(true))
}

/// `x-ferrule-casings` and `x-ferrule-standalone`, each only when the config has some, so specs
/// without them are annotated as before. Written next to `x-ferrule-client` (root and mirrored,
/// Forge gap G2).
fn naming_extensions(ctx: &Ctx<'_>) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    if !ctx.casings.is_empty() {
        out.push(("x-ferrule-casings", ctx.casings.extension()));
    }
    if !ctx.standalone.is_empty() {
        out.push(("x-ferrule-standalone", Value::Array(ctx.standalone.clone())));
    }
    out
}

/// Annotate a copy of `spec` from `config`. Pure.
#[must_use]
pub fn annotate(spec: &Value, config: &Value) -> (Value, Vec<Diagnostic>) {
    let mut out = spec.clone();
    let mut ctx = Ctx::new(config);
    let resources = config.get("resources").map_or(empty(), |r| obj_of(Some(r)));
    for name in resources.keys() {
        if !collides_with_client_group(name) {
            continue;
        }
        ctx.diagnostics.push(Diagnostic::error(
            "FER1006",
            to_json_pointer(&[Seg::from("resources"), Seg::from(name.as_str())]),
            format!("Resource {name} lands in the same Forge group as $client methods ({CLIENT_GROUP})."),
            "Rename the resource; Forge splits names into words, so this one is reserved.",
        ));
    }
    for (name, resource) in resources {
        ctx.collect_models(&out, std::slice::from_ref(name), resource);
    }
    for (i, (name, resource)) in resources.iter().enumerate() {
        ctx.walk(&mut out, std::slice::from_ref(name), &[i], resource);
    }
    // `x-fern-type-name` on every schema `models:` names, so Forge's model names match (gap G3).
    for (schema, model) in &ctx.models {
        if let Some(Value::Object(target)) = get_at_mut(
            &mut out,
            &[
                Seg::from("components"),
                Seg::from("schemas"),
                Seg::from(schema.as_str()),
            ],
        ) && let Some(name) = model.get("name")
        {
            target.insert("x-fern-type-name", name.clone());
        }
    }
    let python = config
        .get("targets")
        .and_then(|t| t.get("python"))
        .and_then(Value::as_object);
    // Root settings are written for a Python target only; without one the spec is left as is.
    let pkg = python.and_then(|p| p.get("package_name")).map(|name| {
        let mut pkg = Map::new();
        pkg.insert("name", name.clone());
        pkg.insert(
            "repo",
            python
                .and_then(|p| p.get("production_repo"))
                .cloned()
                .unwrap_or_else(|| Value::from("")),
        );
        pkg
    });
    let mut client = None;
    if let Some(pkg) = &pkg {
        let (built, ds) = build_client(&out, config);
        ctx.diagnostics.extend(ds);
        // The editions whose defaults apply (the first ones when the config names none).
        let ed = apply_edition(config);
        let mut cfg = Map::new();
        cfg.insert("version", Value::Number(1.0));
        cfg.insert("edition", Value::from(ed.edition));
        cfg.insert("pythonEdition", Value::from(ed.python_edition));
        let package_name = pkg.get("name").cloned().unwrap_or_default();
        cfg.insert("packageName", package_name.clone());
        cfg.insert(
            "projectName",
            python
                .and_then(|p| p.get("project_name"))
                .cloned()
                .unwrap_or(package_name),
        );
        if let Some(root) = out.as_object_mut() {
            root.insert("x-ferrule-config", Value::Object(cfg));
            root.insert("x-ferrule-client", built.clone());
            for (key, value) in naming_extensions(&ctx) {
                root.insert(key, value);
            }
        }
        client = Some(built);
    }
    // Forge drops root and schema extensions (gaps G2, G3), so mirror them onto every method.
    let mut mirrored = Map::new();
    mirrored.insert("x-ferrule-models", Value::Object(ctx.models.clone()));
    if let Some(pkg) = pkg {
        mirrored.insert("x-ferrule-package", Value::Object(pkg));
    }
    if let Some(c) = client {
        mirrored.insert("x-ferrule-client", c);
        for (key, value) in naming_extensions(&ctx) {
            mirrored.insert(key, value);
        }
    }
    let bound: Vec<Vec<Seg>> = ctx.bindings.iter().map(|(p, _)| p.clone()).collect();
    write_bindings(&mut out, ctx.bindings, &mirrored);
    drop_unbound_groups(&mut out, &bound);
    (out, ctx.diagnostics)
}

/// The operation keys of a path item that Forge reads as operations.
const VERBS: [&str; 8] = ["get", "put", "post", "delete", "options", "head", "patch", "trace"];
/// Extensions Forge makes methods from: Fern's grouping (an operation with a group name is
/// generated) and Forge's own aliases (one method per alias).
const FERN_GROUPING: [&str; 3] = ["x-fern-sdk-group-name", "x-fern-sdk-method-name", "x-forge-aliases"];

/// Removes Fern's grouping and Forge aliases from operations no method is bound to, which a
/// spec may carry itself (cloudflare's does), so Forge drops them as Stainless leaves out unconfigured
/// endpoints, instead of grouping them without `x-ferrule-resource`.
fn drop_unbound_groups(out: &mut Value, bound: &[Vec<Seg>]) {
    let Some(Value::Object(paths)) = out.as_object_mut().and_then(|o| o.get_mut("paths")) else {
        return;
    };
    for (route, item) in paths.iter_mut() {
        let Value::Object(item) = item else { continue };
        for verb in VERBS {
            let Some(Value::Object(op)) = item.get_mut(verb) else {
                continue;
            };
            let here = [Seg::from("paths"), Seg::from(route.as_str()), Seg::from(verb)];
            if !bound.iter().any(|p| p.as_slice() == here.as_slice()) {
                for key in FERN_GROUPING {
                    op.remove(key);
                }
            }
        }
    }
}

/// Writes each method's extensions, and `mirrored`, on its operation. An operation bound to
/// several methods gets `x-forge-aliases`, one alias per method in config order: Forge then
/// makes a method of each alias, with only the alias's extensions (gap G19).
fn write_bindings(out: &mut Value, bindings: Vec<(Vec<Seg>, Vec<Map>)>, mirrored: &Map) {
    for (op_path, defs) in bindings {
        let Some(Value::Object(op)) = get_at_mut(out, &op_path) else {
            continue;
        };
        let mut defs: Vec<Map> = defs
            .into_iter()
            .map(|mut def| {
                for (key, value) in mirrored {
                    def.insert(key.clone(), value.clone());
                }
                def
            })
            .collect();
        if defs.len() == 1 {
            // Aliases the spec carried would make Forge methods beside this one.
            op.remove("x-forge-aliases");
            for (key, value) in defs.remove(0) {
                op.insert(key, value);
            }
        } else {
            op.insert(
                "x-forge-aliases",
                Value::Array(defs.into_iter().map(Value::Object).collect()),
            );
        }
    }
}
