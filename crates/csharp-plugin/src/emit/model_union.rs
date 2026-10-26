//! Unions: a `ModelBase` record holding one variant, with `TryPick`, `Switch`, `Match`, implicit
//! conversions, validation and a JSON converter that picks the variant by discriminator or by
//! trying each in turn.

use std::fmt::Write as _;

use crate::doc;
use crate::emit::model::FileCtx;
use crate::naming::{camel_of, local, split_words};
use crate::tree::{UVariant, Un};
use crate::types::CsType;

fn param(v: &UVariant) -> String {
    local(camel_of(&v.label))
}

/// `double` for a value type, `File` for a class.
fn spell(ctx: &FileCtx<'_>, v: &UVariant) -> String {
    match &v.ty {
        CsType::Enum { name, value, .. } => format!("ApiEnum<{}, {}>", value.raw(), ctx.qualified(name)),
        t => t.spell(),
    }
}

/// The variant's type in a doc comment, escaped.
fn spell_doc(ctx: &FileCtx<'_>, v: &UVariant) -> String {
    crate::doc::escape(&spell(ctx, v))
}

/// What `TryPick`'s summary says the variant's type is: a `<see cref>` (generic types by their
/// type parameters, the argument named after).
fn see(ctx: &FileCtx<'_>, v: &UVariant) -> String {
    match &v.ty {
        CsType::List(t) => format!(
            "<see cref=\"List{{T}}\"/> where <c>T</c> is a <c>{}</c>",
            crate::doc::escape(&t.spell())
        ),
        CsType::Dict(t) => format!(
            "<see cref=\"Dictionary{{TKey, TValue}}\"/> where <c>TValue</c> is a <c>{}</c>",
            crate::doc::escape(&t.spell())
        ),
        CsType::Enum { name, .. } => format!(
            "<see cref=\"ApiEnum{{TRaw, TEnum}}\"/> where <c>TEnum</c> is a <c>{}</c>",
            ctx.qualified(name)
        ),
        _ => format!("<see cref=\"{}\"/>", spell(ctx, v)),
    }
}

/// The implicit conversion to the union from one variant: a list converts from `List<T>` and
/// a map from `Dictionary`, as C# allows no conversion from an interface.
fn conversion(ctx: &FileCtx<'_>, n: &str, v: &UVariant) -> String {
    match &v.ty {
        CsType::List(t) => {
            let t = t.spell();
            format!(
                "\n    public static implicit operator {n}(List<{t}> value) =>\n        new((IReadOnlyList<{t}>)value);\n"
            )
        }
        CsType::Dict(t) => {
            let t = t.spell();
            format!(
                "\n    public static implicit operator {n}(Dictionary<string, {t}> value) =>\n        new((IReadOnlyDictionary<string, {t}>)value);\n"
            )
        }
        _ => format!(
            "\n    public static implicit operator {n}({} value) => new(value);\n",
            spell(ctx, v)
        ),
    }
}

/// A property the union exposes: one at least two object variants have with the same type.
struct Shared {
    name: String,
    /// As the union spells it: nullable unless every variant has it, required.
    ty: String,
    /// Whether the type is not nullable (`Match` then needs no type argument).
    plain: bool,
    /// Per variant, whether it has the property.
    has: Vec<bool>,
}

/// The properties at least two object variants share, in the order the variants list them.
fn shared_props(u: &Un) -> Vec<Shared> {
    if u.variants.iter().any(|v| v.props.is_empty()) {
        return Vec::new();
    }
    let mut out: Vec<Shared> = Vec::new();
    for v in &u.variants {
        for p in &v.props {
            // A map's type differs between a named and an inline schema; Stainless does not share it.
            // `Value` is the union's own (a named variant keeps the name in its file).
            if matches!(p.ty, crate::types::CsType::Dict(_))
                || p.name == "Value"
                || out.iter().any(|s| s.name == p.name)
            {
                continue;
            }
            let same = |q: &crate::tree::Prop| q.name == p.name && q.ty.spell() == p.ty.spell();
            let has: Vec<bool> = u.variants.iter().map(|w| w.props.iter().any(same)).collect();
            // A name with another type in some variant is not shared at all.
            let clash = u
                .variants
                .iter()
                .any(|w| w.props.iter().any(|q| q.name == p.name && q.ty.spell() != p.ty.spell()));
            if clash || has.iter().filter(|h| **h).count() < 2 {
                continue;
            }
            let everywhere = has.iter().all(|h| *h)
                && u.variants.iter().all(|w| {
                    w.props
                        .iter()
                        .filter(|q| same(q))
                        .all(|q| (q.required && !q.nullable) || q.constant.is_some())
                });
            let ty = p.ty.spell();
            out.push(Shared {
                name: p.name.clone(),
                ty: if everywhere { ty } else { format!("{ty}?") },
                plain: everywhere,
                has,
            });
        }
    }
    out
}

/// The union's class and converter.
// One straight-line template of the generated C#; splitting it would scatter the file's
// layout across helpers.
#[allow(clippy::too_many_lines)]
pub fn union_decl(ctx: &FileCtx<'_>, u: &Un, place: doc::Place, used: &mut Vec<String>) -> String {
    used.extend(
        [
            "System.Diagnostics.CodeAnalysis",
            "System.Text.Json",
            "System.Text.Json.Serialization",
        ]
        .map(str::to_owned),
    );
    used.push(format!("{}.Core", ctx.g.target.ns));
    used.push(format!("{}.Exceptions", ctx.g.target.ns));
    for v in &u.variants {
        if v.ty.uses_list() {
            used.push("System.Collections.Immutable".to_owned());
            used.push("System.Collections.Generic".to_owned());
        }
    }
    let n = &u.name;
    let exc = format!("{}InvalidDataException", ctx.g.target.client);
    let no_match = format!("\"Data did not match any variant of {n}\"");
    let mut out = String::new();
    out.push_str(&doc::summary(&u.description, 0, place, &[]));
    let _ = write!(
        out,
        "[JsonConverter(typeof({n}Converter))]\npublic record class {n} : ModelBase\n{{\n    public object? Value {{ get; }} = null;\n\n    JsonElement? _element = null;\n\n    public JsonElement Json\n    {{\n        get\n        {{\n            return this._element ??= JsonSerializer.SerializeToElement(\n                this.Value,\n                ModelBase.SerializerOptions\n            );\n        }}\n    }}\n"
    );
    for s in shared_props(u) {
        let arms: Vec<String> = u
            .variants
            .iter()
            .zip(&s.has)
            .map(|(v, has)| {
                if *has {
                    format!("{}: (x) => x.{}", param(v), s.name)
                } else {
                    format!("{}: (_) => null", param(v))
                }
            })
            .collect();
        let targ = if s.plain { String::new() } else { format!("<{}>", s.ty) };
        let _ = write!(
            out,
            "\n    public {} {}\n    {{\n        get {{ return Match{targ}({}); }}\n    }}\n",
            s.ty,
            s.name,
            arms.join(", ")
        );
    }
    for v in &u.variants {
        // A list variant is stored immutable.
        let value = if matches!(v.ty, CsType::List(_)) {
            "ImmutableArray.ToImmutableArray(value)"
        } else {
            "value"
        };
        let _ = write!(
            out,
            "\n    public {n}({} value, JsonElement? element = null)\n    {{\n        this.Value = {value};\n        this._element = element;\n    }}\n",
            spell(ctx, v)
        );
    }
    let _ = write!(
        out,
        "\n    public {n}(JsonElement element)\n    {{\n        this._element = element;\n    }}\n"
    );
    for v in &u.variants {
        let ty = spell(ctx, v);
        // A list is stored as an `ImmutableArray` but held as an `IReadOnlyList`, a reference.
        let value_type = v.ty.is_struct() && !matches!(v.ty, CsType::List(_));
        let label = &v.label;
        let _ = write!(
            out,
            "\n    /// <summary>\n    /// Returns true and sets the <c>out</c> parameter if the instance was constructed with a variant of\n    /// type {see}.\n    ///\n    /// <para>Consider using <see cref=\"Switch\"/> or <see cref=\"Match\"/> if you need to handle every variant.</para>\n    ///\n    /// <example>\n    /// <code>\n    /// if (instance.TryPick{label}(out var value)) {{\n    ///     // `value` is of type `{ty_doc}`\n    ///     Console.WriteLine(value);\n    /// }}\n    /// </code>\n    /// </example>\n    /// </summary>\n    public bool TryPick{label}([NotNullWhen(true)] out {ty}? value)\n    {{\n        value = this.Value as {ty}{q};\n        return value != null;\n    }}\n",
            see = see(ctx, v),
            ty_doc = spell_doc(ctx, v),
            q = if value_type { "?" } else { "" },
        );
    }
    let examples: String = u
        .variants
        .iter()
        .map(|v| format!("    ///     ({} value) =&gt; {{...}}", spell_doc(ctx, v)))
        .collect::<Vec<_>>()
        .join(",\n");
    let actions: Vec<String> = u
        .variants
        .iter()
        .map(|v| format!("{}<{}> {}", ctx.system("Action"), spell(ctx, v), param(v)))
        .collect();
    let cases: String = u
        .variants
        .iter()
        .map(|v| format!("case {} value:\n{}(value);\nbreak;\n", spell(ctx, v), param(v)))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "\n    /// <summary>\n    /// Calls the function parameter corresponding to the variant the instance was constructed with.\n    ///\n    /// <para>Use the <c>TryPick</c> method(s) if you don't need to handle every variant, or <see cref=\"Match\"/>\n    /// if you need your function parameters to return something.</para>\n    ///\n    /// <exception cref=\"{exc}\">\n    /// Thrown when the instance was constructed with an unknown variant (e.g. deserialized from raw data\n    /// that doesn't match any variant's expected shape).\n    /// </exception>\n    ///\n    /// <example>\n    /// <code>\n    /// instance.Switch(\n{examples}\n    /// );\n    /// </code>\n    /// </example>\n    /// </summary>\n    public void Switch({})\n    {{\n        switch (this.Value)\n        {{\n{cases}default:\nthrow new {exc}(\n{no_match}\n);\n        }}\n    }}\n",
        actions.join(", ")
    );
    let funcs: Vec<String> = u
        .variants
        .iter()
        .map(|v| format!("{}<{}, T> {}", ctx.system("Func"), spell(ctx, v), param(v)))
        .collect();
    let arms: String = u
        .variants
        .iter()
        .map(|v| format!("{} value => {}(value),\n", spell(ctx, v), param(v)))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "\n    /// <summary>\n    /// Calls the function parameter corresponding to the variant the instance was constructed with and\n    /// returns its result.\n    ///\n    /// <para>Use the <c>TryPick</c> method(s) if you don't need to handle every variant, or <see cref=\"Switch\"/>\n    /// if you don't need your function parameters to return a value.</para>\n    ///\n    /// <exception cref=\"{exc}\">\n    /// Thrown when the instance was constructed with an unknown variant (e.g. deserialized from raw data\n    /// that doesn't match any variant's expected shape).\n    /// </exception>\n    ///\n    /// <example>\n    /// <code>\n    /// var result = instance.Match(\n{examples}\n    /// );\n    /// </code>\n    /// </example>\n    /// </summary>\n    public T Match<T>({})\n    {{\n        return this.Value switch\n        {{\n{arms}_ => throw new {exc}(\n{no_match}\n),\n        }};\n    }}\n",
        funcs.join(", ")
    );
    for v in &u.variants {
        out.push_str(&conversion(ctx, n, v));
    }
    let validating: Vec<&UVariant> = u.variants.iter().filter(|v| v.ty.validates()).collect();
    let switch = if validating.is_empty() {
        String::new()
    } else {
        let lambdas: Vec<String> = u
            .variants
            .iter()
            .map(|v| {
                if v.ty.validates() {
                    let p = param(v);
                    format!("({p}) => {p}.Validate()")
                } else {
                    "(_) => { }".to_owned()
                }
            })
            .collect();
        format!("this.Switch({});\n", lambdas.join(", "))
    };
    let _ = write!(
        out,
        "\n    /// <summary>\n    /// Validates that the instance was constructed with a known variant and that this variant is valid\n    /// (based on its own <c>Validate</c> method).\n    ///\n    /// <para>This is useful for instances constructed from raw JSON data (e.g. deserialized from an API response).</para>\n    ///\n    /// <exception cref=\"{exc}\">\n    /// Thrown when the instance does not pass validation.\n    /// </exception>\n    /// </summary>\n    public override void Validate()\n    {{\n        if (this.Value == null)\n        {{\n            throw new {exc}(\n                {no_match}\n            );\n        }}\n{switch}    }}\n"
    );
    let indexes: String = u
        .variants
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{} _ => {i},\n", spell(ctx, v)))
        .collect::<Vec<_>>()
        .concat();
    let _ = write!(
        out,
        "\n    public virtual bool Equals({n}? other) =>\n        other != null\n        && this.VariantIndex() == other.VariantIndex()\n        && JsonElement.DeepEquals(this.Json, other.Json);\n\n    public override int GetHashCode()\n    {{\n        return 0;\n    }}\n\n    public override string ToString() =>\n        JsonSerializer.Serialize(\n            FriendlyJsonPrinter.PrintValue(this.Json),\n            ModelBase.ToStringSerializerOptions\n        );\n\n    int VariantIndex()\n    {{\n        return this.Value switch\n        {{\n{indexes}_ => -1,\n        }};\n    }}\n}}\n"
    );
    out.push_str(&converter(ctx, u));
    out
}

fn try_variant(ctx: &FileCtx<'_>, v: &UVariant, catch_all: bool) -> String {
    let ty = spell(ctx, v);
    let exc = format!("{}InvalidDataException", ctx.g.target.client);
    let catch = if catch_all {
        format!(
            "catch ({sys} e) when (e is JsonException || e is {exc})",
            sys = ctx.system("Exception")
        )
    } else {
        "catch (JsonException)".to_owned()
    };
    let body = if let CsType::List(item) = &v.ty {
        let validate = if item.validates() && catch_all {
            "foreach (var item in deserialized)\n{\nitem.Validate();\n}\n"
        } else {
            ""
        };
        format!(
            "var deserialized = JsonSerializer.Deserialize<List<{}>>(element, options);\nif (deserialized != null)\n{{\n{validate}return new(deserialized, element);\n}}\n",
            item.spell()
        )
    } else if v.ty.is_struct() {
        format!("return new(JsonSerializer.Deserialize<{ty}>(element, options), element);\n")
    } else if v.ty.validates() && catch_all {
        format!(
            "var deserialized = JsonSerializer.Deserialize<{ty}>(element, options);\nif (deserialized != null)\n{{\ndeserialized.Validate();\nreturn new(deserialized, element);\n}}\n"
        )
    } else {
        format!(
            "var deserialized = JsonSerializer.Deserialize<{ty}>(element, options);\nif (deserialized != null)\n{{\nreturn new(deserialized, element);\n}}\n"
        )
    };
    format!("try\n{{\n{body}}}\n{catch}\n{{\n// ignore\n}}\n")
}

fn converter(ctx: &FileCtx<'_>, u: &Un) -> String {
    let n = &u.name;
    let mut out = format!(
        "\nsealed class {n}Converter : JsonConverter<{n}>\n{{\n    public override {n}? Read(\n        ref Utf8JsonReader reader,\n        System::Type typeToConvert,\n        JsonSerializerOptions options\n    )\n    {{\n        var element = JsonSerializer.Deserialize<JsonElement>(ref reader, options);\n"
    );
    if let Some(d) = &u.discriminator {
        let var = local(camel_of(&split_words(d).join(" ").replace(' ', "_")));
        let _ = write!(
            out,
            "string? {var};\ntry\n{{\n{var} = element.GetProperty(\"{d}\").GetString();\n}}\ncatch\n{{\n{var} = null;\n}}\n\nswitch ({var})\n{{\n"
        );
        for v in &u.variants {
            for value in &v.discriminator {
                let _ = write!(
                    out,
                    "case \"{value}\":\n{{\n{}\nreturn new(element);\n}}\n",
                    try_variant(ctx, v, false)
                );
            }
        }
        // A variant without a discriminator value of its own is the fallback.
        match u.variants.iter().find(|v| v.discriminator.is_empty()) {
            Some(v) => {
                let _ = write!(
                    out,
                    "default:\n{{\n{}\nreturn new(element);\n}}\n}}\n",
                    try_variant(ctx, v, false)
                );
            }
            None => {
                let _ = write!(out, "default:\n{{\nreturn new {n}(element);\n}}\n}}\n");
            }
        }
    } else {
        for (i, v) in u.variants.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&try_variant(ctx, v, true));
        }
        out.push_str("\nreturn new(element);\n");
    }
    let _ = write!(
        out,
        "    }}\n\n    public override void Write(\n        Utf8JsonWriter writer,\n        {n} value,\n        JsonSerializerOptions options\n    )\n    {{\n        JsonSerializer.Serialize(writer, value.Json, options);\n    }}\n}}\n"
    );
    out
}
