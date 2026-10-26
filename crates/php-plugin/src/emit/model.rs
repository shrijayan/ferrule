//! A model file: a class using `SdkModel` (and `SdkParams` for a method's parameters), a backed
//! enum, or a class using `SdkUnion`.

use std::fmt::Write as _;

use super::Gen;
use super::file::{docblock, php_file, raw_lines, text_lines};
use crate::layout::{BRACE, cat, doc_chain, doc_list, doc_shape, indent, list, render, text};
use crate::tree::{Class, Decl, EnumDecl, PField, UnionDecl};
use ferrule_sdk_model::model::Literal;

use crate::types::{ClassPath, ImportKind, Imports, PType, join_unique, literal};

/// `src/<path>.php` and its text.
#[must_use]
pub fn model_file(g: &Gen<'_>, d: &Decl) -> (String, String) {
    let path = d.path();
    let text = match d {
        Decl::Class(c) => class_file(g, c),
        Decl::Enum(e) => enum_file(g, e),
        Decl::Union(u) => union_file(g, u),
    };
    (format!("src/{}", path.file()), text)
}

fn imports(g: &Gen<'_>, path: &ClassPath) -> Imports {
    g.imports(path.namespace(), Some(path))
}

/// A method signature `head(params): ret` laid out at `depth` (` {` follows on the generator's
/// line), as php-cs-fixer leaves it.
#[must_use]
pub fn signature(head: &str, params: Vec<String>, ret: &str, depth: usize) -> String {
    let doc = cat(vec![
        list(format!("{head}("), params.into_iter().map(text).collect(), ")"),
        text(ret),
    ]);
    format!("{}{}", indent(depth), render(&doc, depth, depth * 2, BRACE))
}

/// `?T` for a nullable native type.
fn opt(native: &str) -> String {
    PType::nullable_native(native)
}

/// The shape entry of a field: `name?: type|null`.
fn shape_entry(f: &PField, imp: &mut Imports) -> String {
    let optional = !f.required;
    let t = f.ty.shape_doc(imp, optional || f.nullable);
    format!("{}{}: {t}", f.name, if optional { "?" } else { "" })
}

/// The shape alias of a class: `@phpstan-type XShape = array{...}`.
pub fn shape_lines(name: &str, fields: &[&PField], imp: &mut Imports) -> Vec<String> {
    if fields.is_empty() {
        return vec![format!("@phpstan-type {name}Shape = array<string,mixed>")];
    }
    let items: Vec<String> = fields.iter().map(|f| shape_entry(f, imp)).collect();
    raw_lines(doc_shape(&format!("@phpstan-type {name}Shape = "), &items, "", 0, 0))
}

fn property(f: &PField, imp: &mut Imports) -> String {
    let ind = indent(1);
    let mut out = String::new();
    let desc = text_lines(&f.description);
    let var = f.ty.var_doc(imp).map(|t| {
        if !f.required || f.nullable {
            format!("@var {t}|null ${}", f.name)
        } else {
            format!("@var {t} ${}", f.name)
        }
    });
    match (&var, desc.is_empty()) {
        (Some(v), true) => {
            let _ = writeln!(out, "{ind}/** {v} */");
        }
        (v, _) => out.push_str(&docblock(&[desc, v.iter().cloned().collect()], 1)),
    }
    let mut args: Vec<String> = Vec::new();
    if let Some(w) = f.api_name() {
        args.push(literal(&ferrule_sdk_model::model::Literal::String(w.to_owned())));
    }
    if let Some(a) = f.ty.attr_args(imp) {
        args.push(a);
    }
    if f.nullable {
        args.push("nullable: true".to_owned());
    }
    let attr = if f.required { "Required" } else { "Optional" };
    let attr_name = imp.runtime(&["Core", "Attributes", attr]);
    if args.is_empty() {
        let _ = writeln!(out, "{ind}#[{attr_name}]");
    } else {
        let _ = writeln!(out, "{ind}#[{attr_name}({})]", args.join(", "));
    }
    let native = f.ty.native(imp);
    match &f.ty {
        PType::Const(v) => {
            let _ = writeln!(out, "{ind}public {native} ${} = {};", f.name, literal(v));
        }
        _ if f.required && !f.nullable => {
            let _ = writeln!(out, "{ind}public {native} ${};", f.name);
        }
        _ => {
            let _ = writeln!(out, "{ind}public {} ${};", opt(&native), f.name);
        }
    }
    out
}

/// Whether a parameter's phpdoc says more than its native type.
fn needs_param_doc(t: &PType) -> bool {
    !matches!(
        t,
        PType::String
            | PType::Date
            | PType::Int
            | PType::Float
            | PType::Bool
            | PType::DateTime
            | PType::Mixed
            | PType::File
            | PType::Rec(_)
    )
}

fn constructor(c: &Class, imp: &mut Imports) -> String {
    let ind = indent(1);
    let must: Vec<&PField> = c.fields.iter().filter(|f| f.required_value()).collect();
    let name = c.path.name();
    let mut out = String::new();
    if !must.is_empty() {
        let with_args: Vec<String> = must.iter().map(|f| format!("{}: ...", f.name)).collect();
        let setters: Vec<String> = must.iter().map(|f| format!("->with{}(...)", f.setter)).collect();
        let mut enforce = vec!["To enforce required parameters use".to_owned(), "```".to_owned()];
        enforce.extend(raw_lines(doc_list(
            &format!("{name}::with"),
            ("(", ")"),
            &with_args,
            "",
            1,
            1,
        )));
        enforce.push("```".to_owned());
        let mut chain = vec!["```".to_owned()];
        chain.extend(raw_lines(doc_chain(&format!("(new {name})"), &setters, 1, 0)));
        chain.push("```".to_owned());
        out.push_str(&docblock(
            &[
                vec![format!("`new {name}()` is missing required properties by the API.")],
                enforce,
                vec!["Otherwise ensure the following setters are called".to_owned()],
                chain,
            ],
            1,
        ));
    }
    let _ = imp;
    let _ = write!(
        out,
        "{ind}public function __construct()\n{ind}{{\n{ind}    $this->initialize();\n{ind}}}\n"
    );
    out
}

fn with_fn(c: &Class, imp: &mut Imports) -> String {
    let ind = indent(1);
    let fields = c.with_order();
    let mut docs = Vec::new();
    let mut params = Vec::new();
    for f in &fields {
        let native = f.ty.param_native(imp);
        let optional = !f.required;
        if needs_param_doc(&f.ty) {
            let d = f.ty.param_doc(imp);
            let d = if optional || f.nullable { format!("{d}|null") } else { d };
            docs.push(format!("@param {d} ${}", f.name));
        }
        if optional {
            params.push(format!("{} ${} = null", opt(&native), f.name));
        } else if let Some(d) = f.default_literal() {
            let native = if f.nullable { opt(&native) } else { native };
            params.push(format!("{native} ${} = {d}", f.name));
        } else if f.nullable {
            params.push(format!("{} ${}", opt(&native), f.name));
        } else {
            params.push(format!("{native} ${}", f.name));
        }
    }
    let mut out = docblock(
        &[
            vec!["Construct an instance from the required parameters.".to_owned()],
            vec!["You must use named parameters to construct any parameters with a default value.".to_owned()],
            docs,
        ],
        1,
    );
    out.push_str(&signature("public static function with", params, ": self", 1));
    out.push('\n');
    let _ = write!(out, "{ind}{{\n{ind}    $self = new self;\n\n");
    let must: Vec<&PField> = c.fields.iter().filter(|f| f.required_value()).collect();
    let rest: Vec<&PField> = c.fields.iter().filter(|f| !f.required).collect();
    for f in &must {
        let _ = writeln!(out, "{ind}    $self['{0}'] = ${0};", f.name);
    }
    if !must.is_empty() && !rest.is_empty() {
        out.push('\n');
    }
    for f in &rest {
        let _ = writeln!(out, "{ind}    null !== ${0} && $self['{0}'] = ${0};", f.name);
    }
    if !fields.is_empty() {
        out.push('\n');
    }
    let _ = writeln!(out, "{ind}    return $self;\n{ind}}}");
    out
}

fn setter(f: &PField, imp: &mut Imports) -> String {
    let ind = indent(1);
    let native = f.ty.param_native(imp);
    let mut docs = Vec::new();
    if needs_param_doc(&f.ty) {
        let d = f.ty.param_doc(imp);
        let d = if f.nullable { format!("{d}|null") } else { d };
        docs.push(format!("@param {d} ${}", f.name));
    }
    let param = if f.nullable {
        format!("{} ${}", opt(&native), f.name)
    } else {
        format!("{native} ${}", f.name)
    };
    let mut out = docblock(&[text_lines(&f.description), docs], 1);
    out.push_str(&signature(
        &format!("public function with{}", f.setter),
        vec![param],
        ": self",
        1,
    ));
    out.push('\n');
    let _ = write!(
        out,
        "{ind}{{\n{ind}    $self = clone $this;\n{ind}    $self['{0}'] = ${0};\n\n{ind}    return $self;\n{ind}}}\n",
        f.name
    );
    out
}

fn class_file(g: &Gen<'_>, c: &Class) -> String {
    let mut imp = imports(g, &c.path);
    let name = c.path.name().to_owned();
    // The body first, so that names are imported in the order Stainless imports them.
    let model = imp.runtime(&["Core", "Concerns", "SdkModel"]);
    let params = c.params.then(|| imp.runtime(&["Core", "Concerns", "SdkParams"]));
    let base = imp.runtime(&["Core", "Contracts", "BaseModel"]);
    let declared = c.declared();
    let mut members = Vec::new();
    for f in &declared {
        members.push(property(f, &mut imp));
    }
    members.push(constructor(c, &mut imp));
    members.push(with_fn(c, &mut imp));
    for f in &c.fields {
        members.push(setter(f, &mut imp));
    }
    let shape = shape_lines(&name, &c.fields.iter().collect::<Vec<_>>(), &mut imp);
    let mut body = docblock(
        &[
            text_lines(&c.description),
            c.see.iter().map(|s| format!("@see {s}")).collect(),
            imp.type_import_lines(),
            shape,
        ],
        0,
    );
    let _ = writeln!(body, "final class {name} implements {base}\n{{");
    let _ = writeln!(body, "    /** @use {model}<{name}Shape> */\n    use {model};");
    if let Some(p) = params {
        let _ = writeln!(body, "    use {p};");
    }
    for m in &members {
        body.push('\n');
        body.push_str(m);
    }
    body.push_str("}\n");
    php_file(&g.ns, &imp, &body)
}

fn enum_file(g: &Gen<'_>, e: &EnumDecl) -> String {
    let imp = imports(g, &e.path);
    let backing = crate::types::Backing::of(e.members.first().map(|m| &m.value)).native();
    let mut body = docblock(&[text_lines(&e.description)], 0);
    let _ = writeln!(body, "enum {}: {backing}\n{{", e.path.name());
    let cases: Vec<String> = e
        .members
        .iter()
        .map(|m| format!("    case {} = {};\n", m.case, literal(&m.value)))
        .collect();
    body.push_str(&cases.join("\n"));
    body.push_str("}\n");
    php_file(&g.ns, &imp, &body)
}

/// A union variant's converter in `variants()`: `'string'`, `X::class`, `new ListOf(...)`.
fn variant_converter(t: &PType, imp: &mut Imports) -> String {
    match t {
        PType::List(inner) => {
            let list_of = imp.runtime(&["Core", "Conversion", "ListOf"]);
            format!("new {list_of}({})", inner.converter(imp))
        }
        PType::Map(inner) => {
            let map_of = imp.runtime(&["Core", "Conversion", "MapOf"]);
            format!("new {map_of}({})", inner.converter(imp))
        }
        // A variant of one value is converted as its type.
        PType::Const(_) => format!("'{}'", t.native(imp)),
        other => other.converter(imp),
    }
}

fn union_file(g: &Gen<'_>, u: &UnionDecl) -> String {
    let mut imp = imports(g, &u.path);
    let name = u.path.name().to_owned();
    let sdk_union = imp.runtime(&["Core", "Concerns", "SdkUnion"]);
    let converter = imp.runtime(&["Core", "Conversion", "Contracts", "Converter"]);
    let source = imp.runtime(&["Core", "Conversion", "Contracts", "ConverterSource"]);
    let entries: Vec<String> = u
        .variants
        .iter()
        .map(|v| {
            let conv = variant_converter(&v.ty, &mut imp);
            match &v.key {
                Some(k) if u.discriminator.is_some() => format!("'{k}' => {conv}"),
                _ => conv,
            }
        })
        .collect();
    let docs: Vec<String> = u
        .variants
        .iter()
        .map(|v| match &v.ty {
            PType::Const(Literal::Boolean(_)) => v.ty.native(&mut imp),
            t => t.doc(&mut imp),
        })
        .collect();
    let mut variants_type = join_unique(docs);
    // `mixed` (a variant on a reference cycle) comes first.
    if variants_type.split('|').any(|t| t == "mixed") {
        let rest: Vec<&str> = variants_type.split('|').filter(|t| *t != "mixed").collect();
        variants_type = std::iter::once("mixed").chain(rest).collect::<Vec<_>>().join("|");
    }
    let mut shape_parts = vec![format!("{name}Variants")];
    for v in &u.variants {
        let inner = match &v.ty {
            PType::Rec(t) => &**t,
            t => t,
        };
        if matches!(inner, PType::Model(_) | PType::Union(..)) {
            shape_parts.push(
                inner
                    .param_doc(&mut imp)
                    .rsplit('|')
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }
    let shape = join_unique(shape_parts);
    let mut body = docblock(
        &[
            text_lines(&u.description),
            imp.type_import_lines(),
            vec![
                format!("@phpstan-type {name}Variants = {variants_type}"),
                format!("@phpstan-type {name}Shape = {shape}"),
            ],
        ],
        0,
    );
    let _ = ImportKind::Other;
    let _ = write!(
        body,
        "final class {name} implements {source}\n{{\n    use {sdk_union};\n"
    );
    if let Some(d) = &u.discriminator {
        let _ = write!(
            body,
            "\n    public static function discriminator(): string\n    {{\n        return '{d}';\n    }}\n"
        );
    }
    let _ = write!(
        body,
        "\n    /**\n     * @return list<string|{converter}|{source}>|array<string,string|{converter}|{source}>\n     */\n"
    );
    body.push_str("    public static function variants(): array\n    {\n");
    let ret = cat(vec![
        text("return "),
        list("[", entries.into_iter().map(text).collect(), "]"),
        text(";"),
    ]);
    let _ = writeln!(body, "{}{}", indent(2), render(&ret, 2, 4, 0));
    body.push_str("    }\n}\n");
    php_file(&g.ns, &imp, &body)
}
