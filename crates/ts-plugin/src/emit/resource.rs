//! A resource's module: the class with its methods and subresources, the types it declares
//! (models, inline responses, params), and the `declare namespace` that re-exports them.
//!
//! One pass for both layouts: the `node` layout imports `APIResource` from `src/resource.ts` and
//! `Core` from `src/core.ts`, takes every path parameter positionally, overloads a method whose
//! params are all optional (`isRequestOptions`), and writes paths as plain template literals.
//!
//! A list method returns a page (`crate::pages`): `getAPIList` with the page class, params that
//! extend `<Class>Params`, a `for await` example, and the module declares the page types of its
//! items, imports the others its methods return and exports them again.

use ferrule_sdk_model::model::{HttpMethod, Method, Type};

use super::HEADER;
use super::decls::print_decl;
use crate::comment::jsdoc;
use crate::ctx::{Ctx, TsResource, relative};
use crate::doc::{Doc, concat, text};
use crate::examples::{Examples, Fill};
use crate::naming::{camel_case, plural, type_name, words_of};
use crate::pretty::{EXAMPLE_WIDTH, Expr, Object, WIDTH, const_decl, expr, for_await, render, signature, specifiers};
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
    /// The page a list method returns.
    pub page: Option<PagePlan>,
}

/// The page a list method returns, as its module writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagePlan {
    /// The page class: `EntriesCursor`.
    pub class: String,
    /// The page type: `BranchesEntriesCursor`.
    pub alias: String,
    /// The item type from this module: `Branch`, `MessagesAPI.Message`.
    pub item: String,
    /// The module that declares the page type.
    pub module: String,
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
            let name = ctx.member(&words_of(&m.name));
            let response_name = method_type(&prefix, m, "Response", ctx);
            let alias = crate::pages::alias_of(ctx, r, m);
            let page_scheme = crate::pages::page_of(ctx, m);
            // A list returns its page type; inline items are declared as `<Prefix><Method>Response`.
            let (response, response_decl, page) = if let Some(a) = alias {
                let (item, decl) = match &a.item {
                    crate::pages::Item::Model(t) => (mapper.model_name(t), None),
                    crate::pages::Item::Inline(n) => {
                        let ty = m.pagination.as_ref().map(|p| &p.item);
                        let description = items_description(ctx, m).unwrap_or_default();
                        let d = ty.map(|t| mapper.model(n, t, &description, false, None));
                        (n.clone(), d)
                    }
                };
                let plan = PagePlan {
                    class: a.class.clone(),
                    alias: a.name.clone(),
                    item,
                    module: a.module.clone(),
                };
                (a.name, decl, Some(plan))
            } else {
                let (response, decl) = response_of(ctx, m, &response_name, mapper);
                (response, decl, None)
            };
            let body_fields = m.body.as_ref().and_then(|b| ctx.client.object_of(&b.ty));
            let has_query = !m.query_params.is_empty();
            let has_body = body_fields.is_some_and(|o| !o.fields.is_empty());
            let has_path = !path_in_params(ctx, m).is_empty();
            let params_arg = match (has_query, has_body) {
                _ if has_path => ParamsArg::Mixed,
                // The `node` layout destructures the query of a method that sends no body, other
                // than a GET: `const { a, b } = params; ... { query: { a, b }, ...options }`.
                // So does the current layout (knock-mgmt-node's `branches.create`).
                (true, false) if m.http_method != HttpMethod::Get => ParamsArg::Mixed,
                (false, false) => ParamsArg::None,
                (true, false) => ParamsArg::Query,
                (false, true) => ParamsArg::Body,
                (true, true) => ParamsArg::Mixed,
            };
            let params_required = has_path
                || m.query_params.iter().any(|p| p.field.required)
                || body_fields.is_some_and(|o| o.fields.iter().any(|f| f.required))
                // The `node` layout takes a required request body even when all its fields are
                // optional (lumaai-typescript's `generations.audio`).
                || (ctx.node() && has_body && m.body.as_ref().is_some_and(|b| b.required));
            // A list's params extend `<Class>Params`, which declares the page fields.
            let inherited = |wire: &str, in_body: bool| {
                page_scheme.is_some_and(|p| p.scheme.in_body == in_body && p.is_request_field(wire))
            };
            let (params_type, params_decl) = if params_arg == ParamsArg::None {
                (None, None)
            } else {
                let params_name = method_type(&prefix, m, "Params", ctx);
                let mut nested = Vec::new();
                let mut fields = Vec::new();
                for p in path_in_params(ctx, m) {
                    fields.push(mapper.field(&p.field, &params_name, &mut nested));
                }
                for p in query_params(ctx, m) {
                    if !inherited(&p.field.wire_name, false) {
                        fields.push(mapper.field(&p.field, &params_name, &mut nested));
                    }
                }
                if let Some(o) = body_fields {
                    let schema = ctx.extras.body_schema(&m.pointer);
                    for f in mapper.ordered(&o.fields) {
                        if !inherited(&f.wire_name, true) {
                            fields.push(mapper.described_field(f, &params_name, &mut nested, schema));
                        }
                    }
                }
                let extends = page
                    .as_ref()
                    .map(|p| format!("{}Params", p.class))
                    .into_iter()
                    .collect();
                let decl = Decl {
                    name: params_name.clone(),
                    description: String::new(),
                    deprecated: false,
                    kind: DeclKind::Interface { extends, fields },
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
                page,
            }
        })
        .collect()
}

/// The description of a list's item schema in the spec (`A Slack channel.`).
fn items_description(ctx: &Ctx<'_>, m: &Method) -> Option<String> {
    let extras = ctx.extras;
    let items = m.pagination.as_ref()?.response.iter().find(|p| p.purpose == "items")?;
    let response = extras.response_schema(&m.pointer)?;
    let array = extras.resolve(extras.property(response, &items.field)?);
    let item = extras.resolve(extras.items(array)?);
    item.get("description")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// What a method's `APIPromise` resolves to, and its declaration when it is declared with the
/// method (`XCreateResponse`).
fn response_of(ctx: &Ctx<'_>, m: &Method, response_name: &str, mapper: &mut Mapper<'_, '_>) -> (String, Option<Decl>) {
    match m.response.as_ref().map(|r| &r.ty) {
        None => ("void".to_owned(), None),
        Some(Type::Ref(t)) => (mapper.model_name(t), None),
        Some(t) => {
            let description = ctx
                .extras
                .response_schema(&m.pointer)
                .and_then(|s| s.get("description"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let d = mapper.model(response_name, t, description, false, None);
            // The `node` layout declares a response without a schema
            // (`type KeyCreateResponse = unknown`) but returns `unknown`.
            let unknown = ctx.node()
                && matches!(
                    t,
                    Type::Scalar {
                        scalar: ferrule_sdk_model::model::Scalar::Unknown
                    }
                );
            (
                if unknown {
                    "unknown".to_owned()
                } else {
                    response_name.to_owned()
                },
                Some(d),
            )
        }
    }
}

/// The path parameters a method takes positionally: all of them in the `node` layout; in the
/// current layout only the last when there are several (the others go in its params object).
#[must_use]
pub fn positional<'m>(ctx: &Ctx<'_>, m: &'m Method) -> &'m [ferrule_sdk_model::model::Param] {
    if m.path_params.len() > 1 && !ctx.node() {
        &m.path_params[m.path_params.len() - 1..]
    } else {
        &m.path_params
    }
}

/// The path parameters a method takes in its params object.
#[must_use]
pub fn path_in_params<'m>(ctx: &Ctx<'_>, m: &'m Method) -> &'m [ferrule_sdk_model::model::Param] {
    if m.path_params.len() > 1 && !ctx.node() {
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
        // The `node` layout names a list's result in the plural: `routesForAgencies`.
        Some(_) if ctx.node() && plan.m.name.written == "list" => {
            ctx.member(&plural(&crate::naming::singular(&words_of(&r.res.name))))
        }
        Some(_) if STANDARD.contains(&plan.m.name.written.as_str()) => {
            ctx.member(&crate::naming::singular(&words_of(&r.res.name)))
        }
        Some(_) => "response".to_owned(),
        None => result_var(&plan.response),
    }
}

/// The `@example` code of a method. The `node` layout writes one only when the call uses an
/// example from the spec (onebusaway-typescript: a path parameter's `example`).
fn example_code(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Option<Vec<String>> {
    let ex = Examples { ctx };
    let fill = if ctx.node() { Fill::Example } else { Fill::Required };
    let args = ex.args(plan.m, fill);
    if ctx.node() && !args.from_spec {
        return None;
    }
    let accessor = format!("{}.{}", chain[chain.len() - 1].accessor(chain), plan.name);
    // A list iterates its items: `for await (const branch of client.branches.list()) {`.
    if let Some(page) = &plan.page {
        let call = Expr::Call(accessor, args.exprs());
        let mut out = vec!["// Automatically fetches more pages as needed.".to_owned()];
        let doc = for_await(&result_var(&page.item), &call, &["// ..."]);
        out.extend(render(&doc, 0, EXAMPLE_WIDTH).split('\n').map(str::to_owned));
        return Some(out);
    }
    let call = Expr::Await(Box::new(Expr::Call(accessor, args.exprs())));
    let doc: Doc = if plan.response == "void" {
        concat(vec![expr(&call), text(";")])
    } else {
        const_decl(&example_var(ctx, chain[chain.len() - 1], plan), &call)
    };
    Some(render(&doc, 0, EXAMPLE_WIDTH).split('\n').map(str::to_owned).collect())
}

fn http_verb(m: HttpMethod) -> &'static str {
    m.as_str()
}

/// `'/embeddings'` or ``path`/v1/sources/${key}` ``; a plain template literal in the `node`
/// layout.
fn path_expr(m: &Method, ctx: &Ctx<'_>) -> String {
    if m.path_params.is_empty() {
        return crate::js::string_literal(&m.path);
    }
    let mut out = m.path.clone();
    for p in &m.path_params {
        let name = if path_in_params(ctx, m)
            .iter()
            .any(|q| q.field.wire_name == p.field.wire_name)
        {
            p.field.wire_name.clone()
        } else {
            ctx.member(&words_of(&p.field.name))
        };
        out = out.replace(&format!("{{{}}}", p.field.wire_name), &format!("${{{name}}}"));
    }
    if ctx.node() {
        format!("`{out}`")
    } else {
        format!("path`{out}`")
    }
}

/// A method's doc comment: its description, `@deprecated`, `@example`.
fn method_doc(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Vec<String> {
    let m = plan.m;
    let mut sections = Vec::new();
    if m.deprecated {
        sections.push(vec!["@deprecated".to_owned()]);
    }
    if let Some(code) = example_code(ctx, chain, plan) {
        let mut example = vec!["@example".to_owned(), "```ts".to_owned()];
        example.extend(code);
        example.push("```".to_owned());
        sections.push(example);
    }
    let description = if m.description.is_empty() {
        &m.summary
    } else {
        &m.description
    };
    jsdoc(description, &sections, 2)
}

/// The name a method's params object has in its signature.
fn params_name(arg: ParamsArg) -> Option<&'static str> {
    match arg {
        ParamsArg::None => None,
        ParamsArg::Query => Some("query"),
        ParamsArg::Body => Some("body"),
        ParamsArg::Mixed => Some("params"),
    }
}

/// `{ key, ...options }`: an object literal of shorthand properties and spreads (an empty raw
/// value prints the key alone), on one line until it does not fit.
fn literal(props: Vec<(String, Expr)>) -> Expr {
    Expr::Object(Object { props, expanded: false })
}

/// The lines of one method of a `node`-layout resource, at two spaces.
fn node_method_lines(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Vec<String> {
    let m = plan.m;
    let mut out = method_doc(ctx, chain, plan);
    let positional: Vec<String> = positional(ctx, m)
        .iter()
        .map(|p| ctx.member(&words_of(&p.field.name)))
        .collect();
    let mut head: Vec<Doc> = positional.iter().map(|p| text(format!("{p}: string"))).collect();
    let options = text("options?: Core.RequestOptions");
    let ret = match &plan.page {
        Some(p) => text(format!(": Core.PagePromise<{}, {}>", p.alias, p.item)),
        None => text(format!(": Core.APIPromise<{}>", plan.response)),
    };
    let arg = params_name(plan.params_arg);
    match (arg, &plan.params_type) {
        (Some(arg), Some(t)) if !plan.params_required => {
            let mut first = head.clone();
            first.extend([text(format!("{arg}?: {t}")), options.clone()]);
            out.push(render(&signature(&plan.name, &first, &ret, ";"), 2, WIDTH));
            let mut second = head.clone();
            second.push(options.clone());
            out.push(render(&signature(&plan.name, &second, &ret, ";"), 2, WIDTH));
            head.extend([text(format!("{arg}: {t} | Core.RequestOptions = {{}}")), options]);
            out.push(render(&signature(&plan.name, &head, &ret, " {"), 2, WIDTH));
            out.push(format!("    if (isRequestOptions({arg})) {{"));
            let mut again: Vec<Expr> = positional.iter().map(|p| Expr::raw(p)).collect();
            again.extend([Expr::raw("{}"), Expr::raw(arg)]);
            let call = concat(vec![
                text("return "),
                expr(&Expr::Call(format!("this.{}", plan.name), again)),
                text(";"),
            ]);
            out.push(render(&call, 6, WIDTH));
            out.push("    }".to_owned());
        }
        (Some(arg), Some(t)) => {
            head.extend([text(format!("{arg}: {t}")), options]);
            out.push(render(&signature(&plan.name, &head, &ret, " {"), 2, WIDTH));
        }
        _ => {
            head.push(options);
            out.push(render(&signature(&plan.name, &head, &ret, " {"), 2, WIDTH));
        }
    }
    let shorthand = |k: &str| (k.to_owned(), Expr::raw(""));
    let mut props: Vec<(String, Expr)> = Vec::new();
    match plan.params_arg {
        ParamsArg::None => {}
        ParamsArg::Query => props.push(shorthand("query")),
        ParamsArg::Body => props.push(shorthand("body")),
        ParamsArg::Mixed => {
            let query: Vec<String> = query_params(ctx, m).iter().map(|p| p.field.wire_name.clone()).collect();
            let has_body = m
                .body
                .as_ref()
                .and_then(|b| ctx.client.object_of(&b.ty))
                .is_some_and(|o| !o.fields.is_empty());
            let mut names = query.clone();
            if has_body {
                names.push("...body".to_owned());
            }
            out.push(render(&specifiers("const ", &names, " = params;"), 4, WIDTH));
            props.push((
                "query".to_owned(),
                literal(query.iter().map(|q| shorthand(q)).collect()),
            ));
            if has_body {
                props.push(shorthand("body"));
            }
        }
    }
    let void = plan.response == "void" && plan.page.is_none();
    if let Some(p) = &plan.page {
        // A list is fetched by its page class, with its HTTP method when it is not a GET.
        out.push(page_return(m, Expr::Raw(path_expr(m, ctx)), Expr::raw(&p.alias), props));
        out.push("  }".to_owned());
        return out;
    }
    let last = if props.is_empty() && !void {
        Expr::raw("options")
    } else {
        props.push(shorthand("...options"));
        if void {
            props.push((
                "headers".to_owned(),
                literal(vec![
                    ("Accept".to_owned(), Expr::str("*/*")),
                    shorthand("...options?.headers"),
                ]),
            ));
        }
        literal(props)
    };
    let call = Expr::Call(
        format!("this._client.{}", http_verb(m.http_method)),
        vec![Expr::Raw(path_expr(m, ctx)), last],
    );
    out.push(render(&concat(vec![text("return "), expr(&call), text(";")]), 4, WIDTH));
    out.push("  }".to_owned());
    out
}

/// `return this._client.getAPIList(path, Page, { ...props, method, ...options });` at four
/// spaces: `props` are the params' properties.
fn page_return(m: &Method, path: Expr, page: Expr, mut props: Vec<(String, Expr)>) -> String {
    if m.http_method != HttpMethod::Get {
        props.push(("method".to_owned(), Expr::str(http_verb(m.http_method))));
    }
    let last = if props.is_empty() {
        Expr::raw("options")
    } else {
        props.push(("...options".to_owned(), Expr::raw("")));
        literal(props)
    };
    let call = Expr::Call("this._client.getAPIList".to_owned(), vec![path, page, last]);
    render(&concat(vec![text("return "), expr(&call), text(";")]), 4, WIDTH)
}

/// The lines of one method, at two spaces.
fn method_lines(ctx: &Ctx<'_>, chain: &[&TsResource<'_>], plan: &MethodPlan<'_>) -> Vec<String> {
    if ctx.node() {
        return node_method_lines(ctx, chain, plan);
    }
    let m = plan.m;
    let mut out = method_doc(ctx, chain, plan);
    let mut params: Vec<Doc> = positional(ctx, m)
        .iter()
        .map(|p| text(format!("{}: string", ctx.member(&words_of(&p.field.name)))))
        .collect();
    let arg = params_name(plan.params_arg);
    if let (Some(arg), Some(t)) = (arg, &plan.params_type) {
        params.push(text(if plan.params_required {
            format!("{arg}: {t}")
        } else {
            format!("{arg}: {t} | null | undefined = {{}}")
        }));
    }
    params.push(text("options?: RequestOptions"));
    let ret = match &plan.page {
        Some(p) => text(format!(": PagePromise<{}, {}>", p.alias, p.item)),
        None => text(format!(": APIPromise<{}>", plan.response)),
    };
    out.push(render(&signature(&plan.name, &params, &ret, " {"), 2, WIDTH));
    if let Some(p) = &plan.page {
        out.extend(page_call(ctx, plan, p));
        out.push("  }".to_owned());
        return out;
    }
    let (destructure, mut props) = call_props(ctx, plan);
    out.extend(destructure);
    if !props.is_empty() {
        props.push(("...options".to_owned(), Expr::raw("")));
    }
    let last = if props.is_empty() {
        Expr::raw("options")
    } else {
        literal(props)
    };
    let call = Expr::Call(
        format!("this._client.{}", http_verb(m.http_method)),
        vec![Expr::Raw(path_expr(m, ctx)), last],
    );
    out.push(render(&concat(vec![text("return "), expr(&call), text(";")]), 4, WIDTH));
    out.push("  }".to_owned());
    out
}

/// How a current-layout method passes its params: the `const { ... } = params;` line when it
/// destructures them, and the request options' properties (`query`, `body`).
fn call_props(ctx: &Ctx<'_>, plan: &MethodPlan<'_>) -> (Option<String>, Vec<(String, Expr)>) {
    let m = plan.m;
    let shorthand = |k: &str| (k.to_owned(), Expr::raw(""));
    let mut props: Vec<(String, Expr)> = Vec::new();
    match plan.params_arg {
        ParamsArg::None => (None, props),
        ParamsArg::Query => (None, vec![shorthand("query")]),
        ParamsArg::Body => (None, vec![shorthand("body")]),
        ParamsArg::Mixed => {
            // `const { fileId, environment, ...body } = params;`
            let mut names: Vec<String> = path_in_params(ctx, m)
                .iter()
                .map(|p| p.field.wire_name.clone())
                .collect();
            let has_body = m
                .body
                .as_ref()
                .and_then(|b| ctx.client.object_of(&b.ty))
                .is_some_and(|o| !o.fields.is_empty());
            let query: Vec<String> = query_params(ctx, m).iter().map(|p| p.field.wire_name.clone()).collect();
            let listed = || literal(query.iter().map(|q| shorthand(q)).collect());
            if has_body {
                names.extend(query.iter().cloned());
                names.push("...body".to_owned());
                if !query.is_empty() {
                    props.push(("query".to_owned(), listed()));
                }
                props.push(shorthand("body"));
            } else if !query.is_empty() && m.http_method != HttpMethod::Get {
                // A query sent without a body other than by a GET is listed (knock-mgmt-node's
                // `branches.create`, `commits.commitAll`).
                names.extend(query.iter().cloned());
                props.push(("query".to_owned(), listed()));
            } else if !query.is_empty() {
                names.push("...query".to_owned());
                props.push(shorthand("query"));
            }
            let line = render(&specifiers("const ", &names, " = params;"), 4, WIDTH);
            (Some(line), props)
        }
    }
}

/// The body of a current-layout list method, at four spaces: the params destructured as other
/// methods do, then `getAPIList` with the page class over the items.
fn page_call(ctx: &Ctx<'_>, plan: &MethodPlan<'_>, page: &PagePlan) -> Vec<String> {
    let m = plan.m;
    let (destructure, props) = call_props(ctx, plan);
    let mut out: Vec<String> = destructure.into_iter().collect();
    let class = Expr::Raw(format!("{}<{}>", page.class, page.item));
    out.push(page_return(m, Expr::Raw(path_expr(m, ctx)), class, props));
    out
}

/// The page types a resource module declares, in order.
#[must_use]
pub fn page_types(ctx: &Ctx<'_>, r: &TsResource<'_>) -> Vec<String> {
    crate::pages::declared_in(ctx, &r.module())
        .into_iter()
        .map(|a| a.name)
        .collect()
}

/// Every type a resource module exports, in `declare namespace` order: models, responses, page
/// types, params.
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
    out.extend(page_types(ctx, r));
    out.extend(plans.iter().filter_map(|p| p.params_type.clone()));
    out
}

/// `type X` in an export list, or `X` for a page type of the `node` layout (a class, a value).
#[must_use]
pub fn export_name(ctx: &Ctx<'_>, pages: &[String], name: &str) -> String {
    if ctx.node() && pages.iter().any(|p| p == name) {
        name.to_owned()
    } else {
        format!("type {name}")
    }
}

/// What a package index or `src/resources/index.ts` exports from `r`'s module: the class and its
/// types, the page types last (current layout) or first (`node` layout).
#[must_use]
pub fn index_names(ctx: &Ctx<'_>, r: &TsResource<'_>) -> Vec<String> {
    let pages = page_types(ctx, r);
    let types: Vec<String> = exports_of(ctx, r)
        .into_iter()
        .filter(|t| !pages.contains(t))
        .map(|t| format!("type {t}"))
        .collect();
    if ctx.node() {
        let mut out = pages;
        out.push(r.class.clone());
        out.extend(types);
        out
    } else {
        let mut out = vec![r.class.clone()];
        out.extend(types);
        out.extend(pages.iter().map(|p| format!("type {p}")));
        out
    }
}

/// `export { A as A, type B as B };` lines of a namespace block, at four spaces.
fn namespace_block(ctx: &Ctx<'_>, class: Option<&str>, types: &[String], pages: &[String]) -> Vec<String> {
    let mut names: Vec<String> = class.map(|c| format!("{c} as {c}")).into_iter().collect();
    names.extend(types.iter().map(|t| format!("{} as {t}", export_name(ctx, pages, t))));
    render(&specifiers("export ", &names, ";"), 2, WIDTH)
        .split('\n')
        .map(str::to_owned)
        .collect()
}

/// The page types of a resource module: the ones it declares (with their declaration) and the
/// ones its methods return that another module declares, which it imports and exports again.
#[derive(Debug, Clone, Default)]
struct PageTypes {
    declared: Vec<(String, String)>,
    /// (module, name), sorted by name.
    imported: Vec<(String, String)>,
}

fn page_decls(ctx: &Ctx<'_>, r: &TsResource<'_>, plans: &[MethodPlan<'_>], mapper: &mut Mapper<'_, '_>) -> PageTypes {
    let module = r.module();
    let declared = crate::pages::declared_in(ctx, &module)
        .into_iter()
        .map(|a| {
            let item = match &a.item {
                crate::pages::Item::Model(t) => mapper.model_name(t),
                crate::pages::Item::Inline(n) => n.clone(),
            };
            let line = if ctx.node() {
                format!("export class {} extends {}<{item}> {{}}", a.name, a.class)
            } else {
                format!("export type {} = {}<{item}>;", a.name, a.class)
            };
            (a.name, line)
        })
        .collect();
    let mut imported: Vec<(String, String)> = Vec::new();
    for p in plans.iter().filter_map(|p| p.page.as_ref()) {
        // A subresource's page types come with its import.
        let from_child = r.children.iter().any(|c| c.module() == p.module);
        if p.module != module && !from_child && !imported.iter().any(|(_, n)| *n == p.alias) {
            imported.push((p.module.clone(), p.alias.clone()));
        }
    }
    imported.sort_by(|a, b| a.1.cmp(&b.1));
    PageTypes { declared, imported }
}

/// The class, the declarations and the namespace of a resource module.
fn body_lines(
    ctx: &Ctx<'_>,
    r: &TsResource<'_>,
    chain: &[&TsResource<'_>],
    plans: &[MethodPlan<'_>],
    decls: &[Decl],
    pages: &PageTypes,
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
    for (_, line) in &pages.declared {
        body.push(String::new());
        body.push(line.clone());
    }
    for d in decls {
        body.push(String::new());
        body.extend(print_decl(d, 0));
    }
    // The `node` layout's page classes and the subresources, attached to the class.
    let mut attached: Vec<String> = Vec::new();
    if ctx.node() {
        for (name, _) in &pages.declared {
            attached.push(format!("{}.{name} = {name};", r.class));
        }
    }
    for c in &r.children {
        attached.push(format!("{}.{} = {};", r.class, c.class, c.class));
        if ctx.node() {
            for p in page_types(ctx, c) {
                attached.push(format!("{}.{p} = {p};", r.class));
            }
        }
    }
    if !attached.is_empty() {
        body.push(String::new());
        body.extend(attached);
    }
    // The namespace.
    let own = exports_of(ctx, r);
    let own_pages: Vec<String> = pages.declared.iter().map(|(n, _)| n.clone()).collect();
    let mut blocks: Vec<Vec<String>> = Vec::new();
    if !own.is_empty() {
        blocks.push(namespace_block(ctx, None, &own, &own_pages));
    }
    for c in &r.children {
        blocks.push(namespace_block(
            ctx,
            Some(&c.class),
            &exports_of(ctx, c),
            &page_types(ctx, c),
        ));
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
    // The page types of other modules its methods return, exported again.
    if !pages.imported.is_empty() {
        let names: Vec<String> = pages
            .imported
            .iter()
            .map(|(_, n)| if ctx.node() { n.clone() } else { format!("type {n}") })
            .collect();
        body.push(String::new());
        body.push(render(&specifiers("export ", &names, ";"), 0, WIDTH));
    }

    body
}

/// `import { EntriesCursor, type EntriesCursorParams, PagePromise } from '../core/pagination';`:
/// the page classes the module declares types of (and, in the current layout, fetches with), the
/// params its methods' params extend, and `PagePromise` (current layout).
fn pagination_import(ctx: &Ctx<'_>, dir: &str, plans: &[MethodPlan<'_>], pages: &PageTypes) -> Option<String> {
    let mut names: Vec<(String, String)> = Vec::new();
    let mut add = |sort: String, name: String| {
        if !names.iter().any(|(_, n)| *n == name) {
            names.push((sort, name));
        }
    };
    for a in crate::pages::aliases(ctx) {
        if pages.declared.iter().any(|(n, _)| *n == a.name) {
            add(a.class.clone(), a.class);
        }
    }
    for p in plans.iter().filter_map(|p| p.page.as_ref().map(|page| (p, page))) {
        let (plan, page) = p;
        if !ctx.node() {
            add(page.class.clone(), page.class.clone());
        }
        if plan.params_type.is_some() {
            let params = format!("{}Params", page.class);
            add(params.clone(), format!("type {params}"));
        }
    }
    if !ctx.node() && plans.iter().any(|p| p.page.is_some()) {
        add("PagePromise".to_owned(), "PagePromise".to_owned());
    }
    if names.is_empty() {
        return None;
    }
    names.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<String> = names.into_iter().map(|(_, n)| n).collect();
    let module = if ctx.node() { "pagination" } else { "core/pagination" };
    let tail = format!(" from '{}';", relative(dir, module));
    Some(render(&specifiers("import ", &names, &tail), 0, WIDTH))
}

/// The namespace imports, each followed by the page types imported from the same module, then
/// page types from modules without a namespace import.
fn namespace_imports(dir: &str, namespaces: &[(String, String)], pages: &PageTypes) -> Vec<String> {
    let mut out = Vec::new();
    let mut namespaces = namespaces.to_vec();
    namespaces.sort_by(|a, b| a.0.cmp(&b.0));
    let named = |module: &str| -> Option<String> {
        let names: Vec<String> = pages
            .imported
            .iter()
            .filter(|(m, _)| m == module)
            .map(|(_, n)| n.clone())
            .collect();
        (!names.is_empty()).then(|| {
            render(
                &specifiers("import ", &names, &format!(" from '{}';", relative(dir, module))),
                0,
                WIDTH,
            )
        })
    };
    for (ns, target) in &namespaces {
        out.push(format!("import * as {ns} from '{}';", relative(dir, target)));
        out.extend(named(target));
    }
    let mut rest: Vec<&String> = pages
        .imported
        .iter()
        .map(|(m, _)| m)
        .filter(|m| !namespaces.iter().any(|(_, t)| t == *m))
        .collect();
    rest.dedup();
    for m in rest {
        out.extend(named(m));
    }
    out
}

/// The imports of a resource module, given the namespaces its types use.
fn import_lines(
    ctx: &Ctx<'_>,
    r: &TsResource<'_>,
    dir: &str,
    namespaces: &[(String, String)],
    plans: &[MethodPlan<'_>],
    pages: &PageTypes,
) -> Vec<String> {
    // Imports, now that the body knows what it uses.
    if ctx.node() {
        return node_import_lines(ctx, r, dir, namespaces, plans, pages);
    }
    let mut imports: Vec<String> = vec![format!(
        "import {{ APIResource }} from '{}';",
        relative(dir, "core/resource")
    )];
    imports.extend(namespace_imports(dir, namespaces, pages));
    imports.extend(child_imports(ctx, r, dir));
    if plans.iter().any(|p| p.page.is_none()) {
        imports.push(format!(
            "import {{ APIPromise }} from '{}';",
            relative(dir, "core/api-promise")
        ));
    }
    imports.extend(pagination_import(ctx, dir, plans, pages));
    if !plans.is_empty() {
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

/// `import * as XAPI from './x'; import { X, XParams } from './x';` for each subresource.
fn child_imports(ctx: &Ctx<'_>, r: &TsResource<'_>, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    for c in &r.children {
        let target = relative(dir, &c.module());
        out.push(format!("import * as {}API from '{target}';", c.class));
        let mut names = vec![c.class.clone()];
        names.extend(exports_of(ctx, c));
        names.sort();
        out.push(render(
            &specifiers("import ", &names, &format!(" from '{target}';")),
            0,
            WIDTH,
        ));
    }
    out
}

/// The imports of a `node`-layout resource module: `APIResource`, `isRequestOptions` for
/// overloaded methods, `Core`, then the namespaces and subresources as in the current layout, and
/// the page classes last.
fn node_import_lines(
    ctx: &Ctx<'_>,
    r: &TsResource<'_>,
    dir: &str,
    namespaces: &[(String, String)],
    plans: &[MethodPlan<'_>],
    pages: &PageTypes,
) -> Vec<String> {
    let mut imports: Vec<String> = vec![format!(
        "import {{ APIResource }} from '{}';",
        relative(dir, "resource")
    )];
    if plans.iter().any(|p| p.params_type.is_some() && !p.params_required) {
        imports.push(format!(
            "import {{ isRequestOptions }} from '{}';",
            relative(dir, "core")
        ));
    }
    if !plans.is_empty() {
        imports.push(format!("import * as Core from '{}';", relative(dir, "core")));
    }
    imports.extend(namespace_imports(dir, namespaces, pages));
    imports.extend(child_imports(ctx, r, dir));
    imports.extend(pagination_import(ctx, dir, plans, pages));
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
    let pages = page_decls(ctx, r, &plans, &mut mapper);
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

    let body = body_lines(ctx, r, &chain, &plans, &decls, &pages);
    let imports = import_lines(ctx, r, &dir, &mapper.imports, &plans, &pages);
    let mut out = vec![HEADER.to_owned(), String::new()];
    out.extend(imports);
    out.push(String::new());
    out.extend(body);
    let mut text = out.join("\n");
    text.push('\n');
    text
}

/// `src/resources/shared.ts`: the `$shared` models.
#[must_use]
pub fn emit_shared(ctx: &Ctx<'_>) -> String {
    let mut mapper = Mapper::new(ctx, "resources/shared");
    let decls: Vec<Decl> = ctx
        .shared_models()
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
    let mut out = vec![HEADER.to_owned(), String::new()];
    let mut namespaces = mapper.imports.clone();
    namespaces.sort_by(|a, b| a.0.cmp(&b.0));
    if !namespaces.is_empty() {
        for (ns, target) in &namespaces {
            out.push(format!("import * as {ns} from '{}';", relative("resources", target)));
        }
        out.push(String::new());
    }
    for (i, d) in decls.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.extend(print_decl(d, 0));
    }
    out.join("\n") + "\n"
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
    // A standalone API's index names its class only.
    let own = if r.res.standalone {
        vec![r.class.clone()]
    } else {
        index_names(ctx, r)
    };
    entries.push((relative(&dir, &r.module()), own));
    for c in &r.children {
        let mut names = vec![c.class.clone()];
        let target = if c.res.standalone {
            relative(&dir, &c.dir())
        } else if c.package {
            format!("{}/index", relative(&dir, &c.dir()))
        } else {
            names = index_names(ctx, c);
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
        let names = if r.res.standalone {
            vec![r.class.clone()]
        } else {
            index_names(ctx, r)
        };
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
