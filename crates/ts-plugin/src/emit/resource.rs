//! A resource's module: the class with its methods and subresources, the types it declares
//! (models, inline responses, params), and the `declare namespace` that re-exports them.

use ferrule_sdk_model::model::{HttpMethod, Method, Type};

use super::HEADER;
use super::decls::print_decl;
use crate::comment::jsdoc;
use crate::ctx::{Ctx, TsResource, relative};
use crate::doc::{Doc, concat, text};
use crate::examples::{Examples, Fill};
use crate::naming::{camel_case, type_name, words_of};
use crate::pretty::{EXAMPLE_WIDTH, Expr, WIDTH, const_decl, expr, render, signature, specifiers};
use crate::types::{Decl, DeclKind, Mapper};

/// How a method takes its non-path parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamsArg {
    None,
    Query,
    Body,
    /// Query and body together: destructured.
    Mixed,
}

/// A method as the resource class writes it.
#[derive(Debug, Clone)]
pub struct MethodPlan<'a> {
    pub m: &'a Method,
    pub name: String,
    /// `EmbeddingCreateParams`, when the method has query or body parameters.
    pub params_type: Option<String>,
    /// Whether any query or body parameter is required.
    pub params_required: bool,
    pub params_arg: ParamsArg,
    /// What `APIPromise` resolves to: a type name or `void`.
    pub response: String,
    /// The response type's name when it is declared here (`XCreateResponse`), else `None`.
    pub response_decl: Option<Decl>,
    pub params_decl: Option<Decl>,
}

/// The type names a resource module exports, in the order its `declare namespace` lists them.
#[derive(Debug, Clone, Default)]
pub struct Exports {
    pub types: Vec<String>,
}

/// `Embedding` + `Create` + `Params`.
fn method_type(prefix: &str, m: &Method, suffix: &str, ctx: &Ctx<'_>) -> String {
    format!("{prefix}{}{suffix}", type_name(&words_of(&m.name), &ctx.client.casings))
}

/// Standard methods come first in this order; the rest follow by name (as the oracles order).
const STANDARD: [&str; 5] = ["create", "retrieve", "update", "list", "delete"];

/// A resource's methods in the order Stainless writes them.
#[must_use]
pub fn sorted_methods<'a>(r: &TsResource<'a>) -> Vec<&'a Method> {
    let rank = |m: &Method| {
        STANDARD
            .iter()
            .position(|&s| s == m.name.written)
            .unwrap_or(STANDARD.len())
    };
    let mut ms: Vec<&Method> = r.res.methods.iter().collect();
    ms.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| ferrule_sdk_model::js::cmp(&a.name.written, &b.name.written))
    });
    ms
}

/// Plans every method of `r`, declaring their types through `mapper`.
#[must_use]
pub fn plan_methods<'a>(ctx: &Ctx<'a>, r: &TsResource<'a>, mapper: &mut Mapper<'_, 'a>) -> Vec<MethodPlan<'a>> {
    let prefix = r.singular(&ctx.client.casings);
    sorted_methods(r)
        .into_iter()
        .map(|m| {
            let name = camel_case(&words_of(&m.name), &ctx.client.casings);
            let response_name = method_type(&prefix, m, "Response", ctx);
            let (response, response_decl) = match m.response.as_ref().map(|r| &r.ty) {
                None => ("void".to_owned(), None),
                Some(Type::Ref(t)) => (mapper.model_name(t), None),
                Some(t) => {
                    let description = ctx
                        .extras
                        .response_schema(&m.pointer)
                        .and_then(|s| s.get("description"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let d = mapper.model(&response_name, t, description, false, None);
                    (response_name, Some(d))
                }
            };
            let body_fields = m.body.as_ref().and_then(|b| ctx.client.object_of(&b.ty));
            let has_query = !m.query_params.is_empty();
            let has_body = body_fields.is_some_and(|o| !o.fields.is_empty());
            let has_path = !path_in_params(m).is_empty();
            let params_arg = match (has_query, has_body) {
                _ if has_path => ParamsArg::Mixed,
                (false, false) => ParamsArg::None,
                (true, false) => ParamsArg::Query,
                (false, true) => ParamsArg::Body,
                (true, true) => ParamsArg::Mixed,
            };
            let params_required = has_path
                || m.query_params.iter().any(|p| p.field.required)
                || body_fields.is_some_and(|o| o.fields.iter().any(|f| f.required));
            let (params_type, params_decl) = if params_arg == ParamsArg::None {
                (None, None)
            } else {
                let params_name = method_type(&prefix, m, "Params", ctx);
                let mut nested = Vec::new();
                let mut fields = Vec::new();
                for p in path_in_params(m) {
                    fields.push(mapper.field(&p.field, &params_name, &mut nested));
                }
                for p in query_params(ctx, m) {
                    fields.push(mapper.field(&p.field, &params_name, &mut nested));
                }
                if let Some(o) = body_fields {
                    let schema = ctx.extras.body_schema(&m.pointer);
                    for f in mapper.ordered(&o.fields) {
                        fields.push(mapper.described_field(f, &params_name, &mut nested, schema));
                    }
                }
                let decl = Decl {
                    name: params_name.clone(),
                    description: String::new(),
                    deprecated: false,
                    kind: DeclKind::Interface {
                        extends: Vec::new(),
                        fields,
                    },
                    nested,
                };
                (Some(params_name), Some(decl))
            };
            MethodPlan {
                m,
                name,
                params_type,
                params_required,
                params_arg,
                response,
                response_decl,
                params_decl,
            }
        })
        .collect()
}

/// The path parameters a method takes positionally: all of them, or only the last when there
/// are several (the others go in its params object).
#[must_use]
pub fn positional(m: &Method) -> &[ferrule_sdk_model::model::Param] {
    if m.path_params.len() > 1 {
        &m.path_params[m.path_params.len() - 1..]
    } else {
        &m.path_params
    }
}

/// The path parameters a method takes in its params object.
#[must_use]
pub fn path_in_params(m: &Method) -> &[ferrule_sdk_model::model::Param] {
    if m.path_params.len() > 1 {
        &m.path_params[..m.path_params.len() - 1]
    } else {
        &[]
    }
}

/// A method's query parameters, sorted by name unless the config keeps the spec's order.
#[must_use]
pub fn query_params<'m>(ctx: &Ctx<'_>, m: &'m Method) -> Vec<&'m ferrule_sdk_model::model::Param> {
    let fields: Vec<ferrule_sdk_model::model::Field> = m.query_params.iter().map(|p| p.field.clone()).collect();
    let mapper = Mapper::new(ctx, "");
    mapper
        .ordered(&fields)
        .into_iter()
        .filter_map(|f| m.query_params.iter().find(|p| p.field.wire_name == f.wire_name))
        .collect()
}

/// The variable an example assigns the response to: `embeddingResponse` for `EmbeddingResponse`.
fn result_var(response: &str) -> String {
    let base = response.rsplit('.').next().unwrap_or(response);
    camel_case(&crate::naming::split_words(base), &[])
}

/// The variable an example assigns a method's result to: the response model's name, or the
/// resource's singular for a response declared with the method (`const job = ...`).
#[must_use]
pub fn example_var(ctx: &Ctx<'_>, r: &TsResource<'_>, plan: &MethodPlan<'_>) -> String {
    const STANDARD: [&str; 6] = ["create", "retrieve", "get", "update", "list", "delete"];
    match &plan.response_decl {
        // `Array<Model>`: the model's plural.
        Some(Decl {
            kind: DeclKind::Alias(crate::pretty::Ty::Generic(g, args)),
            ..
        }) if g == "Array" && matches!(args.as_slice(), [crate::pretty::Ty::Name(_)]) => {
            let crate::pretty::Ty::Name(item) = &args[0] else {
                unreachable!()
            };
            format!("{}s", result_var(item))
        }
        Some(_) if STANDARD.contains(&plan.m.name.written.as_str()) => {
            camel_case(&crate::naming::singular(&words_of(&r.res.name)), &ctx.client.casings)
        }
        Some(_) => "response".to_owned(),
        None => result_var(&plan.response),
    }
}

/// The `@example` code of a method.
fn example_code(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Vec<String> {
    let ex = Examples { ctx };
    let args = ex.args(plan.m, Fill::Required);
    let accessor = format!("{}.{}", chain[chain.len() - 1].accessor(chain), plan.name);
    let call = Expr::Await(Box::new(Expr::Call(accessor, args.exprs())));
    let doc: Doc = if plan.response == "void" {
        concat(vec![expr(&call), text(";")])
    } else {
        const_decl(&example_var(ctx, chain[chain.len() - 1], plan), &call)
    };
    render(&doc, 0, EXAMPLE_WIDTH).split('\n').map(str::to_owned).collect()
}

fn http_verb(m: HttpMethod) -> &'static str {
    m.as_str()
}

/// `'/embeddings'` or ``path`/v1/sources/${key}` ``.
fn path_expr(m: &Method, ctx: &Ctx<'_>) -> String {
    if m.path_params.is_empty() {
        return crate::js::string_literal(&m.path);
    }
    let mut out = m.path.clone();
    for p in &m.path_params {
        let name = if path_in_params(m).iter().any(|q| q.field.wire_name == p.field.wire_name) {
            p.field.wire_name.clone()
        } else {
            camel_case(&words_of(&p.field.name), &ctx.client.casings)
        };
        out = out.replace(&format!("{{{}}}", p.field.wire_name), &format!("${{{name}}}"));
    }
    format!("path`{out}`")
}

/// The lines of one method, at two spaces.
fn method_lines(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Vec<String> {
    let m = plan.m;
    let mut example = vec!["@example".to_owned(), "```ts".to_owned()];
    example.extend(example_code(ctx, chain, plan));
    example.push("```".to_owned());
    let mut sections = Vec::new();
    if m.deprecated {
        sections.push(vec!["@deprecated".to_owned()]);
    }
    sections.push(example);
    let description = if m.description.is_empty() {
        &m.summary
    } else {
        &m.description
    };
    let mut out = jsdoc(description, &sections, 2);
    let mut params: Vec<Doc> = positional(m)
        .iter()
        .map(|p| {
            text(format!(
                "{}: string",
                camel_case(&words_of(&p.field.name), &ctx.client.casings)
            ))
        })
        .collect();
    let arg = match plan.params_arg {
        ParamsArg::None => None,
        ParamsArg::Query => Some("query"),
        ParamsArg::Body => Some("body"),
        ParamsArg::Mixed => Some("params"),
    };
    if let (Some(arg), Some(t)) = (arg, &plan.params_type) {
        params.push(text(if plan.params_required {
            format!("{arg}: {t}")
        } else {
            format!("{arg}: {t} | null | undefined = {{}}")
        }));
    }
    params.push(text("options?: RequestOptions"));
    let ret = text(format!(": APIPromise<{}>", plan.response));
    out.push(render(&signature(&plan.name, &params, &ret, " {"), 2, WIDTH));
    let path = path_expr(m, ctx);
    let verb = http_verb(m.http_method);
    let call = match plan.params_arg {
        ParamsArg::None => format!("return this._client.{verb}({path}, options);"),
        ParamsArg::Query => format!("return this._client.{verb}({path}, {{ query, ...options }});"),
        ParamsArg::Body => format!("return this._client.{verb}({path}, {{ body, ...options }});"),
        ParamsArg::Mixed => {
            // `const { fileId, environment, ...body } = params;`
            let mut names: Vec<String> = path_in_params(m).iter().map(|p| p.field.wire_name.clone()).collect();
            let has_body = m
                .body
                .as_ref()
                .and_then(|b| ctx.client.object_of(&b.ty))
                .is_some_and(|o| !o.fields.is_empty());
            let query: Vec<String> = query_params(ctx, m).iter().map(|p| p.field.wire_name.clone()).collect();
            let mut parts = Vec::new();
            if has_body {
                names.extend(query.iter().cloned());
                names.push("...body".to_owned());
                if !query.is_empty() {
                    parts.push(format!("query: {{ {} }}", query.join(", ")));
                }
                parts.push("body".to_owned());
            } else if !query.is_empty() {
                names.push("...query".to_owned());
                parts.push("query".to_owned());
            }
            out.push(format!("    const {{ {} }} = params;", names.join(", ")));
            if parts.is_empty() {
                format!("return this._client.{verb}({path}, options);")
            } else {
                parts.push("...options".to_owned());
                format!("return this._client.{verb}({path}, {{ {} }});", parts.join(", "))
            }
        }
    };
    out.push(format!("    {call}"));
    out.push("  }".to_owned());
    out
}

/// Every type a resource module exports, in `declare namespace` order.
#[must_use]
pub fn exports_of(ctx: &Ctx<'_>, r: &TsResource<'_>) -> Vec<String> {
    let mut mapper = Mapper::new(ctx, &r.module());
    let plans = plan_methods(ctx, r, &mut mapper);
    let mut out: Vec<String> = ctx.models_of(&r.keys).iter().map(|d| ctx.type_of(&d.r#ref)).collect();
    out.extend(
        plans
            .iter()
            .filter_map(|p| p.response_decl.as_ref().map(|d| d.name.clone())),
    );
    out.extend(plans.iter().filter_map(|p| p.params_type.clone()));
    out
}

/// `export { A as A, type B as B };` lines of a namespace block, at four spaces.
fn namespace_block(class: Option<&str>, types: &[String]) -> Vec<String> {
    let mut names: Vec<String> = class.map(|c| format!("{c} as {c}")).into_iter().collect();
    names.extend(types.iter().map(|t| format!("type {t} as {t}")));
    render(&specifiers("export ", &names, ";"), 2, WIDTH)
        .split('\n')
        .map(str::to_owned)
        .collect()
}

/// The class, the declarations and the namespace of a resource module.
fn body_lines(
    ctx: &Ctx<'_>,
    r: &TsResource<'_>,
    chain: &[&TsResource<'_>],
    plans: &[MethodPlan<'_>],
    decls: &[Decl],
) -> Vec<String> {
    let mut body: Vec<String> = Vec::new();
    // The class.
    body.extend(jsdoc(r.description.as_deref().unwrap_or(""), &[], 0));
    let mut members: Vec<Vec<String>> = Vec::new();
    let subs: Vec<String> = r
        .children
        .iter()
        .map(|c| {
            format!(
                "  {}: {}API.{} = new {}API.{}(this._client);",
                c.prop, c.class, c.class, c.class, c.class
            )
        })
        .collect();
    if !subs.is_empty() {
        members.push(subs);
    }
    for p in plans {
        members.push(method_lines(ctx, chain, p));
    }
    if members.is_empty() {
        body.push(format!("export class {} extends APIResource {{}}", r.class));
    } else {
        body.push(format!("export class {} extends APIResource {{", r.class));
        for (i, m) in members.into_iter().enumerate() {
            if i > 0 {
                body.push(String::new());
            }
            body.extend(m);
        }
        body.push("}".to_owned());
    }
    for d in decls {
        body.push(String::new());
        body.extend(print_decl(d, 0));
    }
    // Subresources attached to the class, and the namespace.
    if !r.children.is_empty() {
        body.push(String::new());
        for c in &r.children {
            body.push(format!("{}.{} = {};", r.class, c.class, c.class));
        }
    }
    let own: Vec<String> = decls.iter().map(|d| d.name.clone()).collect();
    let mut blocks: Vec<Vec<String>> = Vec::new();
    if !own.is_empty() {
        blocks.push(namespace_block(None, &own));
    }
    for c in &r.children {
        blocks.push(namespace_block(Some(&c.class), &exports_of(ctx, c)));
    }
    if !blocks.is_empty() {
        body.push(String::new());
        body.push(format!("export declare namespace {} {{", r.class));
        for (i, b) in blocks.into_iter().enumerate() {
            if i > 0 {
                body.push(String::new());
            }
            body.extend(b);
        }
        body.push("}".to_owned());
    }

    body
}

/// The imports of a resource module, given the namespaces its types use.
fn import_lines(
    ctx: &Ctx<'_>,
    r: &TsResource<'_>,
    dir: &str,
    namespaces: &[(String, String)],
    plans: &[MethodPlan<'_>],
) -> Vec<String> {
    // Imports, now that the body knows what it uses.
    let mut imports: Vec<String> = vec![format!(
        "import {{ APIResource }} from '{}';",
        relative(dir, "core/resource")
    )];
    let mut namespaces = namespaces.to_vec();
    namespaces.sort_by(|a, b| a.0.cmp(&b.0));
    for (ns, target) in &namespaces {
        imports.push(format!("import * as {ns} from '{}';", relative(dir, target)));
    }
    for c in &r.children {
        let target = relative(dir, &c.module());
        imports.push(format!("import * as {}API from '{target}';", c.class));
        let mut names = vec![c.class.clone()];
        names.extend(exports_of(ctx, c));
        names.sort();
        imports.push(render(
            &specifiers("import ", &names, &format!(" from '{target}';")),
            0,
            WIDTH,
        ));
    }
    if !plans.is_empty() {
        imports.push(format!(
            "import {{ APIPromise }} from '{}';",
            relative(dir, "core/api-promise")
        ));
        imports.push(format!(
            "import {{ RequestOptions }} from '{}';",
            relative(dir, "internal/request-options")
        ));
        if plans.iter().any(|p| !p.m.path_params.is_empty()) {
            imports.push(format!(
                "import {{ path }} from '{}';",
                relative(dir, "internal/utils/path")
            ));
        }
    }
    imports
}

/// The text of a resource's module.
#[must_use]
pub fn emit_resource(ctx: &Ctx<'_>, r: &TsResource<'_>) -> String {
    let module = r.module();
    let dir = r.module_dir();
    let chain = ctx.chain(r);
    let mut mapper = Mapper::new(ctx, &module);
    let plans = plan_methods(ctx, r, &mut mapper);
    let mut decls: Vec<Decl> = ctx
        .models_of(&r.keys)
        .iter()
        .map(|d| {
            mapper.model(
                &ctx.type_of(&d.r#ref),
                &d.ty,
                &d.description,
                d.deprecated,
                Some(&d.r#ref.schema),
            )
        })
        .collect();
    decls.extend(plans.iter().filter_map(|p| p.response_decl.clone()));
    decls.extend(plans.iter().filter_map(|p| p.params_decl.clone()));

    let body = body_lines(ctx, r, &chain, &plans, &decls);
    let imports = import_lines(ctx, r, &dir, &mapper.imports, &plans);
    let mut out = vec![HEADER.to_owned(), String::new()];
    out.extend(imports);
    out.push(String::new());
    out.extend(body);
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// `export * from './x/index';`: the file beside a package's directory.
#[must_use]
pub fn emit_shim(r: &TsResource<'_>) -> String {
    let leaf = r.files.last().map_or("", String::as_str);
    format!("{HEADER}\n\nexport * from './{leaf}/index';\n")
}

/// A package's `index.ts`: its class, then each subresource with its types.
#[must_use]
pub fn emit_index(ctx: &Ctx<'_>, r: &TsResource<'_>) -> String {
    let dir = r.dir();
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    let mut own = vec![r.class.clone()];
    // A standalone API's index names its class only.
    if !r.res.standalone {
        own.extend(exports_of(ctx, r).into_iter().map(|t| format!("type {t}")));
    }
    entries.push((relative(&dir, &r.module()), own));
    for c in &r.children {
        let mut names = vec![c.class.clone()];
        let target = if c.res.standalone {
            relative(&dir, &c.dir())
        } else if c.package {
            format!("{}/index", relative(&dir, &c.dir()))
        } else {
            names.extend(exports_of(ctx, c).into_iter().map(|t| format!("type {t}")));
            relative(&dir, &c.module())
        };
        entries.push((target, names));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = vec![HEADER.to_owned(), String::new()];
    for (target, names) in entries {
        out.push(render(
            &specifiers("export ", &names, &format!(" from '{target}';")),
            0,
            WIDTH,
        ));
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// `src/resources/index.ts`: every top-level resource with its types.
#[must_use]
pub fn emit_resources_index(ctx: &Ctx<'_>) -> String {
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    for r in &ctx.resources {
        let mut names = vec![r.class.clone()];
        if !r.res.standalone {
            names.extend(exports_of(ctx, r).into_iter().map(|t| format!("type {t}")));
        }
        entries.push((relative("resources", &r.module()), names));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = vec![HEADER.to_owned(), String::new()];
    if !ctx.shared_models().is_empty() {
        out.push("export * from './shared';".to_owned());
    }
    for (target, names) in entries {
        out.push(render(
            &specifiers("export ", &names, &format!(" from '{target}';")),
            0,
            WIDTH,
        ));
    }
    let mut text = out.join("\n");
    text.push('\n');
    text
}
