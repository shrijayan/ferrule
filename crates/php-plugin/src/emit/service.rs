//! The services of each resource: `Services/<Path>Service.php` (named parameters, which it
//! collects and passes to the raw service), `Services/<Path>RawService.php` (a params array or
//! object, sent with the client), and their interfaces under `ServiceContracts/`. One emitter
//! prints all four, switched by [`Kind`].

use std::fmt::Write as _;

use super::file::{docblock, php_file, raw_lines, text_lines};
use super::model::signature;
use super::{Gen, Meth, Res};
use crate::layout::{cat, doc_shape, indent, list, render, text};
use crate::tree::PField;
use crate::types::{ClassPath, ImportKind, Imports, PType};

/// Which of a resource's four classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Service,
    Raw,
    Contract,
    RawContract,
}

impl Kind {
    const ALL: [Self; 4] = [Self::Service, Self::Raw, Self::Contract, Self::RawContract];

    fn raw(self) -> bool {
        matches!(self, Self::Raw | Self::RawContract)
    }

    fn contract(self) -> bool {
        matches!(self, Self::Contract | Self::RawContract)
    }

    fn class(self, r: &Res<'_>) -> ClassPath {
        match self {
            Self::Service => r.class("Services", "Service"),
            Self::Raw => r.class("Services", "RawService"),
            Self::Contract => r.class("ServiceContracts", "Contract"),
            Self::RawContract => r.class("ServiceContracts", "RawContract"),
        }
    }
}

/// Every service and contract file.
#[must_use]
pub fn service_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for r in &g.resources {
        for kind in Kind::ALL {
            let path = kind.class(r);
            out.push((format!("src/{}", path.file()), file(g, r, kind, &path)));
        }
    }
    out
}

fn request_opts(imp: &mut Imports) -> String {
    let opts = ClassPath::new(vec!["RequestOptions".to_owned()]);
    imp.type_alias("RequestOpts", &opts, ImportKind::Other)
}

/// The params class of a method, if it has one.
fn params_class<'m>(m: &'m Meth<'_>) -> Option<&'m crate::tree::Class> {
    m.args.params.as_ref()
}

/// A service parameter: `string $name`, `?bool $x = null`, `int $limit = 1000`.
fn service_param(f: &PField, imp: &mut Imports) -> String {
    let native = f.ty.param_native(imp);
    if f.must_pass() {
        if f.nullable {
            format!("{} ${}", PType::nullable_native(&native), f.name)
        } else {
            format!("{native} ${}", f.name)
        }
    } else if let PType::Const(v) = &f.ty {
        format!("{native} ${} = {}", f.name, crate::types::literal(v))
    } else if let Some(d) = f.default_literal() {
        format!("{native} ${} = {d}", f.name)
    } else {
        format!("{} ${} = null", PType::nullable_native(&native), f.name)
    }
}

/// The service's parameters: those a caller must pass, then the others, in field order.
fn service_fields<'m>(m: &'m Meth<'_>) -> Vec<&'m PField> {
    let fields: Vec<&PField> = params_class(m).map(|c| c.fields.iter().collect()).unwrap_or_default();
    let mut out: Vec<&PField> = fields.iter().copied().filter(|f| f.must_pass()).collect();
    out.extend(fields.iter().copied().filter(|f| !f.must_pass()));
    out
}

/// `@param` for a parameter with a description, or whose type says more than its native type.
fn param_doc(f: &PField, imp: &mut Imports) -> Option<String> {
    let t = f.ty.param_doc(imp);
    let t = if f.nullable { format!("{t}|null") } else { t };
    let desc = text_lines(&f.description);
    let native = f.ty.param_native(imp);
    if desc.is_empty() && t == native {
        return None;
    }
    let mut line = format!("@param {t} ${}", f.name);
    let mut lines = desc.into_iter();
    if let Some(first) = lines.next() {
        line.push(' ');
        line.push_str(&first);
    }
    let rest: Vec<String> = lines.collect();
    Some(if rest.is_empty() {
        line
    } else {
        format!("{line}\n{}", rest.join("\n"))
    })
}

/// The response's native return type and its `@return` (`None` when the native type says it).
fn returns(m: &Meth<'_>, imp: &mut Imports) -> (String, Option<String>) {
    match &m.response {
        None => ("mixed".to_owned(), None),
        Some(t) => {
            let native = match t {
                PType::Union(_, variants) => {
                    crate::types::join_unique(variants.iter().map(|v| v.native(imp)).collect())
                }
                t => t.native(imp),
            };
            let doc = t.return_doc(imp);
            if doc == native {
                (native, None)
            } else {
                (native, Some(doc))
            }
        }
    }
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines().map(str::to_owned).collect()
}

fn method_description(m: &Meth<'_>) -> Vec<String> {
    let d = if m.method.description.trim().is_empty() {
        &m.method.summary
    } else {
        &m.method.description
    };
    text_lines(d)
}

/// The raw service's `@param array{...}|XParams $params` lines.
fn raw_params_doc(c: &crate::tree::Class, imp: &mut Imports, kind: Kind) -> Vec<String> {
    let class = imp.class(&c.path);
    if kind == Kind::RawContract {
        return vec![format!("@param array<string,mixed>|{class} $params")];
    }
    let items: Vec<String> = c
        .fields
        .iter()
        .map(|f| {
            let t = f.ty.param_doc(imp);
            let t = if f.nullable { format!("{t}|null") } else { t };
            format!("{}{}: {t}", f.name, if f.required { "" } else { "?" })
        })
        .collect();
    raw_lines(doc_shape("@param ", &items, &format!("|{class} $params"), 1, 3))
}

fn method(g: &Gen<'_>, m: &Meth<'_>, kind: Kind, imp: &mut Imports) -> String {
    let ind = indent(1);
    let mut params_doc: Vec<String> = Vec::new();
    let mut sig: Vec<String> = Vec::new();
    if let Some(p) = &m.args.positional {
        let native = p.ty.param_native(imp);
        if let Some(d) = param_doc(p, imp) {
            params_doc.extend(split_lines(&d));
        }
        sig.push(format!("{native} ${}", p.name));
    }
    let class = params_class(m);
    let fields = service_fields(m);
    if kind.raw() {
        if let Some(c) = class {
            params_doc.extend(raw_params_doc(c, imp, kind));
            sig.push(format!("array|{} $params", imp.class(&c.path)));
        }
    } else {
        for f in &fields {
            // A path parameter keeps its description on the method.
            let documented = m.args.path.iter().find(|p| p.name == f.name).unwrap_or(f);
            if let Some(d) = param_doc(documented, imp) {
                params_doc.extend(split_lines(&d));
            }
            sig.push(service_param(f, imp));
        }
    }
    let opts = imp.class(&ClassPath::new(vec!["RequestOptions".to_owned()]));
    params_doc.push(format!("@param {}|null $requestOptions", request_opts(imp)));
    sig.push(format!("{opts}|array|null $requestOptions = null"));
    let (native, ret_doc) = returns(m, imp);
    let base_response = imp.runtime(&["Core", "Contracts", "BaseResponse"]);
    let (ret_native, ret_doc) = if kind.raw() {
        let inner = m
            .response
            .as_ref()
            .map_or_else(|| "mixed".to_owned(), |t| t.return_doc(imp));
        (base_response.clone(), Some(format!("BaseResponse<{inner}>")))
    } else {
        (native, ret_doc)
    };
    let exception = imp.runtime(&["Core", "Exceptions", "APIException"]);
    let mut sections = vec![vec!["@api".to_owned()]];
    if !kind.contract() {
        sections.push(method_description(m));
    }
    sections.push(params_doc);
    if let Some(r) = ret_doc {
        sections.push(vec![format!("@return {r}")]);
    }
    sections.push(vec![format!("@throws {exception}")]);
    let mut out = docblock(&sections, 1);
    let head = format!("public function {}", m.name);
    let sig_text = signature(&head, sig, &format!(": {ret_native}"), 1);
    if kind.contract() {
        out.push_str(&sig_text);
        out.push_str(";\n");
        return out;
    }
    out.push_str(&sig_text);
    out.push_str(" {\n");
    let body = if kind == Kind::Raw {
        raw_body(g, m, imp)
    } else {
        service_body(m, &fields, imp)
    };
    out.push_str(&body);
    let _ = writeln!(out, "{ind}}}");
    out
}

/// `$params = Util::removeNulls([...]);` and the call of the raw service.
fn service_body(m: &Meth<'_>, fields: &[&PField], imp: &mut Imports) -> String {
    let ind2 = indent(2);
    let mut out = String::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = &m.args.positional {
        args.push(format!("${}", p.name));
    }
    if let Some(c) = params_class(m) {
        let util = imp.runtime(&["Core", "Util"]);
        let _ = fields;
        let entries = c
            .fields
            .iter()
            .map(|f| text(format!("'{0}' => ${0}", f.name)))
            .collect();
        let stmt = cat(vec![
            text("$params = "),
            list(format!("{util}::removeNulls("), vec![list("[", entries, "]")], ")"),
            text(";"),
        ]);
        let _ = write!(out, "{ind2}{}\n\n", render(&stmt, 2, 4, 0));
        args.push("params: $params".to_owned());
    }
    args.push("requestOptions: $requestOptions".to_owned());
    let _ = writeln!(out, "{ind2}// @phpstan-ignore-next-line argument.type");
    let _ = write!(
        out,
        "{ind2}$response = $this->raw->{}({});\n\n",
        m.name,
        args.join(", ")
    );
    let _ = writeln!(out, "{ind2}return $response->parse();");
    out
}

/// A path template with its parameters: `'v1/folder'` or `['v1/files/%1$s', $fileID]`.
fn path_expr(m: &Meth<'_>) -> String {
    let path = m.method.path.trim_start_matches('/');
    let mut names: Vec<String> = Vec::new();
    let mut out = String::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}') else { break };
        let wire = &rest[start + 1..start + end];
        let field = m
            .args
            .path
            .iter()
            .chain(m.args.positional.iter())
            .find(|f| f.wire == wire);
        names.push(field.map_or_else(|| wire.to_owned(), |f| f.name.clone()));
        let _ = write!(out, "%{}$s", names.len());
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    if names.is_empty() {
        format!("'{out}'")
    } else {
        let vars: Vec<String> = names.iter().map(|n| format!("${n}")).collect();
        format!("['{out}', {}]", vars.join(", "))
    }
}

fn raw_body(g: &Gen<'_>, m: &Meth<'_>, imp: &mut Imports) -> String {
    let ind2 = indent(2);
    let ind3 = indent(3);
    let mut out = String::new();
    let class = params_class(m);
    let body = m.method.body.is_some() && class.is_some();
    // With a body as well, the query parameters are picked out of the parsed params by their
    // wire names (the keys `parseRequest` dumps) and the body is the rest, as Stainless's PHP
    // SDKs split them (courier-php, telnyx-php).
    let split = body && !m.args.query.is_empty();
    let options = if let Some(c) = class {
        let name = imp.class(&c.path);
        let _ = write!(
            out,
            "{ind2}[$parsed, $options] = {name}::parseRequest(\n{ind3}$params,\n{ind3}$requestOptions,\n{ind2});\n"
        );
        for p in &m.args.path {
            let _ = write!(
                out,
                "{ind2}${0} = $parsed['{0}'];\n{ind2}unset($parsed['{0}']);\n",
                p.name
            );
        }
        if split {
            let keys: Vec<String> = m.args.query.iter().map(|f| format!("'{}'", f.wire)).collect();
            let _ = writeln!(out, "{ind2}$query_params = array_flip([{}]);", keys.join(", "));
        }
        out.push('\n');
        "$options"
    } else {
        "$requestOptions"
    };
    let _ = writeln!(out, "{ind2}// @phpstan-ignore-next-line return.type");
    let _ = writeln!(out, "{ind2}return $this->client->request(");
    let _ = writeln!(out, "{ind3}method: '{}',", m.method.http_method.as_str());
    let _ = writeln!(out, "{ind3}path: {},", path_expr(m));
    if split {
        let _ = writeln!(out, "{ind3}query: array_intersect_key($parsed, $query_params),");
    } else if !m.args.query.is_empty() {
        let _ = writeln!(out, "{ind3}query: $parsed,");
    }
    if m.method
        .body
        .as_ref()
        .is_some_and(|b| b.media_type == "multipart/form-data")
    {
        let _ = writeln!(out, "{ind3}headers: ['Content-Type' => 'multipart/form-data'],");
    }
    if split {
        let _ = writeln!(out, "{ind3}body: (object) array_diff_key($parsed, $query_params),");
    } else if body {
        let _ = writeln!(out, "{ind3}body: (object) $parsed,");
    }
    let _ = writeln!(out, "{ind3}options: {options},");
    let convert = m
        .response
        .as_ref()
        .map_or_else(|| "null".to_owned(), |t| t.response_converter(imp));
    let _ = writeln!(out, "{ind3}convert: {convert},");
    let _ = writeln!(out, "{ind2});");
    let _ = g;
    out
}

fn file(g: &Gen<'_>, r: &Res<'_>, kind: Kind, path: &ClassPath) -> String {
    let mut imp = g.imports(path.namespace(), Some(path));
    let name = path.name().to_owned();
    let mut members: Vec<String> = Vec::new();
    let client = if kind.contract() {
        String::new()
    } else {
        imp.class(&ClassPath::new(vec!["Client".to_owned()]))
    };
    let contract = match kind {
        Kind::Service => Some(imp.class(&Kind::Contract.class(r))),
        Kind::Raw => Some(imp.class(&Kind::RawContract.class(r))),
        _ => None,
    };
    if kind == Kind::Service {
        let raw = imp.class(&Kind::Raw.class(r));
        members.push(format!("    /**\n     * @api\n     */\n    public {raw} $raw;\n"));
        let mut init = vec![format!("        $this->raw = new {raw}($client);\n")];
        for &i in &r.children {
            let child = &g.resources[i];
            let svc = imp.class(&Kind::Service.class(child));
            members.push(format!(
                "    /**\n     * @api\n     */\n    public {svc} ${};\n",
                child.attr
            ));
            init.push(format!("        $this->{} = new {svc}($client);\n", child.attr));
        }
        members.push(format!(
            "    /**\n     * @internal\n     */\n    public function __construct(private {client} $client)\n    {{\n{}    }}\n",
            init.concat()
        ));
    } else if kind == Kind::Raw {
        members.push(format!(
            "    // @phpstan-ignore-next-line\n    /**\n     * @internal\n     */\n    public function __construct(private {client} $client) {{}}\n"
        ));
    }
    for m in &r.methods {
        members.push(method(g, m, kind, &mut imp));
    }
    let head = imp.type_import_lines();
    let mut body = docblock(&[head], 0);
    let decl = if kind.contract() {
        format!("interface {name}")
    } else {
        format!("final class {name} implements {}", contract.unwrap_or_default())
    };
    if members.is_empty() {
        let _ = writeln!(body, "{decl} {{}}");
    } else {
        let _ = writeln!(body, "{decl}\n{{");
        body.push_str(&members.join("\n"));
        body.push_str("}\n");
    }
    php_file(&g.ns, &imp, &body)
}
