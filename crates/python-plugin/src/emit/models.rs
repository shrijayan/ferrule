//! Pydantic model files under `types/`: one root model and the inline objects it contains.

use std::cell::RefCell;

use super::common::{HEADER, escape_docstring, py_string, rel_import, sort_fields, wrap};
use super::cycles::Cycles;
use super::format::{call, format_expr};
use super::params::{Decl, Named, name_params};
use crate::js;
use crate::model::{SdkField, SdkObject, SdkRef, SdkType};
use crate::printer::{LINE_LENGTH, PythonPrinter};
use crate::type_mapper::{AnnotationContext, alias_expr, inline_union_alias_expr, is_nullable, to_annotation};

/// Wrap width of docstring text (measured on the oracles).
const DOC_WIDTH: usize = 80;
/// Longest first sentence still printed as a summary line.
const SUMMARY_MAX: usize = 82;

pub struct ModelFile<'a> {
    /// The root class and its module under `types`.
    pub r#ref: &'a SdkRef,
    pub ty: &'a SdkObject,
    /// Whether nested classes carry the root's name: true for types synthesized from a method
    /// (`AgenciesWithCoverageListResponseData`), false for models the config names (`Agency`
    /// inside `References`).
    pub prefix_children: bool,
    /// The reference cycles among the named models, for forward references.
    pub cycles: Option<&'a Cycles>,
}

/// A module declaring `Name: TypeAlias = ...`: a named model whose schema is not an object, or a
/// method's response that is not one.
pub struct AliasFile<'a> {
    pub r#ref: &'a SdkRef,
    pub ty: &'a SdkType,
    /// The reference cycles among the named models, for forward references.
    pub cycles: Option<&'a Cycles>,
}

/// Names the root and its inline objects; see [`name_params`] for the rules.
fn name_objects<'a>(file: &ModelFile<'a>) -> Named<'a> {
    let fields: Vec<&'a SdkField> = file.ty.fields.iter().collect();
    let mut named = name_params(&fields, if file.prefix_children { &file.r#ref.name } else { "" });
    named.add_root(file.ty, &file.r#ref.name);
    named
}

struct ModelContext<'n, 'a> {
    named: &'n Named<'a>,
    here: String,
    module: &'n str,
    cycles: Option<&'n Cycles>,
    /// The imports of deferred models, printed after the declarations.
    trailer: RefCell<PythonPrinter>,
}

impl<'n, 'a> ModelContext<'n, 'a> {
    fn new(named: &'n Named<'a>, module: &'n str, cycles: Option<&'n Cycles>) -> Self {
        Self {
            named,
            here: format!("types.{module}"),
            module,
            cycles,
            trailer: RefCell::new(PythonPrinter::default()),
        }
    }

    /// The name of `r` imported at the top of the module, as a base class needs it.
    fn import_ref(&self, p: &mut PythonPrinter, r: &SdkRef) -> String {
        if r.module != self.module {
            p.import_from(&rel_import(&self.here, &format!("types.{}", r.module)), &r.name);
        }
        r.name.clone()
    }

    /// The imports after the declarations, or none.
    fn trailer_lines(&self) -> Vec<String> {
        self.trailer.borrow().import_block("")
    }
}

impl AnnotationContext for ModelContext<'_, '_> {
    fn object_name(&self, obj: &SdkObject) -> String {
        self.named.name_of(obj).unwrap_or("").to_owned()
    }
    fn ref_name(&self, p: &mut PythonPrinter, r: &SdkRef) -> String {
        if self.cycles.is_some_and(|c| c.is_deferred(r)) {
            // A forward reference, imported once every declaration of the module exists.
            p.import_from("__future__", "annotations");
            if r.module != self.module {
                self.trailer
                    .borrow_mut()
                    .import_from(&rel_import(&self.here, &format!("types.{}", r.module)), &r.name);
            }
            return py_string(&r.name);
        }
        self.import_ref(p, r)
    }
    fn runtime(&self, p: &mut PythonPrinter, module: &str, name: &str) {
        p.import_from(&rel_import(&self.here, module), name);
    }
    fn union_name(&self, union: &SdkType) -> Option<String> {
        self.named.alias_of(union).map(str::to_owned)
    }
}

/// Field names that would shadow an attribute of Pydantic's `BaseModel`: Stainless appends `_`
/// and keeps the wire name as the alias (`schema_`: knock-mgmt, imagekit, ogx, perplexity). The
/// oracles keep other names that `BaseModel` v1 also has, such as `fields`.
const SHADOWS_BASE_MODEL: [&str; 1] = ["schema"];

/// The attribute name of a model field.
fn model_py_name(field: &SdkField) -> String {
    if SHADOWS_BASE_MODEL.contains(&field.py_name.as_str()) {
        format!("{}_", field.py_name)
    } else {
        field.py_name.clone()
    }
}

fn field_lines(p: &mut PythonPrinter, field: &SdkField, ctx: &dyn AnnotationContext, indent: &str) -> Vec<String> {
    let py_name = model_py_name(field);
    let mut ann = to_annotation(&field.ty, p, ctx);
    if !field.required && !is_nullable(&field.ty) {
        // A plain union takes `None` as one more member: `Union[A, B, None]`.
        if matches!(field.ty, SdkType::Union { .. }) && ann.starts_with("Union[") {
            ann = format!("{}, None]", &ann[..ann.len() - 1]);
        } else {
            p.import_from("typing", "Optional");
            ann = format!("Optional[{ann}]");
        }
    }
    // Optional fields, and required nullable ones (as the oracles print them), default to None.
    let has_default = !field.required || is_nullable(&field.ty);
    let alias = py_name != field.wire_name;
    if alias {
        p.import_from("pydantic", "Field as FieldInfo");
    }
    let mut args: Vec<String> = Vec::new();
    if alias {
        args.push(format!("alias={}", py_string(&field.wire_name)));
    }
    if has_default {
        args.push("default=None".to_owned());
    }
    let args = args.join(", ");
    let head = format!("{indent}{py_name}: {ann}");
    let one = format!("{head} = FieldInfo({args})");
    let mut lines = if !alias {
        vec![if has_default { format!("{head} = None") } else { head }]
    } else if js::len(&one) <= LINE_LENGTH {
        vec![one]
    } else {
        vec![
            format!("{head} = FieldInfo("),
            format!("{indent}    {args}"),
            format!("{indent})"),
        ]
    };
    lines.extend(docstring(&field.description, indent));
    lines
}

/// `text.split(/\n\s*\n/)`.
fn split_paragraphs(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\n' {
            let mut j = i + 1;
            let mut last_newline = None;
            while j < chars.len() && js::is_space(chars[j]) {
                if chars[j] == '\n' {
                    last_newline = Some(j);
                }
                j += 1;
            }
            if let Some(k) = last_newline {
                out.push(chars[start..i].iter().collect());
                start = k + 1;
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out.push(chars[start..].iter().collect());
    out
}

/// `text.replace(/\s+/g, ' ')`.
fn collapse_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if js::is_space(c) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// `/^(.+?[.!?])\s+(?=\S)(.*)$/.exec(text)` on text whose spaces are collapsed: the first
/// sentence and the rest.
fn first_sentence(text: &str) -> Option<(String, String)> {
    let chars: Vec<char> = text.chars().collect();
    for i in 1..chars.len() {
        if !matches!(chars[i], '.' | '!' | '?') {
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && js::is_space(chars[j]) {
            j += 1;
        }
        if j > i + 1 && j < chars.len() {
            return Some((chars[..=i].iter().collect(), chars[j..].iter().collect()));
        }
    }
    None
}

fn indented(lines: &[String], indent: &str) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{indent}{l}")
            }
        })
        .collect()
}

/// Wrapped paragraphs with a blank line between them.
fn paragraphs(paras: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, para) in paras.iter().enumerate() {
        if i > 0 {
            out.push(String::new());
        }
        out.extend(wrap(para, DOC_WIDTH));
    }
    out
}

/// A docstring: one line when it fits, otherwise wrapped with the quotes on their own line.
#[must_use]
pub fn docstring(text: &str, indent: &str) -> Vec<String> {
    let clean = escape_docstring(js::trim(text));
    if clean.is_empty() {
        return Vec::new();
    }
    let body = wrap(&clean, DOC_WIDTH);
    let one = format!("{indent}\"\"\"{}\"\"\"", body.join(" "));
    if body.len() == 1 && js::len(&one) <= LINE_LENGTH {
        return vec![one];
    }
    // Summary style when the first paragraph's first sentence is short and more text follows.
    let mut paras = split_paragraphs(&clean);
    let para = paras.remove(0);
    let flat_para = collapse_spaces(&para);
    let m = first_sentence(&flat_para);
    let summary = m.as_ref().map_or_else(|| flat_para.clone(), |(s, _)| s.clone());
    let mut rest_paras: Vec<String> = m
        .iter()
        .map(|(_, rest)| rest.clone())
        .filter(|r| !r.is_empty())
        .collect();
    rest_paras.extend(paras.iter().cloned());
    if !rest_paras.is_empty() && js::len(&summary) <= SUMMARY_MAX {
        let wrapped = wrap(&summary, DOC_WIDTH);
        let mut out = vec![format!("{indent}\"\"\"{}", wrapped.first().map_or("", String::as_str))];
        out.extend(indented(&wrapped[wrapped.len().min(1)..], indent));
        out.push(String::new());
        out.extend(indented(&paragraphs(&rest_paras), indent));
        out.push(format!("{indent}\"\"\""));
        return out;
    }
    let mut all = vec![para];
    all.extend(paras);
    let mut out = vec![format!("{indent}\"\"\"")];
    out.extend(indented(&paragraphs(&all), indent));
    out.push(format!("{indent}\"\"\""));
    out
}

/// The `__all__` list, wrapped one name per line when it doesn't fit.
#[must_use]
pub fn all_list<S: AsRef<str>>(names: &[S]) -> Vec<String> {
    let quoted: Vec<String> = names.iter().map(|n| py_string(n.as_ref())).collect();
    let one = format!("__all__ = [{}]", quoted.join(", "));
    if js::len(&one) <= LINE_LENGTH {
        return vec![one];
    }
    let mut out = vec!["__all__ = [".to_owned()];
    out.extend(quoted.iter().map(|q| format!("    {q},")));
    out.push("]".to_owned());
    out
}

/// One top-level statement of a model module, for the blank lines between them.
struct Block {
    /// A class, which ruff surrounds with two blank lines; other statements get one.
    class: bool,
    lines: Vec<String>,
}

/// Prints `__all__`, the blocks and the trailing imports with the blank lines ruff leaves.
fn print_module(p: &mut PythonPrinter, exported: &[&str], blocks: Vec<Block>, trailer: &[String]) {
    for line in all_list(exported) {
        p.line(&line);
    }
    let mut previous_class = false;
    for block in blocks {
        p.blank();
        if block.class || previous_class {
            p.blank();
        }
        for line in &block.lines {
            p.line(line);
        }
        previous_class = block.class;
    }
    if !trailer.is_empty() {
        p.blank();
        if previous_class {
            p.blank();
        }
        for line in trailer {
            p.line(line);
        }
    }
}

/// The classes and union aliases `named` declares, in declaration order.
fn decl_blocks(p: &mut PythonPrinter, named: &Named<'_>, ctx: &ModelContext<'_, '_>) -> Vec<Block> {
    named
        .postorder()
        .into_iter()
        .map(|decl| match decl {
            Decl::Alias { name, union } => Block {
                class: false,
                lines: alias_lines(p, name, union, ctx),
            },
            Decl::Class { name, obj } => Block {
                class: true,
                lines: class_lines(p, name, obj, ctx),
            },
        })
        .collect()
}

/// Whether `t` holds an inline object, which would need a class name of its own.
fn has_inline_object(t: &SdkType) -> bool {
    match t {
        SdkType::Object(_) => true,
        SdkType::Array(inner) | SdkType::Map(inner) => has_inline_object(inner),
        SdkType::Union { variants, .. } => variants.iter().any(has_inline_object),
        _ => false,
    }
}

/// The typed extra keys of an object with fields that takes other keys too, as knock-mgmt-python
/// `types/source_environment_settings.py` declares them: `__pydantic_extra__` (assigned only for
/// type checkers, for Pydantic before 2.8) and a `__getattr__` stub. `builtins.object` where a
/// field named `object` shadows the builtin (ogx-client-python `batch_cancel_response.py`).
/// Extra values that are inline objects are left out: they would need names Stainless gives them.
fn extra_lines(p: &mut PythonPrinter, obj: &SdkObject, ctx: &ModelContext<'_, '_>) -> Option<Vec<String>> {
    let extra = obj.extra.as_deref().filter(|t| !has_inline_object(t))?;
    let ann = to_annotation(extra, p, ctx);
    let ann = if ann == "object" && obj.fields.iter().any(|f| f.py_name == "object") {
        p.import_module("builtins");
        "builtins.object".to_owned()
    } else {
        ann
    };
    p.import_from("typing", "TYPE_CHECKING");
    p.import_from("typing", "Dict");
    p.import_from("pydantic", "Field as FieldInfo");
    Some(vec![
        "    if TYPE_CHECKING:".to_owned(),
        "        # Some versions of Pydantic <2.8.0 have a bug and don\u{2019}t allow assigning a".to_owned(),
        "        # value to this field, so for compatibility we avoid doing it at runtime.".to_owned(),
        format!(
            "        __pydantic_extra__: Dict[str, {ann}] = FieldInfo(init=False)  # pyright: ignore[reportIncompatibleVariableOverride]"
        ),
        String::new(),
        "        # Stub to indicate that arbitrary properties are accepted.".to_owned(),
        "        # To access properties that are not valid identifiers you can use `getattr`, e.g.".to_owned(),
        "        # `getattr(obj, '$type')`".to_owned(),
        format!("        def __getattr__(self, attr: str) -> {ann}: ..."),
        "    else:".to_owned(),
        format!("        __pydantic_extra__: Dict[str, {ann}]"),
    ])
}

fn class_lines(p: &mut PythonPrinter, name: &str, obj: &SdkObject, ctx: &ModelContext<'_, '_>) -> Vec<String> {
    let mut bases: Vec<String> = obj.bases.iter().map(|b| ctx.import_ref(p, b)).collect();
    if bases.is_empty() {
        p.import_from(&rel_import(&ctx.here, "_models"), "BaseModel");
        bases.push("BaseModel".to_owned());
    }
    let mut fields: Vec<Vec<String>> = sort_fields(&obj.fields)
        .into_iter()
        .map(|f| field_lines(p, f, ctx, "    "))
        .collect();
    if let Some(extra) = extra_lines(p, obj, ctx) {
        fields.push(extra);
    }
    let doc = docstring(obj.description.as_deref().unwrap_or(""), "    ");
    let body: Vec<Vec<String>> = if fields.is_empty() && doc.is_empty() {
        vec![vec!["    pass".to_owned()]]
    } else {
        let mut b = Vec::new();
        if !doc.is_empty() {
            b.push(doc);
        }
        b.extend(fields);
        b
    };
    let mut lines = vec![format!("class {name}({}):", bases.join(", "))];
    for (i, f) in body.into_iter().enumerate() {
        if i > 0 {
            lines.push(String::new());
        }
        lines.extend(f);
    }
    lines
}

/// Renders one model file, `types/<module>.py`.
#[must_use]
pub fn emit_model_file(file: &ModelFile<'_>) -> String {
    let mut p = PythonPrinter::new(&[HEADER]);
    let named = name_objects(file);
    let ctx = ModelContext::new(&named, &file.r#ref.module, file.cycles);
    let blocks = decl_blocks(&mut p, &named, &ctx);
    let exported: Vec<&str> = named.preorder().iter().map(Decl::name).collect();
    print_module(&mut p, &exported, blocks, &ctx.trailer_lines());
    p.render()
}

/// A module whose root is a type alias, `Name: TypeAlias = ...`, after the classes and aliases of
/// the inline objects inside it: the items of an array or the values of a map are `<Name>Item`,
/// the object variants of a union are named after their schemas. A union in a reference cycle
/// is a `TypeAliasType`, as Pydantic needs for a recursive alias.
#[must_use]
pub fn emit_alias_model_file(file: &AliasFile<'_>) -> String {
    let name = &file.r#ref.name;
    let mut p = PythonPrinter::new(&[HEADER]);
    p.import_from("typing_extensions", "TypeAlias");
    let mut named = Named::default();
    named.add_alias_root(file.ty, name);
    let ctx = ModelContext::new(&named, &file.r#ref.module, file.cycles);
    let mut blocks = decl_blocks(&mut p, &named, &ctx);
    let expr = alias_expr(file.ty, &mut p, &ctx, true);
    let recursive = matches!(file.ty, SdkType::Union { .. }) && file.cycles.is_some_and(|c| c.is_cyclic(file.r#ref));
    let lines = if recursive {
        p.import_from("typing", "TYPE_CHECKING");
        p.import_from("typing_extensions", "TypeAliasType");
        ctx.runtime(&mut p, "_compat", "PYDANTIC_V1");
        let mut lines = vec!["if TYPE_CHECKING or not PYDANTIC_V1:".to_owned()];
        let wrapped = call("TypeAliasType(", vec![py_string(name).into(), expr.clone()], false);
        lines.extend(format_expr(&wrapped.into(), "    ", &format!("{name} = "), ""));
        lines.push("else:".to_owned());
        lines.extend(format_expr(&expr, "    ", &format!("{name}: TypeAlias = "), ""));
        lines
    } else {
        format_expr(&expr, "", &format!("{name}: TypeAlias = "), "")
    };
    blocks.push(Block { class: false, lines });
    let mut exported: Vec<&str> = vec![name];
    exported.extend(named.preorder().iter().map(Decl::name));
    print_module(&mut p, &exported, blocks, &ctx.trailer_lines());
    p.render()
}

/// A response that is not an object: `Name: TypeAlias = <annotation>`.
#[must_use]
pub fn emit_alias_file(r: &SdkRef, ty: &SdkType) -> String {
    emit_alias_model_file(&AliasFile {
        r#ref: r,
        ty,
        cycles: None,
    })
}

/// `Name: TypeAlias = Union[...]`, `Annotated` with its discriminator when the spec names one.
fn alias_lines(p: &mut PythonPrinter, name: &str, u: &SdkType, ctx: &dyn AnnotationContext) -> Vec<String> {
    p.import_from("typing_extensions", "TypeAlias");
    let expr = inline_union_alias_expr(u, p, ctx, true);
    format_expr(&expr, "", &format!("{name}: TypeAlias = "), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SdkPrimitive;

    fn field(wire: &str, required: bool, ty: SdkType) -> SdkField {
        SdkField {
            wire_name: wire.into(),
            py_name: wire.into(),
            required,
            description: String::new(),
            ty,
        }
    }

    #[test]
    fn wraps_long_docstrings() {
        assert_eq!(docstring("", ""), Vec::<String>::new());
        let word = "word ".repeat(30);
        assert_eq!(
            docstring(&word, "    "),
            [
                "    \"\"\"".to_owned(),
                format!("    {}", "word ".repeat(16).trim()),
                format!("    {}", "word ".repeat(14).trim()),
                "    \"\"\"".to_owned(),
            ]
        );
        assert_eq!(
            docstring("Short first. Then more.\n\nAnd a paragraph.", ""),
            ["\"\"\"Short first.", "", "Then more.", "", "And a paragraph.", "\"\"\""]
        );
    }

    #[test]
    fn escapes_docstrings_so_the_spec_cannot_close_them() {
        let lines = docstring("x \"\"\" + __import__(\"os\").system(\"id\") + \"\"\"", "    ");
        let text = lines.join("\n");
        let text = text.trim();
        assert!(!text[3..text.len() - 3].contains("\"\"\""));
        assert!(text.contains("\\\"\\\"\\\""));
        assert_eq!(docstring("ends with \"", ""), ["\"\"\"ends with \\\"\"\"\""]);
    }

    #[test]
    fn prints_pass_for_empty_classes() {
        let r = SdkRef {
            name: "Empty".into(),
            module: "empty".into(),
        };
        let obj = SdkObject {
            fields: vec![
                field(
                    "items",
                    true,
                    SdkType::Map(Box::new(SdkType::Object(SdkObject::default()))),
                ),
                field("same", false, SdkType::Ref(r.clone())),
            ],
            ..SdkObject::default()
        };
        let out = emit_model_file(&ModelFile {
            r#ref: &r,
            ty: &obj,
            prefix_children: true,
            cycles: None,
        });
        assert!(out.contains("class EmptyItems(BaseModel):\n    pass\n"), "{out}");
        assert!(out.contains("items: Dict[str, EmptyItems]\n"));
        assert!(out.contains("same: Optional[Empty] = None\n"));
    }

    #[test]
    fn names_union_variants_after_their_schemas() {
        let variant = |hint: &str, tag: &str| {
            SdkType::Object(SdkObject {
                hint: Some(hint.into()),
                description: Some(format!("{hint} object")),
                fields: vec![field(
                    "type",
                    true,
                    SdkType::Literal(vec![crate::model::SdkLiteral::String(tag.into())]),
                )],
                ..SdkObject::default()
            })
        };
        let obj = SdkObject {
            description: Some("A thing.".into()),
            fields: vec![
                field(
                    "frame",
                    false,
                    SdkType::Union {
                        variants: vec![variant("ImageRef", "image"), variant("VideoRef", "video")],
                        discriminator: Some("type".into()),
                    },
                ),
                field(
                    "n",
                    false,
                    SdkType::Union {
                        variants: vec![
                            SdkType::Primitive(SdkPrimitive::String),
                            SdkType::Primitive(SdkPrimitive::Integer),
                        ],
                        discriminator: None,
                    },
                ),
            ],
            ..SdkObject::default()
        };
        let r = SdkRef {
            name: "Thing".into(),
            module: "thing".into(),
        };
        let model = emit_model_file(&ModelFile {
            r#ref: &r,
            ty: &obj,
            prefix_children: false,
            cycles: None,
        });
        assert!(
            model.contains("class Thing(BaseModel):\n    \"\"\"A thing.\"\"\""),
            "{model}"
        );
        assert!(model.contains(
            "Frame: TypeAlias = Annotated[Union[FrameImageRef, FrameVideoRef], PropertyInfo(discriminator=\"type\")]"
        ));
        assert!(model.contains("n: Union[str, int, None] = None"));
    }

    #[test]
    fn names_variants_without_a_schema_after_their_tag() {
        let variant = |tag: &str| {
            SdkType::Object(SdkObject {
                fields: vec![field(
                    "type",
                    true,
                    SdkType::Literal(vec![crate::model::SdkLiteral::String(tag.into())]),
                )],
                ..SdkObject::default()
            })
        };
        let union = SdkType::Union {
            variants: vec![variant("S3_COMPATIBLE"), variant("web-folder"), variant("GCS")],
            discriminator: Some("type".into()),
        };
        let r = SdkRef {
            name: "Origin".into(),
            module: "origin".into(),
        };
        let out = emit_alias_model_file(&AliasFile {
            r#ref: &r,
            ty: &union,
            cycles: None,
        });
        assert!(
            out.contains("__all__ = [\"Origin\", \"S3Compatible\", \"WebFolder\", \"Gcs\"]"),
            "{out}"
        );
        // Without a discriminator there is no tag to go by.
        let plain = SdkType::Union {
            variants: vec![variant("a"), variant("b")],
            discriminator: None,
        };
        let out = emit_alias_model_file(&AliasFile {
            r#ref: &r,
            ty: &plain,
            cycles: None,
        });
        assert!(
            out.contains("Origin: TypeAlias = Union[OriginVariant0, OriginVariant1]"),
            "{out}"
        );
    }

    #[test]
    fn aliases_non_object_responses() {
        let out = emit_alias_file(
            &SdkRef {
                name: "Ids".into(),
                module: "ids".into(),
            },
            &SdkType::Array(Box::new(SdkType::Ref(SdkRef {
                name: "Id".into(),
                module: "shared.id".into(),
            }))),
        );
        assert!(out.contains("from .shared.id import Id"), "{out}");
    }
}
