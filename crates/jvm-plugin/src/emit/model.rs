//! Model classes: an object with its builder, validation and Jackson wiring, and an enum class
//! (`Known`/`Value`), in Stainless's member order. One pass writes both flavors.

use std::fmt::Write as _;

use crate::emit::Gen;
use crate::kt::{Imports, doc, string};
use crate::naming::{camel, capitalize, constant, decapitalize, singular, split_words};
use crate::types::{Class, ClassKind, Jt, Prop, Shape, list_shadowed};

/// The `KDoc` line every value getter throws with.
fn throws_line(g: &Gen<'_>, missing: bool) -> String {
    if missing {
        format!(
            "@throws {}InvalidDataException if the JSON field has an unexpected type or is unexpectedly missing or null (e.g. if the server responded with an unexpected value).",
            g.target.client
        )
    } else {
        format!(
            "@throws {}InvalidDataException if the JSON field has an unexpected type (e.g. if the server responded with an unexpected value).",
            g.target.client
        )
    }
}

/// The required-fields block of a builder's docs, or empty.
#[must_use]
pub fn required_block(g: &Gen<'_>, names: &[String]) -> String {
    if names.is_empty() {
        return String::new();
    }
    let lang = if g.java() { "java" } else { "kotlin" };
    let mut out = format!("The following fields are required:\n```{lang}\n");
    for n in names {
        let _ = writeln!(out, ".{n}()");
    }
    out.push_str("```");
    out
}

/// Writes a class and its nested classes. `chain` holds the enclosing classes, outermost first.
pub fn class(g: &Gen<'_>, c: &Class<'_>, chain: &[&Class<'_>], imp: &mut Imports) -> String {
    match &c.shape {
        Shape::Object { .. } => object(g, c, chain, imp),
        Shape::Enum { .. } => enumeration(g, c, imp),
    }
}

/// `JsonField<T>` of a prop, as declared.
fn field_ty(p: &Prop<'_>, qualify: bool) -> String {
    if p.ty == Jt::Value {
        "JsonValue".to_owned()
    } else {
        format!("JsonField<{}>", p.ty.kt(qualify))
    }
}

/// The text of `n` items joined by `sep`.
fn join<I: IntoIterator<Item = String>>(items: I, sep: &str) -> String {
    items.into_iter().collect::<Vec<_>>().join(sep)
}

// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
fn object(g: &Gen<'_>, c: &Class<'_>, chain: &[&Class<'_>], imp: &mut Imports) -> String {
    let Shape::Object { props, bases } = &c.shape else {
        unreachable!()
    };
    let mut scope: Vec<&Class<'_>> = chain.to_vec();
    scope.push(c);
    let qualify = list_shadowed(&scope);
    let java = g.java();
    let name = &c.name;
    let err = format!("{}InvalidDataException", g.target.client);
    imp.add("com.fasterxml.jackson.annotation.JsonAnyGetter");
    imp.add("com.fasterxml.jackson.annotation.JsonAnySetter");
    imp.add("com.fasterxml.jackson.annotation.JsonCreator");
    imp.add("com.fasterxml.jackson.annotation.JsonProperty");
    imp.add("java.util.Collections");
    imp.add("java.util.Objects");
    for n in ["ExcludeMissing", "JsonField", "JsonMissing", "JsonValue"] {
        imp.core(n);
    }
    imp.sdk(&format!("errors.{err}"));
    if java {
        imp.add("java.util.Optional");
        imp.add("kotlin.jvm.optionals.getOrNull");
    }
    for p in props {
        add_type_imports(g, &p.ty, imp);
        if p.required() {
            imp.core("checkRequired");
        }
        if p.ty.items().is_some() {
            imp.core("checkKnown");
            imp.core("toImmutable");
        }
    }

    let mut out = String::new();
    out.push_str(&doc(&c.doc));
    let _ = writeln!(out, "class {name}");
    out.push_str("@JsonCreator(mode = JsonCreator.Mode.DISABLED)\nprivate constructor(\n");
    for p in props {
        let _ = writeln!(out, "private val {}: {},", p.name, field_ty(p, qualify));
    }
    out.push_str("private val additionalProperties: MutableMap<String, JsonValue>,\n) {\n\n");

    // The Jackson constructor.
    out.push_str("@JsonCreator\nprivate constructor(\n");
    for p in props {
        let _ = writeln!(
            out,
            "@JsonProperty({}) @ExcludeMissing {}: {} = JsonMissing.of(),",
            string(p.wire()),
            p.name,
            field_ty(p, qualify)
        );
    }
    let _ = writeln!(
        out,
        ") : this({}, mutableMapOf())\n",
        join(props.iter().map(|p| p.name.clone()), ", ")
    );

    // Conversions to the models this one extends.
    for base in bases {
        let Some(decl) = g.client.model(base) else { continue };
        let Some(bo) = g.client.object_of(&decl.ty) else {
            continue;
        };
        let bname = g.mapper.model_name(base);
        imp.add(g.mapper.model_path(base));
        let _ = write!(out, "fun to{bname}(): {bname} = {bname}.builder()");
        for f in g.mapper.all_fields(bo) {
            let n = camel(&crate::naming::words(&f.name), g.casings);
            let _ = write!(out, ".{n}({n})");
        }
        out.push_str(".build()\n\n");
    }

    // Value getters.
    for p in props {
        if p.ty == Jt::Value {
            continue;
        }
        let mut text = p.field.description.trim().to_owned();
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        let missing = p.required() && !p.nullable;
        text.push_str(&throws_line(g, missing));
        out.push_str(&doc(&text));
        let wire = string(p.wire());
        let t = p.ty.kt(qualify);
        if missing {
            let _ = writeln!(out, "fun {}(): {t} = {}.getRequired({wire})\n", p.name, p.name);
        } else if java {
            let _ = writeln!(
                out,
                "fun {}(): Optional<{t}> = {}.getOptional({wire})\n",
                p.name, p.name
            );
        } else {
            let _ = writeln!(out, "fun {}(): {t}? = {}.getNullable({wire})\n", p.name, p.name);
        }
    }

    // Raw getters.
    for p in props {
        let n = &p.name;
        if p.ty == Jt::Value {
            out.push_str(&doc(&p.field.description));
            let _ = writeln!(
                out,
                "@JsonProperty({}) @ExcludeMissing fun _{n}(): JsonValue = {n}\n",
                string(p.wire())
            );
            continue;
        }
        out.push_str(&doc(&format!(
            "Returns the raw JSON value of [{n}].\n\nUnlike [{n}], this method doesn't throw if the JSON field has an unexpected type."
        )));
        let _ = writeln!(
            out,
            "@JsonProperty({}) @ExcludeMissing fun _{n}(): {} = {n}\n",
            string(p.wire()),
            field_ty(p, qualify)
        );
    }

    out.push_str(
        "@JsonAnySetter\nprivate fun putAdditionalProperty(key: String, value: JsonValue) {\nadditionalProperties.put(key, value)\n}\n\n",
    );
    out.push_str(
        "@JsonAnyGetter\n@ExcludeMissing\nfun _additionalProperties(): Map<String, JsonValue> = Collections.unmodifiableMap(additionalProperties)\n\n",
    );
    out.push_str("fun toBuilder() = Builder().from(this)\n\n");

    let required: Vec<String> = props.iter().filter(|p| p.required()).map(|p| p.name.clone()).collect();
    let block = required_block(g, &required);
    let jvm_static = if java { "@JvmStatic " } else { "" };
    let jvm_synthetic = if java { "@JvmSynthetic\n" } else { "" };
    out.push_str("companion object {\n\n");
    let mut builder_doc = format!("Returns a mutable builder for constructing an instance of [{name}].");
    if !block.is_empty() {
        builder_doc.push_str("\n\n");
        builder_doc.push_str(&block);
    }
    out.push_str(&doc(&builder_doc));
    let _ = writeln!(out, "{jvm_static}fun builder() = Builder()\n}}\n");

    // The builder.
    let _ = writeln!(
        out,
        "/** A builder for [{name}]. */\nclass Builder internal constructor() {{\n"
    );
    for p in props {
        let n = &p.name;
        let decl = if let Some(items) = p.ty.items() {
            format!("JsonField<MutableList<{}>>? = null", items.kt(qualify))
        } else if p.ty == Jt::Value {
            "JsonValue = JsonMissing.of()".to_owned()
        } else if p.required() {
            format!("{}? = null", field_ty(p, qualify))
        } else {
            format!("{} = JsonMissing.of()", field_ty(p, qualify))
        };
        let _ = writeln!(out, "private var {n}: {decl}");
    }
    out.push_str("private var additionalProperties: MutableMap<String, JsonValue> = mutableMapOf()\n\n");
    let var = decapitalize(name);
    let _ = writeln!(out, "{jvm_synthetic}internal fun from({var}: {name}) = apply {{");
    for p in props {
        let n = &p.name;
        let lhs = if *n == var { format!("this.{n}") } else { n.clone() };
        if p.ty.items().is_some() {
            let _ = writeln!(out, "{lhs} = {var}.{n}.map {{ it.toMutableList() }}");
        } else {
            let _ = writeln!(out, "{lhs} = {var}.{n}");
        }
    }
    let _ = writeln!(
        out,
        "additionalProperties = {var}.additionalProperties.toMutableMap()\n}}\n"
    );

    for p in props {
        out.push_str(&setters(g, p, qualify));
    }

    out.push_str(&additional_properties_builder());

    let mut build_doc = format!(
        "Returns an immutable instance of [{name}].\n\nFurther updates to this [Builder] will not mutate the returned instance."
    );
    if !block.is_empty() {
        build_doc.push_str("\n\n");
        build_doc.push_str(&block);
        build_doc.push_str("\n\n@throws IllegalStateException if any required field is unset.");
    }
    out.push_str(&doc(&build_doc));
    let _ = write!(out, "fun build(): {name} = {name}(");
    for p in props {
        let n = &p.name;
        let arg = match (p.required(), p.ty.items().is_some()) {
            (true, true) => format!("checkRequired({}, {n}).map {{ it.toImmutable() }}", string(n)),
            (true, false) if p.ty != Jt::Value => format!("checkRequired({}, {n})", string(n)),
            (false, true) => format!("({n} ?: JsonMissing.of()).map {{ it.toImmutable() }}"),
            _ => n.clone(),
        };
        let _ = write!(out, "{arg}, ");
    }
    out.push_str("additionalProperties.toMutableMap(),)\n}\n\n");

    // Validation.
    out.push_str("private var validated: Boolean = false\n\n");
    out.push_str(&doc(&format!(
        "Validates that the types of all values in this object match their expected types recursively.\n\nThis method is _not_ forwards compatible with new types from the API for existing fields.\n\n@throws {err} if any value type in this object doesn't match its expected type."
    )));
    let _ = writeln!(
        out,
        "fun validate(): {name} = apply {{\nif (validated) {{\nreturn@apply\n}}\n"
    );
    for p in props {
        if let Some(line) = validate_line(g, p) {
            let _ = writeln!(out, "{line}");
        }
    }
    out.push_str("validated = true\n}\n\n");
    let _ = writeln!(
        out,
        "fun isValid(): Boolean =\ntry {{\nvalidate()\ntrue\n}} catch (e: {err}) {{\nfalse\n}}\n"
    );
    out.push_str(&doc(
        "Returns a score indicating how many valid values are contained in this object recursively.\n\nUsed for best match union deserialization.",
    ));
    let validity = if props.is_empty() {
        "0".to_owned()
    } else {
        join(props.iter().map(|p| validity_term(g, p)), " + ")
    };
    let _ = writeln!(out, "{jvm_synthetic}internal fun validity(): Int = {validity}\n");

    for n in &c.nested {
        out.push_str(&class(g, n, &scope, imp));
        out.push('\n');
    }

    // Equality and printing.
    let _ = write!(
        out,
        "override fun equals(other: Any?): Boolean {{\nif (this === other) {{\nreturn true\n}}\n\nreturn other is {name} && "
    );
    for p in props {
        let _ = write!(out, "{n} == other.{n} && ", n = p.name);
    }
    out.push_str("additionalProperties == other.additionalProperties\n}\n\n");
    let mut hashed: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
    hashed.push("additionalProperties".to_owned());
    let _ = writeln!(
        out,
        "private val hashCode: Int by lazy {{ Objects.hash({}) }}\n",
        join(hashed, ", ")
    );
    out.push_str("override fun hashCode(): Int = hashCode\n\n");
    let mut shown: Vec<String> = props.iter().map(|p| format!("{n}=${n}", n = p.name)).collect();
    shown.push("additionalProperties=$additionalProperties".to_owned());
    let _ = writeln!(
        out,
        "override fun toString() =\n\"{name}{{{}}}\"\n}}",
        join(shown, ", ")
    );
    out
}

fn add_type_imports(g: &Gen<'_>, ty: &Jt, imp: &mut Imports) {
    let mut paths = Vec::new();
    ty.imports(&mut paths);
    for p in paths {
        if let Some(rel) = p.strip_prefix('.') {
            imp.sdk(rel);
        } else {
            imp.add(p);
        }
    }
    if ty.class_kind() == Some(ClassKind::Enum) {
        let _ = g;
    }
}

/// The setters of one prop in a model builder.
fn setters(g: &Gen<'_>, p: &Prop<'_>, qualify: bool) -> String {
    let java = g.java();
    let n = &p.name;
    let mut out = String::new();
    let desc = p.field.description.trim();
    if p.ty == Jt::Value {
        out.push_str(&doc(desc));
        let _ = writeln!(out, "fun {n}({n}: JsonValue) = apply {{ this.{n} = {n} }}\n");
        return out;
    }
    let t = p.ty.kt(qualify);
    out.push_str(&doc(desc));
    if p.nullable {
        let _ = writeln!(out, "fun {n}({n}: {t}?) = {n}(JsonField.ofNullable({n}))\n");
        if p.ty.is_primitive() {
            out.push_str(&doc(&format!(
                "Alias for [Builder.{n}].\n\nThis unboxed primitive overload exists for backwards compatibility."
            )));
            let _ = writeln!(out, "fun {n}({n}: {t}) = {n}({n} as {t}?)\n");
        }
        if java {
            out.push_str(&doc(&format!(
                "Alias for calling [Builder.{n}] with `{n}.orElse(null)`."
            )));
            let _ = writeln!(out, "fun {n}({n}: Optional<{t}>) = {n}({n}.getOrNull())\n");
        }
    } else {
        let _ = writeln!(out, "fun {n}({n}: {t}) = {n}(JsonField.of({n}))\n");
    }
    out.push_str(&doc(&format!(
        "Sets [Builder.{n}] to an arbitrary JSON value.\n\nYou should usually call [Builder.{n}] with a well-typed {} value instead. This method is primarily for setting the field to an undocumented or not yet supported value.",
        p.ty.doc()
    )));
    if p.ty.items().is_some() {
        let _ = writeln!(
            out,
            "fun {n}({n}: JsonField<{t}>) = apply {{ this.{n} = {n}.map {{ it.toMutableList() }} }}\n"
        );
    } else {
        let _ = writeln!(out, "fun {n}({n}: JsonField<{t}>) = apply {{ this.{n} = {n} }}\n");
    }
    if let Some(items) = p.ty.items() {
        let one_words = singular(&split_words(n));
        let one = camel(&one_words, g.casings);
        let target = if one == *n { format!("Builder.{n}") } else { n.clone() };
        out.push_str(&doc(&format!(
            "Adds a single {} to [{target}].\n\n@throws IllegalStateException if the field was previously set to a non-list.",
            items.doc()
        )));
        let it = items.kt(qualify);
        let field = if one == *n { format!("this.{n}") } else { n.clone() };
        let _ = writeln!(
            out,
            "fun add{}({one}: {it}) = apply {{\n{field} = ({field} ?: JsonField.of(mutableListOf())).also {{\ncheckKnown({}, it).add({one})\n}}\n}}\n",
            capitalize(&one),
            string(n)
        );
    }
    out
}

/// The `additionalProperties` methods every model builder ends with.
fn additional_properties_builder() -> String {
    "fun additionalProperties(additionalProperties: Map<String, JsonValue>) = apply {\nthis.additionalProperties.clear()\nputAllAdditionalProperties(additionalProperties)\n}\n\n\
fun putAdditionalProperty(key: String, value: JsonValue) = apply {\nadditionalProperties.put(key, value)\n}\n\n\
fun putAllAdditionalProperties(additionalProperties: Map<String, JsonValue>) = apply {\nthis.additionalProperties.putAll(additionalProperties)\n}\n\n\
fun removeAdditionalProperty(key: String) = apply { additionalProperties.remove(key) }\n\n\
fun removeAllAdditionalProperties(keys: Set<String>) = apply {\nkeys.forEach(::removeAdditionalProperty)\n}\n\n"
        .to_owned()
}

/// The statement `validate()` runs for one prop, if any.
fn validate_line(g: &Gen<'_>, p: &Prop<'_>) -> Option<String> {
    if p.ty == Jt::Value {
        return None;
    }
    let n = &p.name;
    let present = p.required() && !p.nullable;
    let item_class = p.ty.items().and_then(Jt::class_kind).is_some();
    let class = p.ty.class_kind().is_some();
    Some(match (present, item_class, class) {
        (_, false, false) => format!("{n}()"),
        (true, true, _) => format!("{n}().forEach {{ it.validate() }}"),
        (true, false, true) => format!("{n}().validate()"),
        (false, true, _) if g.java() => format!("{n}().ifPresent {{ it.forEach {{ it.validate() }} }}"),
        (false, true, _) => format!("{n}()?.forEach {{ it.validate() }}"),
        (false, false, true) if g.java() => format!("{n}().ifPresent {{ it.validate() }}"),
        (false, false, true) => format!("{n}()?.validate()"),
    })
}

/// One term of `validity()`.
fn validity_term(g: &Gen<'_>, p: &Prop<'_>) -> String {
    let n = &p.name;
    let known = if g.java() {
        format!("{n}.asKnown().getOrNull()")
    } else {
        format!("{n}.asKnown()")
    };
    if p.ty == Jt::Value {
        return "0".to_owned();
    }
    if let Some(items) = p.ty.items() {
        return if items.class_kind().is_some() {
            format!("({known}?.sumOf {{ it.validity().toInt() }} ?: 0)")
        } else {
            format!("({known}?.size ?: 0)")
        };
    }
    if p.ty.class_kind().is_some() {
        return format!("({known}?.validity() ?: 0)");
    }
    if g.java() {
        format!("(if ({n}.asKnown().isPresent) 1 else 0)")
    } else {
        format!("(if ({n}.asKnown() == null) 0 else 1)")
    }
}

/// An enum class: known values as constants, `Known` and `Value` enums, and validation.
// A Stainless file's members in their order, one after another: splitting the function would
// scatter that order across helpers.
#[allow(clippy::too_many_lines)]
fn enumeration(g: &Gen<'_>, c: &Class<'_>, imp: &mut Imports) -> String {
    let Shape::Enum { members, ty } = &c.shape else {
        unreachable!()
    };
    let java = g.java();
    let name = &c.name;
    let err = format!("{}InvalidDataException", g.target.client);
    imp.add("com.fasterxml.jackson.annotation.JsonCreator");
    imp.core("Enum");
    imp.core("JsonField");
    imp.sdk(&format!("errors.{err}"));
    let t = ty.kt(false);
    let consts: Vec<(String, String)> = members
        .iter()
        .map(|m| {
            let lit = match &m.value {
                ferrule_sdk_model::model::Literal::String(s) => string(s),
                ferrule_sdk_model::model::Literal::Number(n) => format!("{n}"),
                ferrule_sdk_model::model::Literal::Boolean(b) => b.to_string(),
            };
            (constant(&m.name.words), lit)
        })
        .collect();

    let mut out = String::new();
    out.push_str(&doc(&c.doc));
    let _ = writeln!(
        out,
        "class {name} @JsonCreator private constructor(private val value: JsonField<{t}>) : Enum {{\n"
    );
    out.push_str(&doc(
        "Returns this class instance's raw value.\n\nThis is usually only useful if this instance was deserialized from data that doesn't match any known member, and you want to know that value. For example, if the SDK is on an older version than the API, then the API may respond with new members that the SDK is unaware of.",
    ));
    let _ = writeln!(
        out,
        "@com.fasterxml.jackson.annotation.JsonValue fun _value(): JsonField<{t}> = value\n"
    );
    out.push_str("companion object {\n\n");
    let jvm_field = if java { "@JvmField " } else { "" };
    for (k, lit) in &consts {
        let _ = writeln!(out, "{jvm_field}val {k} = of({lit})\n");
    }
    let jvm_static = if java { "@JvmStatic " } else { "" };
    let _ = writeln!(
        out,
        "{jvm_static}fun of(value: {t}) = {name}(JsonField.of(value))\n}}\n"
    );

    let _ = writeln!(
        out,
        "/** An enum containing [{name}]'s known values. */\nenum class Known {{"
    );
    for (k, _) in &consts {
        let _ = writeln!(out, "{k},");
    }
    out.push_str("}\n\n");
    out.push_str(&doc(&format!(
        "An enum containing [{name}]'s known values, as well as an [_UNKNOWN] member.\n\nAn instance of [{name}] can contain an unknown value in a couple of cases:\n- It was deserialized from data that doesn't match any known member. For example, if the SDK is on an older version than the API, then the API may respond with new members that the SDK is unaware of.\n- It was constructed with an arbitrary value using the [of] method."
    )));
    out.push_str("enum class Value {\n");
    for (k, _) in &consts {
        let _ = writeln!(out, "{k},");
    }
    let _ = writeln!(
        out,
        "/** An enum member indicating that [{name}] was instantiated with an unknown value. */\n_UNKNOWN,\n}}\n"
    );
    out.push_str(&doc(
        "Returns an enum member corresponding to this class instance's value, or [Value._UNKNOWN] if the class was instantiated with an unknown value.\n\nUse the [known] method instead if you're certain the value is always known or if you want to throw for the unknown case.",
    ));
    out.push_str("fun value(): Value =\nwhen (this) {\n");
    for (k, _) in &consts {
        let _ = writeln!(out, "{k} -> Value.{k}");
    }
    out.push_str("else -> Value._UNKNOWN\n}\n\n");
    out.push_str(&doc(&format!(
        "Returns an enum member corresponding to this class instance's value.\n\nUse the [value] method instead if you're uncertain the value is always known and don't want to throw for the unknown case.\n\n@throws {err} if this class instance's value is a not a known member."
    )));
    out.push_str("fun known(): Known =\nwhen (this) {\n");
    for (k, _) in &consts {
        let _ = writeln!(out, "{k} -> Known.{k}");
    }
    let _ = writeln!(out, "else -> throw {err}(\"Unknown {name}: $value\")\n}}\n");
    out.push_str(&doc(&format!(
        "Returns this class instance's primitive wire representation.\n\nThis differs from the [toString] method because that method is primarily for debugging and generally doesn't throw.\n\n@throws {err} if this class instance's value does not have the expected primitive type."
    )));
    if java {
        let _ = writeln!(
            out,
            "fun as{t}(): {t} =\n_value().as{t}().orElseThrow {{\n{err}(\"Value is not a {t}\")\n}}\n"
        );
    } else {
        let _ = writeln!(
            out,
            "fun as{t}(): {t} =\n_value().as{t}() ?: throw {err}(\"Value is not a {t}\")\n"
        );
    }
    out.push_str("private var validated: Boolean = false\n\n");
    out.push_str(&doc(&format!(
        "Validates that the types of all values in this object match their expected types recursively.\n\nThis method is _not_ forwards compatible with new types from the API for existing fields.\n\n@throws {err} if any value type in this object doesn't match its expected type."
    )));
    let _ = writeln!(
        out,
        "fun validate(): {name} = apply {{\nif (validated) {{\nreturn@apply\n}}\n\nknown()\nvalidated = true\n}}\n"
    );
    let _ = writeln!(
        out,
        "fun isValid(): Boolean =\ntry {{\nvalidate()\ntrue\n}} catch (e: {err}) {{\nfalse\n}}\n"
    );
    out.push_str(&doc(
        "Returns a score indicating how many valid values are contained in this object recursively.\n\nUsed for best match union deserialization.",
    ));
    let jvm_synthetic = if java { "@JvmSynthetic " } else { "" };
    let _ = writeln!(
        out,
        "{jvm_synthetic}internal fun validity(): Int = if (value() == Value._UNKNOWN) 0 else 1\n"
    );
    let _ = writeln!(
        out,
        "override fun equals(other: Any?): Boolean {{\nif (this === other) {{\nreturn true\n}}\n\nreturn other is {name} && value == other.value\n}}\n"
    );
    out.push_str("override fun hashCode() = value.hashCode()\n\noverride fun toString() = value.toString()\n}\n");
    out
}

/// The file of a method's response class, when the response is an inline object.
#[must_use]
pub fn response_file(
    g: &Gen<'_>,
    s: &crate::emit::Service<'_>,
    m: &ferrule_sdk_model::model::Method,
) -> Option<(String, String)> {
    let ferrule_sdk_model::model::Type::Object(out) = &m.response.as_ref()?.ty else {
        return None;
    };
    let name = g.response_name(s, m);
    let c = g
        .mapper
        .object_class(name.clone(), out, out.description.clone().unwrap_or_default());
    let mut imp = Imports::new(g.root());
    let body = class(g, &c, &[], &mut imp);
    let path = format!("{}/{name}.kt", crate::emit::params::models_dir(g, s));
    Some((path, crate::kt::file(&g.models_package(s), &imp, &body)))
}

/// The file of a named model.
#[must_use]
pub fn model_file(g: &Gen<'_>, d: &ferrule_sdk_model::model::ModelDecl) -> Option<(String, String)> {
    let name = g.mapper.model_name(&d.r#ref);
    let c = match &d.ty {
        ferrule_sdk_model::model::Type::Object(out) => g.mapper.object_class(name.clone(), out, d.description.clone()),
        _ => return None,
    };
    let mut imp = Imports::new(g.root());
    let body = class(g, &c, &[], &mut imp);
    let pkg = g.mapper.model_package(&d.r#ref);
    let rel = pkg
        .strip_prefix(g.root())
        .unwrap_or(&pkg)
        .trim_start_matches('.')
        .replace('.', "/");
    let path = format!("{}/{rel}/{name}.kt", g.core_main());
    Some((path, crate::kt::file(&pkg, &imp, &body)))
}
