//! `Models/**.cs`: params classes (`ParamsBase`), models (`JsonModel`), enums with their JSON
//! converters, and unions (`ModelBase`), as the generator writes them before csharpier lays the
//! code out.

use std::fmt::Write as _;

use ferrule_sdk_model::model::Literal;

use crate::doc;
use crate::emit::{Gen, local_of, usings};
use crate::tree::{Decl, En, Kind, Loc, ModelFile, Obj, Prop, Un};
use crate::types::CsType;

/// The state of one file: its namespace and how it spells `System` types.
pub struct FileCtx<'a> {
    pub g: &'a Gen<'a>,
    /// The namespace below `Models`.
    pub ns: Vec<String>,
    /// Whether the file aliases `System` (`using System = System;`) and so writes `System::Uri`.
    pub sys: bool,
}

impl FileCtx<'_> {
    /// A `System` type: `System::Uri` in a file that aliases `System`, else `Uri`.
    #[must_use]
    pub fn system(&self, name: &str) -> String {
        if self.sys {
            format!("System::{name}")
        } else {
            name.to_owned()
        }
    }

    /// The declared name of an enum called `Type`, which clashes with `System.Type`.
    #[must_use]
    pub fn qualified(&self, name: &str) -> String {
        if name == "Type" {
            format!("global::{}.Type", self.g.models_ns(&self.ns))
        } else {
            name.to_owned()
        }
    }
}

/// A literal as C# writes it.
#[must_use]
pub fn literal(l: &Literal) -> String {
    match l {
        Literal::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Literal::Boolean(b) => b.to_string(),
        Literal::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{n:.1}")
            } else {
                n.to_string()
            }
        }
    }
}

/// The doc place of a file's `i`th declaration: the top-level one is a model's, an inline enum
/// or union is a property's type.
fn place(i: usize) -> doc::Place {
    if i == 0 { doc::Place::Model } else { doc::Place::Variant }
}

/// Whether a file aliases `System`: an enum's or union's converter names `System::Type`, which
/// an enum called `Type` would hide.
fn file_uses_system(file: &ModelFile) -> bool {
    file.decls.iter().any(|d| matches!(d, Decl::Enum(_) | Decl::Union(_)))
}

/// The text of one model file.
#[must_use]
pub fn model_file(g: &Gen<'_>, file: &ModelFile) -> (String, String) {
    let ctx = FileCtx {
        g,
        ns: file.ns.clone(),
        sys: file_uses_system(file),
    };
    let mut body = String::new();
    let mut used: Vec<String> = Vec::new();
    for (i, d) in file.decls.iter().enumerate() {
        if i > 0 {
            body.push('\n');
        }
        match d {
            Decl::Object(o) if i == 0 && matches!(file.kind, Kind::Params(..)) => {
                body.push_str(&params_class(&ctx, file, o, &mut used));
            }
            Decl::Object(o) | Decl::Member(o) => body.push_str(&model_class(&ctx, o, &mut used)),
            Decl::Enum(e) => body.push_str(&enum_decl(&ctx, e, place(i), &mut used)),
            Decl::Union(u) => body.push_str(&union_decl(&ctx, u, place(i), &mut used)),
        }
    }
    let ns = g.models_ns(&file.ns);
    let mut list = used;
    if ctx.sys {
        list.push("System = System".to_owned());
        list.retain(|u| u != "System");
    }
    let mut namespaces = Vec::new();
    for d in &file.decls {
        if let Decl::Object(o) | Decl::Member(o) = d {
            for p in &o.props {
                p.ty.namespaces(&mut namespaces);
            }
        }
        if let Decl::Union(u) = d {
            for v in &u.variants {
                v.ty.namespaces(&mut namespaces);
            }
        }
    }
    for other in namespaces {
        // A model in this namespace or an enclosing one is found without a using.
        if !file.ns.starts_with(&other) {
            list.push(g.models_ns(&other));
        }
    }
    // Without the alias, `System` types are imported instead.
    let body = if ctx.sys || !body.contains("System::") {
        body
    } else {
        list.push("System".to_owned());
        body.replace("System::", "")
    };
    let text = format!("{}\nnamespace {ns};\n\n{body}", sort_usings(&list));
    (format!("src/{}/{}", g.target.ns, file.path()), text)
}

/// `using` directives as csharpier sorts them: `System` ones first, then the rest, then
/// aliases.
#[must_use]
pub fn sort_usings(list: &[String]) -> String {
    let mut plain: Vec<String> = list.iter().filter(|u| !u.contains(" = ")).cloned().collect();
    let mut aliases: Vec<String> = list.iter().filter(|u| u.contains(" = ")).cloned().collect();
    plain.sort_by(|a, b| {
        let sa = a == "System" || a.starts_with("System.");
        let sb = b == "System" || b.starts_with("System.");
        sb.cmp(&sa).then_with(|| a.cmp(b))
    });
    plain.dedup();
    aliases.sort();
    aliases.dedup();
    let mut out = usings(plain);
    for a in aliases {
        let _ = writeln!(out, "using {a};");
    }
    out
}

/// The accessor suffix and type argument of a property's getter.
fn getter(p: &Prop) -> String {
    let storage = p.ty.storage();
    let kind = if p.ty.is_struct() { "Struct" } else { "Class" };
    let null = if p.required && !p.nullable {
        "NotNull"
    } else {
        "Nullable"
    };
    format!("Get{null}{kind}<{storage}>(\"{}\")", p.wire)
}

/// The `init` accessor's body.
fn setter(p: &Prop) -> String {
    let f = p.loc.field();
    let optional = !p.required || p.nullable;
    let set = match &p.ty {
        CsType::List(_) | CsType::Dict(_) => {
            let convert = if matches!(p.ty, CsType::List(_)) {
                "ImmutableArray.ToImmutableArray(value)"
            } else {
                "FrozenDictionary.ToFrozenDictionary(value)"
            };
            if optional {
                format!(
                    "this.{f}.Set<{}?>(\"{}\", value == null ? null : {convert});",
                    p.ty.storage(),
                    p.wire
                )
            } else {
                format!("this.{f}.Set<{}>(\"{}\", {convert});", p.ty.storage(), p.wire)
            }
        }
        _ => format!("this.{f}.Set(\"{}\", value);", p.wire),
    };
    if !p.required && !p.nullable {
        format!("if (value == null)\n{{\nreturn;\n}}\n\n{set}")
    } else {
        set
    }
}

fn prop_doc(p: &Prop, out: &mut String) {
    out.push_str(&doc::summary(&p.description, 4, doc::Place::Property, &[]));
}

/// A property backed by a raw dictionary.
fn raw_prop(p: &Prop, out: &mut String) {
    prop_doc(p, out);
    let f = p.loc.field();
    let optional = !p.required || p.nullable;
    let q = if optional && p.constant.is_none() { "?" } else { "" };
    let req = if p.required && p.constant.is_none() {
        "required "
    } else {
        ""
    };
    let ty = p.ty.spell();
    if p.constant.is_some() {
        let _ = write!(
            out,
            "    public {ty} {name}\n    {{\n        get\n        {{\n            this.{f}.Freeze();\n            return this.{f}.GetNotNullStruct<JsonElement>(\"{wire}\");\n        }}\n        init {{ this.{f}.Set(\"{wire}\", value); }}\n    }}\n",
            name = p.name,
            wire = p.wire
        );
        return;
    }
    let init = setter(p);
    let init = if init.contains('\n') {
        format!("init\n{{\n{init}\n}}")
    } else {
        format!("init {{ {init} }}")
    };
    let _ = write!(
        out,
        "    public {req}{ty}{q} {name}\n    {{\n        get\n        {{\n            this.{f}.Freeze();\n            return this.{f}.{getter};\n        }}\n{init}\n    }}\n",
        name = p.name,
        getter = getter(p),
    );
}

/// `Validate`'s line for one property.
fn validate_line(p: &Prop, ctx: &FileCtx<'_>) -> String {
    let optional = !p.required || p.nullable;
    if let Some(c) = &p.constant {
        return format!(
            "if (!JsonElement.DeepEquals(this.{n}, JsonSerializer.SerializeToElement({v})))\n{{\nthrow new {e}InvalidDataException(\"Invalid value given for constant\");\n}}",
            n = p.name,
            v = literal(c),
            e = ctx.g.target.client
        );
    }
    match &p.ty {
        CsType::List(t) if t.validates() => {
            if optional {
                format!("foreach (var item in this.{} ?? [])\n{{\nitem.Validate();\n}}", p.name)
            } else {
                format!("foreach (var item in this.{})\n{{\nitem.Validate();\n}}", p.name)
            }
        }
        CsType::Dict(t) if t.validates() => {
            if optional {
                format!(
                    "if (this.{n} != null)\n{{\nforeach (var item in this.{n}.Values)\n{{\nitem.Validate();\n}}\n}}",
                    n = p.name
                )
            } else {
                format!("foreach (var item in this.{}.Values)\n{{\nitem.Validate();\n}}", p.name)
            }
        }
        t if t.validates() => {
            if optional {
                format!("this.{}?.Validate();", p.name)
            } else {
                format!("this.{}.Validate();", p.name)
            }
        }
        _ => format!("_ = this.{};", p.name),
    }
}

fn note_uses(ctx: &FileCtx<'_>, props: &[Prop], used: &mut Vec<String>) {
    for p in props {
        if p.ty.uses_list() {
            used.push("System.Collections.Immutable".to_owned());
        }
        if p.constant.is_some() {
            used.push(exceptions(ctx));
        }
    }
}

fn exceptions(ctx: &FileCtx<'_>) -> String {
    format!("{}.Exceptions", ctx.g.target.ns)
}

fn core(ctx: &FileCtx<'_>) -> String {
    format!("{}.Core", ctx.g.target.ns)
}

/// A `JsonModel` class and its `FromRaw` class.
// One straight-line template of the generated C#; splitting it would scatter the file's
// layout across helpers.
#[allow(clippy::too_many_lines)]
fn model_class(ctx: &FileCtx<'_>, o: &Obj, used: &mut Vec<String>) -> String {
    let n = &o.name;
    let local = local_of(n);
    used.extend(
        [
            "System.Collections.Frozen",
            "System.Collections.Generic",
            "System.Diagnostics.CodeAnalysis",
            "System.Text.Json",
            "System.Text.Json.Serialization",
        ]
        .map(str::to_owned),
    );
    used.push(core(ctx));
    note_uses(ctx, &o.props, used);
    let mut out = String::new();
    out.push_str(&doc::summary(&o.description, 0, doc::Place::Model, &[]));
    let _ = write!(
        out,
        "[JsonConverter(typeof(JsonModelConverter<{n}, {n}FromRaw>))]\npublic sealed record class {n} : JsonModel\n{{\n"
    );
    for (i, p) in o.props.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        raw_prop(p, &mut out);
    }
    for (base, names) in &o.bases {
        let assigns: String = names
            .iter()
            .map(|p| format!("{p} = {local}.{p},\n"))
            .collect::<Vec<_>>()
            .concat();
        let _ = write!(
            out,
            "\n    public static implicit operator {base}({n} {local}) =>\n        new()\n        {{\n{assigns}        }};\n"
        );
    }
    let validate: Vec<String> = o.props.iter().map(|p| validate_line(p, ctx)).collect();
    let _ = write!(
        out,
        "{sep}    /// <inheritdoc/>\n    public override void Validate()\n    {{\n{body}    }}\n",
        sep = if o.props.is_empty() && o.bases.is_empty() {
            ""
        } else {
            "\n"
        },
        body = validate.iter().map(|l| format!("{l}\n")).collect::<Vec<_>>().concat()
    );
    let consts: Vec<&Prop> = o.props.iter().filter(|p| p.constant.is_some()).collect();
    let const_inits: String = consts
        .iter()
        .map(|p| {
            format!(
                "this.{} = JsonSerializer.SerializeToElement({});\n",
                p.name,
                literal(p.constant.as_ref().unwrap_or(&Literal::Boolean(false)))
            )
        })
        .collect::<Vec<_>>()
        .concat();
    if consts.is_empty() {
        let _ = write!(out, "\n    public {n}() {{ }}\n");
    } else {
        let _ = write!(out, "\n    public {n}()\n    {{\n{const_inits}    }}\n");
    }
    let _ = write!(
        out,
        "\n#pragma warning disable CS8618\n    [SetsRequiredMembers]\n    public {n}({n} {local})\n        : base({local}) {{ }}\n#pragma warning restore CS8618\n"
    );
    let raw_init = if consts.is_empty() {
        String::new()
    } else {
        format!("\n{const_inits}")
    };
    let _ = write!(
        out,
        "\n    public {n}(IReadOnlyDictionary<string, JsonElement> rawData)\n    {{\n        this._rawData = new(rawData);\n{raw_init}    }}\n"
    );
    let _ = write!(
        out,
        "\n#pragma warning disable CS8618\n    [SetsRequiredMembers]\n    {n}(FrozenDictionary<string, JsonElement> rawData)\n    {{\n        this._rawData = new(rawData);\n    }}\n#pragma warning restore CS8618\n"
    );
    let _ = write!(
        out,
        "\n    /// <inheritdoc cref=\"{n}FromRaw.FromRawUnchecked\"/>\n    public static {n} FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)\n    {{\n        return new(FrozenDictionary.ToFrozenDictionary(rawData));\n    }}\n"
    );
    let required: Vec<&Prop> = o.props.iter().filter(|p| p.required && p.constant.is_none()).collect();
    if let [single] = required.as_slice() {
        let arg = crate::naming::local(local_of(&single.name));
        let _ = write!(
            out,
            "\n    [SetsRequiredMembers]\n    public {n}({ty} {arg})\n        : this()\n    {{\n        this.{p} = {arg};\n    }}\n",
            ty = single.ty.spell(),
            p = single.name
        );
    }
    let _ = write!(
        out,
        "}}\n\nclass {n}FromRaw : IFromRawJson<{n}>\n{{\n    /// <inheritdoc/>\n    public {n} FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>\n        {n}.FromRawUnchecked(rawData);\n}}\n"
    );
    out
}

/// A params class.
// One straight-line template of the generated C#; splitting it would scatter the file's
// layout across helpers.
#[allow(clippy::too_many_lines)]
fn params_class(ctx: &FileCtx<'_>, file: &ModelFile, o: &Obj, used: &mut Vec<String>) -> String {
    let n = &o.name;
    let local = local_of(n);
    let Kind::Params(si, mi) = file.kind else {
        return String::new();
    };
    let meth = &ctx.g.services[si].methods[mi];
    let m = meth.method;
    let whole = o.props.iter().find(|p| p.loc == Loc::WholeBody);
    let has_body = m.body.is_some() && whole.is_none();
    let any_body = m.body.is_some();
    used.extend(
        [
            "System",
            "System.Collections.Frozen",
            "System.Collections.Generic",
            "System.Diagnostics.CodeAnalysis",
            "System.Net.Http",
            "System.Text.Json",
        ]
        .map(str::to_owned),
    );
    used.push(core(ctx));
    if any_body {
        used.push("Text = System.Text".to_owned());
    }
    note_uses(ctx, &o.props, used);
    let mut out = String::new();
    out.push_str(&doc::summary(&o.description, 0, doc::Place::Params, &[doc::NOTE_PARA]));
    let _ = write!(out, "public record class {n} : ParamsBase\n{{\n");
    let mut first = true;
    if has_body {
        out.push_str("    readonly JsonDictionary _rawBodyData = new();\n    public IReadOnlyDictionary<string, JsonElement> RawBodyData\n    {\n        get { return this._rawBodyData.Freeze(); }\n    }\n");
        first = false;
    }
    if whole.is_some() {
        out.push_str("    public JsonElement RawBodyData { get; private init; }\n");
        first = false;
    }
    let path: Vec<&Prop> = o.props.iter().filter(|p| p.loc == Loc::Path).collect();
    for p in &o.props {
        if !first {
            out.push('\n');
        }
        first = false;
        if p.loc == Loc::Path {
            // The last path param is the overloads' argument, so the params may leave it unset.
            if path.last().is_some_and(|l| l.name == p.name) {
                let _ = writeln!(out, "    public {}? {} {{ get; init; }}", p.ty.spell(), p.name);
            } else {
                let _ = writeln!(out, "    public required {} {} {{ get; init; }}", p.ty.spell(), p.name);
            }
        } else if p.loc == Loc::WholeBody {
            whole_body_prop(p, &mut out);
        } else {
            raw_prop(p, &mut out);
        }
    }
    let raw_args = |kind: &str| {
        let mut v = vec![
            format!("{kind}<string, JsonElement> rawHeaderData"),
            format!("{kind}<string, JsonElement> rawQueryData"),
        ];
        if has_body {
            v.push(format!("{kind}<string, JsonElement> rawBodyData"));
        } else if whole.is_some() {
            v.push("JsonElement rawBodyData".to_owned());
        }
        v
    };
    let raw_assigns: String = {
        let mut s = "this._rawHeaderData = new(rawHeaderData);\nthis._rawQueryData = new(rawQueryData);\n".to_owned();
        if has_body {
            s.push_str("this._rawBodyData = new(rawBodyData);\n");
        } else if whole.is_some() {
            s.push_str("this.RawBodyData = rawBodyData;\n");
        }
        s
    };
    let path_args: Vec<String> = path.iter().map(|p| crate::naming::local(local_of(&p.name))).collect();
    let _ = writeln!(out, "{}    public {n}() {{ }}", if first { "" } else { "\n" });
    let mut copies: String = path
        .iter()
        .map(|p| format!("this.{0} = {local}.{0};\n", p.name))
        .collect::<Vec<_>>()
        .concat();
    // The body's copy follows the path params' after a blank line.
    let body_copy = if has_body {
        Some(format!("this._rawBodyData = new({local}._rawBodyData);"))
    } else {
        whole.map(|_| format!("this.RawBodyData = {local}.RawBodyData;"))
    };
    if let Some(line) = body_copy {
        if !copies.is_empty() {
            copies.push('\n');
        }
        let _ = writeln!(copies, "{line}");
    }
    if copies.is_empty() {
        let _ = write!(
            out,
            "\n#pragma warning disable CS8618\n    [SetsRequiredMembers]\n    public {n}({n} {local})\n        : base({local}) {{ }}\n#pragma warning restore CS8618\n"
        );
    } else {
        let _ = write!(
            out,
            "\n#pragma warning disable CS8618\n    [SetsRequiredMembers]\n    public {n}({n} {local})\n        : base({local})\n    {{\n{copies}    }}\n#pragma warning restore CS8618\n"
        );
    }
    let _ = write!(
        out,
        "\n    public {n}(\n{args}\n    )\n    {{\n{raw_assigns}    }}\n",
        args = raw_args("IReadOnlyDictionary").join(",\n")
    );
    let mut frozen_args = raw_args("FrozenDictionary");
    let mut ro_args = raw_args("IReadOnlyDictionary");
    for (p, a) in path.iter().zip(&path_args) {
        frozen_args.push(format!("{} {a}", p.ty.spell()));
        ro_args.push(format!("{} {a}", p.ty.spell()));
    }
    let path_sets: String = path
        .iter()
        .zip(&path_args)
        .map(|(p, a)| format!("this.{} = {a};\n", p.name))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "\n#pragma warning disable CS8618\n    [SetsRequiredMembers]\n    {n}(\n{args}\n    )\n    {{\n{raw_assigns}{path_sets}    }}\n#pragma warning restore CS8618\n",
        args = frozen_args.join(",\n")
    );
    let mut conv = vec![
        "FrozenDictionary.ToFrozenDictionary(rawHeaderData)".to_owned(),
        "FrozenDictionary.ToFrozenDictionary(rawQueryData)".to_owned(),
    ];
    if has_body {
        conv.push("FrozenDictionary.ToFrozenDictionary(rawBodyData)".to_owned());
    } else if whole.is_some() {
        conv.push("rawBodyData".to_owned());
    }
    conv.extend(path_args.iter().cloned());
    let _ = write!(
        out,
        "\n    /// <inheritdoc cref=\"IFromRawJson{{T}}.FromRawUnchecked\"/>\n    public static {n} FromRawUnchecked(\n{args}\n    )\n    {{\n        return new(\n{conv}\n        );\n    }}\n",
        args = ro_args.join(",\n"),
        conv = conv.join(",\n")
    );
    let mut entries: String = path
        .iter()
        .map(|p| format!("[\"{0}\"] = JsonSerializer.SerializeToElement(this.{0}),\n", p.name))
        .collect::<Vec<_>>()
        .concat();
    entries.push_str("[\"HeaderData\"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawHeaderData.Freeze())),\n[\"QueryData\"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawQueryData.Freeze())),\n");
    if has_body {
        entries.push_str("[\"BodyData\"] = FriendlyJsonPrinter.PrintValue(this._rawBodyData.Freeze()),\n");
    } else if whole.is_some() {
        entries.push_str("[\"BodyData\"] = FriendlyJsonPrinter.PrintValue(this.RawBodyData),\n");
    }
    let _ = write!(
        out,
        "\n    public override string ToString() =>\n        JsonSerializer.Serialize(\n            FriendlyJsonPrinter.PrintValue(\n                new Dictionary<string, JsonElement>()\n                {{\n{entries}                }}\n            ),\n            ModelBase.ToStringSerializerOptions\n        );\n"
    );
    let mut eq: Vec<String> = path
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i + 1 == path.len() {
                format!("(this.{0}?.Equals(other.{0}) ?? other.{0} == null)", p.name)
            } else {
                format!("this.{0}.Equals(other.{0})", p.name)
            }
        })
        .collect();
    eq.push("this._rawHeaderData.Equals(other._rawHeaderData)".to_owned());
    eq.push("this._rawQueryData.Equals(other._rawQueryData)".to_owned());
    if has_body {
        eq.push("this._rawBodyData.Equals(other._rawBodyData)".to_owned());
    } else if whole.is_some() {
        eq.push("this.RawBodyData.Equals(other.RawBodyData)".to_owned());
    }
    let _ = write!(
        out,
        "\n    public virtual bool Equals({n}? other)\n    {{\n        if (other == null)\n        {{\n            return false;\n        }}\n        return {};\n    }}\n",
        eq.join("\n            && ")
    );
    let url = url_expr(&m.path, &path);
    let _ = write!(
        out,
        "\n    public override {uri} Url(ClientOptions options)\n    {{\n        return new {builder}({url})\n        {{\n            Query = this.QueryString(options),\n        }}.Uri;\n    }}\n",
        uri = ctx.system("Uri"),
        builder = ctx.system("UriBuilder"),
    );
    if any_body {
        out.push_str("\n    internal override HttpContent? BodyContent()\n    {\n        return new StringContent(\n            JsonSerializer.Serialize(this.RawBodyData, ModelBase.SerializerOptions),\n            Text::Encoding.UTF8,\n            \"application/json\"\n        );\n    }\n");
    }
    out.push_str("\n    internal override void AddHeadersToRequest(HttpRequestMessage request, ClientOptions options)\n    {\n        ParamsBase.AddDefaultHeaders(request, options);\n        foreach (var item in this.RawHeaderData)\n        {\n            ParamsBase.AddHeaderElementToRequest(request, item.Key, item.Value);\n        }\n    }\n\n    public override int GetHashCode()\n    {\n        return 0;\n    }\n}\n");
    out
}

/// The property of a whole body: read from and written to `RawBodyData`.
fn whole_body_prop(p: &Prop, out: &mut String) {
    prop_doc(p, out);
    let ty = p.ty.spell();
    let (req, q, get) = if p.required {
        ("required ", "", "GetNotNullClass")
    } else {
        ("", "?", "GetNullableClass")
    };
    let _ = write!(
        out,
        "    public {req}{ty}{q} {name}\n    {{\n        get\n        {{\n            return WrappedJsonSerializer.{get}<{ty}>(this.RawBodyData, \"RawBodyData\");\n        }}\n        init {{ this.RawBodyData = JsonSerializer.SerializeToElement(value); }}\n    }}\n",
        name = p.name
    );
}

/// `options.BaseUrl.ToString().TrimEnd('/') + "/v1/files/{0}"` with the path params formatted in.
fn url_expr(path: &str, params: &[&Prop]) -> String {
    let base = "options.BaseUrl.ToString().TrimEnd('/')";
    if params.is_empty() {
        return format!("{base} + \"{path}\"");
    }
    let mut template = String::new();
    let mut rest = path;
    let mut i = 0;
    while let Some(start) = rest.find('{') {
        template.push_str(&rest[..start]);
        let end = rest[start..].find('}').map_or(rest.len(), |e| start + e);
        let _ = write!(template, "{{{i}}}");
        i += 1;
        rest = &rest[(end + 1).min(rest.len())..];
    }
    template.push_str(rest);
    let args: String = params
        .iter()
        .map(|p| format!(", this.{}", p.name))
        .collect::<Vec<_>>()
        .concat();
    format!("{base} + string.Format(\"{template}\"{args})")
}

/// An enum and its JSON converter.
fn enum_decl(ctx: &FileCtx<'_>, e: &En, place: doc::Place, used: &mut Vec<String>) -> String {
    used.extend(["System.Text.Json", "System.Text.Json.Serialization"].map(str::to_owned));
    used.push(exceptions(ctx));
    let n = &e.name;
    let q = ctx.qualified(n);
    let raw = e.value.raw();
    let mut out = String::new();
    out.push_str(&doc::summary(&e.description, 0, place, &[]));
    let members: String = e
        .members
        .iter()
        .map(|(m, _)| format!("    {m},\n"))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "[JsonConverter(typeof({n}Converter))]\npublic enum {n}\n{{\n{members}}}\n"
    );
    let reads: String = e
        .members
        .iter()
        .map(|(m, v)| format!("{} => {q}.{m},\n", literal(v)))
        .collect::<Vec<_>>()
        .concat();
    let writes: String = e
        .members
        .iter()
        .map(|(m, v)| format!("{q}.{m} => {},\n", literal(v)))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "\nsealed class {n}Converter : JsonConverter<{q}>\n{{\n    public override {q} Read(\n        ref Utf8JsonReader reader,\n        System::Type typeToConvert,\n        JsonSerializerOptions options\n    )\n    {{\n        return JsonSerializer.Deserialize<{raw}>(ref reader, options) switch\n        {{\n{reads}            _ => ({q})(-1),\n        }};\n    }}\n\n    public override void Write(Utf8JsonWriter writer, {q} value, JsonSerializerOptions options)\n    {{\n        JsonSerializer.Serialize(\n            writer,\n            value switch\n            {{\n{writes}                _ => throw new {c}InvalidDataException(\n                    string.Format(\"Invalid value '{{0}}' in {{1}}\", value, nameof(value))\n                ),\n            }},\n            options\n        );\n    }}\n}}\n",
        c = ctx.g.target.client
    );
    out
}

/// A union of its variants, with its JSON converter.
fn union_decl(ctx: &FileCtx<'_>, u: &Un, place: doc::Place, used: &mut Vec<String>) -> String {
    super::model_union::union_decl(ctx, u, place, used)
}
