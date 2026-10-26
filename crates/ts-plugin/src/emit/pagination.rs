//! The page classes: `src/core/pagination.ts` (the current layout, with `src/pagination.ts`
//! re-exporting it) or `src/pagination.ts` (the `node` layout), one class per stainless.yml
//! `pagination` scheme, in config order.
//!
//! Each scheme declares `<Class>Response<Item>` (its response fields, all required, nested
//! objects in a namespace), `<Class>Params` (its request fields, all optional) and the class,
//! which reads each field with a default and computes the next page's request. The shared head of
//! the current layout (`AbstractPage`, `PagePromise`) is `templates/pagination_head.ts.tmpl`, the
//! text of knocklabs/knock-mgmt-node@ca88fdba's `src/core/pagination.ts`; the `node` layout's
//! `AbstractPage` is in the runtime's `src/core.ts`. Modified by The Ferrule Authors: the client
//! and error names replaced by `%%TOKEN%%`s.
//!
//! Proven against the oracles: `cursor` in both layouts (knock-mgmt-node, knock-node) and
//! `page_number` in the `node` layout (honcho-node-core). The other schemes (`cursor_id`,
//! `cursor_url`, `offset`, `fake_page`, and `page_number` in the current layout) follow the same
//! shapes with the next-page rules of `runtime/src/ferrule_runtime/pagination.py`, and the
//! `hasNextPage` override of a page with a `has_more` field is Ferrule's own text; no
//! Apache-2.0 oracle pins them yet.

use super::decls::print_decl;
use super::template::render as fill;
use super::{File, HEADER};
use crate::ctx::Ctx;
use crate::doc::text;
use crate::js;
use crate::pages::{Page, default_value, field, pages};
use crate::pretty::{Ty, WIDTH, property_signature, render, signature};
use crate::types::{Decl, DeclKind, Mapper};

const HEAD: &str = include_str!("templates/pagination_head.ts.tmpl");

/// `this.a?.b`: a response path read off the page.
fn read_path(path: &[String]) -> String {
    format!("this.{}", path.join("?."))
}

/// Where the request options carry the scheme's request fields: `body` for `param_location:
/// body` in the current layout, else `query` (the `node` layout's `params` join the query).
fn request_place(ctx: &Ctx<'_>, page: Page<'_>) -> &'static str {
    if page.scheme.in_body && !ctx.node() {
        "body"
    } else {
        "query"
    }
}

/// `(this.options.query as Record<string, unknown>)?.['key']`: a request field of the previous
/// page's request, from where the scheme sends it.
fn previous_request_field(ctx: &Ctx<'_>, page: Page<'_>, key: &str) -> String {
    format!(
        "(this.options.{} as Record<string, unknown>)?.[{}]",
        request_place(ctx, page),
        js::string_literal(key)
    )
}

/// `{ ...this.options, query: { ...maybeObj(this.options.query), key: value } }` (current
/// layout) or `{ params: { key: value } }` (`node` layout), at four spaces, after `return `.
fn next_request(ctx: &Ctx<'_>, page: Page<'_>, key: &str, value: &str) -> Vec<String> {
    let key = js::property_key(key);
    if ctx.node() {
        return vec![
            "    return {".to_owned(),
            "      params: {".to_owned(),
            format!("        {key}: {value},"),
            "      },".to_owned(),
            "    };".to_owned(),
        ];
    }
    let place = request_place(ctx, page);
    vec![
        "    return {".to_owned(),
        "      ...this.options,".to_owned(),
        format!("      {place}: {{"),
        format!("        ...maybeObj(this.options.{place}),"),
        format!("        {key}: {value},"),
        "      },".to_owned(),
        "    };".to_owned(),
    ]
}

/// `cursor`: the next cursor from the response, sent as the cursor param.
fn next_cursor(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let path = page.response_path("next_cursor_field").unwrap_or_default();
    let param = page.request_param("next_cursor_param").unwrap_or("cursor");
    let mut out = vec![
        format!("    const cursor = {};", read_path(&path)),
        "    if (!cursor) {".to_owned(),
        "      return null;".to_owned(),
        "    }".to_owned(),
        String::new(),
    ];
    out.extend(next_request(ctx, page, param, "cursor"));
    out
}

/// `cursor_id`: the last item's ID, or the first one's when paging backwards.
fn next_cursor_id(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let after = page.request_param("next_cursor_id_param").unwrap_or("after");
    let id = item_id(page);
    let mut out = vec![
        "    const data = this.getPaginatedItems();".to_owned(),
        format!("    const id = data[data.length - 1]?.{id};"),
        "    if (!id) {".to_owned(),
        "      return null;".to_owned(),
        "    }".to_owned(),
        String::new(),
    ];
    if let Some(before) = page.request_param("previous_cursor_id_param") {
        let mut reverse = vec![
            format!("    if ({}) {{", previous_request_field(ctx, page, before)),
            format!("      const first = data[0]?.{id};"),
            "      if (!first) {".to_owned(),
            "        return null;".to_owned(),
            "      }".to_owned(),
            String::new(),
        ];
        reverse.extend(
            next_request(ctx, page, before, "first")
                .into_iter()
                .map(|l| format!("  {l}")),
        );
        reverse.push("    }".to_owned());
        reverse.push(String::new());
        out.splice(1..1, reverse);
    }
    out.extend(next_request(ctx, page, after, "id"));
    out
}

/// `cursor_url`: the URL of the next page from the response.
fn next_cursor_url(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let path = page
        .response_path("cursor_url_field")
        .or_else(|| page.response_path("next_url_field"))
        .unwrap_or_default();
    let mut out = vec![
        format!("    const url = {};", read_path(&path)),
        "    if (!url) {".to_owned(),
        "      return null;".to_owned(),
        "    }".to_owned(),
        String::new(),
    ];
    if ctx.node() {
        out.push("    return { url: new URL(url) };".to_owned());
    } else {
        out.extend([
            "    const next = new URL(url);".to_owned(),
            "    return {".to_owned(),
            "      ...this.options,".to_owned(),
            // The whole URL: `buildURL` takes an absolute path as is.
            "      path: next.toString(),".to_owned(),
            "      query: Object.fromEntries(next.searchParams),".to_owned(),
            "    };".to_owned(),
        ]);
    }
    out
}

/// `offset`: the offset after this page's items, until the total count.
fn next_offset(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let items = page.items().unwrap_or("data");
    let param = page.request_param("offset_count_param").unwrap_or("offset");
    let mut out = vec![
        format!(
            "    const offset = Number({} ?? 0);",
            previous_request_field(ctx, page, param)
        ),
        format!("    const currentCount = offset + this.{items}.length;"),
    ];
    if let Some(total) = page.response_path("offset_total_count_field") {
        out.push(String::new());
        out.push(format!("    if (currentCount >= {}) {{", read_path(&total)));
        out.push("      return null;".to_owned());
        out.push("    }".to_owned());
    }
    out.push(String::new());
    out.extend(next_request(ctx, page, param, "currentCount"));
    out
}

/// `page_number`: the page after the current one, until the total page count.
fn next_page_number(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let current = page
        .response_path("current_page_number_field")
        .unwrap_or_else(|| vec!["page".to_owned()]);
    let param = page.request_param("page_number_param").unwrap_or("page");
    let mut out = vec![
        format!("    const currentPage = {};", read_path(&current)),
        String::new(),
    ];
    if let Some(total) = page.response_path("total_page_count_field") {
        out.push(format!("    if (currentPage >= {}) {{", read_path(&total)));
        out.push("      return null;".to_owned());
        out.push("    }".to_owned());
        out.push(String::new());
    }
    if ctx.node() {
        out.push(format!(
            "    return {{ params: {{ {}: currentPage + 1 }} }};",
            js::property_key(param)
        ));
    } else {
        out.extend(next_request(ctx, page, param, "currentPage + 1"));
    }
    out
}

/// The body of `nextPageRequestOptions` / `nextPageInfo`, at four spaces.
fn next_page_body(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    match page.kind() {
        "cursor" => next_cursor(ctx, page),
        "cursor_id" => next_cursor_id(ctx, page),
        "cursor_url" => next_cursor_url(ctx, page),
        "offset" => next_offset(ctx, page),
        "page_number" => next_page_number(ctx, page),
        // `fake_page`: the response is one page.
        _ => vec!["    return null;".to_owned()],
    }
}

/// The item field a `cursor_id` page reads the next cursor from: `id`.
fn item_id(page: Page<'_>) -> String {
    page.response_path("cursor_item_id")
        .and_then(|p| p.last().cloned())
        .unwrap_or_else(|| "id".to_owned())
}

/// The `<Class>Response<Item>` interface with its namespace, and the class fields' types.
fn response_decls(ctx: &Ctx<'_>, page: Page<'_>, class: &str) -> (Vec<String>, Vec<(String, Ty)>) {
    let ns = format!("{class}Response");
    let items = page.items();
    let mut mapper = Mapper::new(ctx, "core/pagination");
    let mut nested = Vec::new();
    let mut fields = Vec::new();
    for (k, v) in &page.scheme.response {
        let mut f = mapper.field(&field(k, v, true), &ns, &mut nested);
        if Some(k.as_str()) == items {
            f.ty = Ty::Generic("Array".to_owned(), vec![Ty::name("Item")]);
        }
        fields.push(f);
    }
    let types = fields.iter().map(|f| (f.key.clone(), f.ty.clone())).collect();
    let decl = Decl {
        name: format!("{ns}<Item>"),
        description: String::new(),
        deprecated: false,
        kind: DeclKind::Interface {
            extends: Vec::new(),
            fields,
        },
        nested: Vec::new(),
    };
    let mut out = print_decl(&decl, 0);
    if !nested.is_empty() {
        out.push(String::new());
        out.push(format!("export namespace {ns} {{"));
        for (i, d) in nested.iter().enumerate() {
            if i > 0 {
                out.push(String::new());
            }
            out.extend(print_decl(d, 2));
        }
        out.push("}".to_owned());
    }
    (out, types)
}

/// The `<Class>Params` interface.
fn params_decl(ctx: &Ctx<'_>, page: Page<'_>, class: &str) -> Vec<String> {
    let name = format!("{class}Params");
    let mut mapper = Mapper::new(ctx, "core/pagination");
    let mut nested = Vec::new();
    let fields = page
        .scheme
        .request
        .iter()
        .map(|(k, v)| mapper.field(&field(k, v, false), &name, &mut nested))
        .collect();
    print_decl(
        &Decl {
            name,
            description: String::new(),
            deprecated: false,
            kind: DeclKind::Interface {
                extends: Vec::new(),
                fields,
            },
            nested,
        },
        0,
    )
}

/// `export class X<Item> extends AbstractPage<Item> implements XResponse<Item> {`, broken over
/// lines when it does not fit.
fn class_head(page: Page<'_>, class: &str) -> Vec<String> {
    let generic = if page.kind() == "cursor_id" {
        format!("Item extends {{ {}: string }}", item_id(page))
    } else {
        "Item".to_owned()
    };
    let one = format!("export class {class}<{generic}> extends AbstractPage<Item> implements {class}Response<Item> {{");
    if one.len() <= WIDTH {
        return vec![one];
    }
    vec![
        format!("export class {class}<{generic}>"),
        "  extends AbstractPage<Item>".to_owned(),
        format!("  implements {class}Response<Item>"),
        "{".to_owned(),
    ]
}

/// One scheme: its response and params interfaces and its class.
fn scheme_lines(ctx: &Ctx<'_>, page: Page<'_>) -> Vec<String> {
    let class = page.class(ctx);
    let (mut out, types) = response_decls(ctx, page, &class);
    out.push(String::new());
    out.extend(params_decl(ctx, page, &class));
    out.push(String::new());
    out.extend(class_head(page, &class));
    for (i, (key, t)) in types.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.push(render(&property_signature(key, false, t), 2, WIDTH));
    }
    if !types.is_empty() {
        out.push(String::new());
    }
    let client = if ctx.node() {
        "APIClient".to_owned()
    } else {
        ctx.settings.client_class.clone()
    };
    let params = [
        text(format!("client: {client}")),
        text("response: Response"),
        text(format!("body: {class}Response<Item>")),
        text("options: FinalRequestOptions"),
    ];
    out.push(render(&signature("constructor", &params, &text(""), " {"), 2, WIDTH));
    out.push("    super(client, response, body, options);".to_owned());
    out.push(String::new());
    for (k, v) in &page.scheme.response {
        let key = js::property_key(k);
        let access = if js::is_identifier(k) {
            format!("body.{k}")
        } else {
            format!("body[{}]", js::string_literal(k))
        };
        let this = if js::is_identifier(k) {
            format!("this.{k}")
        } else {
            format!("this[{key}]")
        };
        match default_value(v) {
            Some(d) => out.push(format!("    {this} = {access} || {d};")),
            None => out.push(format!("    {this} = {access};")),
        }
    }
    out.push("  }".to_owned());
    out.push(String::new());
    out.push("  getPaginatedItems(): Item[] {".to_owned());
    out.push(format!("    return this.{} ?? [];", page.items().unwrap_or("data")));
    out.push("  }".to_owned());
    if let Some(has_more) = page.has_more() {
        out.extend([
            String::new(),
            "  override hasNextPage(): boolean {".to_owned(),
            format!("    return this.{has_more} !== false && super.hasNextPage();"),
            "  }".to_owned(),
        ]);
    }
    out.push(String::new());
    let fake = page.kind() == "fake_page";
    if ctx.node() {
        out.push("  // @deprecated Please use `nextPageInfo()` instead".to_owned());
        if fake {
            out.extend([
                "  /**".to_owned(),
                "   * This page represents a response that isn't actually paginated at the API level".to_owned(),
                "   * so there will never be any next page params.".to_owned(),
                "   */".to_owned(),
                "  nextPageParams(): null {".to_owned(),
                "    return null;".to_owned(),
                "  }".to_owned(),
                String::new(),
                "  nextPageInfo(): null {".to_owned(),
            ]);
        } else {
            out.extend([
                format!("  nextPageParams(): Partial<{class}Params> | null {{"),
                "    const info = this.nextPageInfo();".to_owned(),
                "    if (!info) return null;".to_owned(),
                "    if ('params' in info) return info.params;".to_owned(),
                "    const params = Object.fromEntries(info.url.searchParams);".to_owned(),
                "    if (!Object.keys(params).length) return null;".to_owned(),
                "    return params;".to_owned(),
                "  }".to_owned(),
                String::new(),
                "  nextPageInfo(): PageInfo | null {".to_owned(),
            ]);
        }
    } else {
        out.push("  nextPageRequestOptions(): PageRequestOptions | null {".to_owned());
    }
    out.extend(next_page_body(ctx, page));
    out.push("  }".to_owned());
    out.push("}".to_owned());
    out
}

/// The page classes' file(s); none when the config has no `pagination`.
#[must_use]
pub fn emit_pagination(ctx: &Ctx<'_>) -> Vec<File> {
    let pages = pages(ctx);
    if pages.is_empty() {
        return Vec::new();
    }
    let mut out = vec![HEADER.to_owned(), String::new()];
    if ctx.node() {
        out.push(
            "import { AbstractPage, Response, APIClient, FinalRequestOptions, PageInfo } from './core';".to_owned(),
        );
    } else {
        let client = ctx.settings.client_class.as_str();
        let error = format!("{client}Error");
        out.push(
            fill(HEAD, &[("CLIENT", client), ("ERROR", error.as_str())], &[])
                .trim_end_matches('\n')
                .to_owned(),
        );
    }
    for page in &pages {
        out.push(String::new());
        out.extend(scheme_lines(ctx, *page));
    }
    let text = out.join("\n") + "\n";
    if ctx.node() {
        return vec![("src/pagination.ts".to_owned(), text)];
    }
    vec![
        ("src/core/pagination.ts".to_owned(), text),
        (
            "src/pagination.ts".to_owned(),
            "/** @deprecated Import from ./core/pagination instead */\nexport * from './core/pagination';\n".to_owned(),
        ),
    ]
}
