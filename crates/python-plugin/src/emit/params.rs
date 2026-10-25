//! `types/<resource>_<method>_params.py`: the `TypedDict` of a method's query and body parameters,
//! followed by the `TypedDicts` (and union aliases) of the inline objects inside them.

use std::collections::{HashMap, HashSet};

use super::common::{FieldLike, HEADER, method_params, method_ref, py_string, rel_import, sort_fields};
use super::format::{bracket, print_expr};
use super::models::{all_list, docstring};
use crate::model::{SdkMethod, SdkObject, SdkPrimitive, SdkRef, SdkResource, SdkType};
use crate::naming::{class_segment, pascal_case, singular, split_words};
use crate::printer::PythonPrinter;
use crate::type_mapper::{AnnotationContext, WithoutUnionNames, to_annotation};

/// A declaration in a params or model file: a class, or a union alias of classes.
#[derive(Debug, Clone, Copy)]
pub enum Decl<'a> {
    Class { name: &'a str, obj: &'a SdkObject },
    Alias { name: &'a str, union: &'a SdkType },
}

impl<'a> Decl<'a> {
    #[must_use]
    pub fn name(&self) -> &'a str {
        match self {
            Self::Class { name, .. } | Self::Alias { name, .. } => name,
        }
    }
}

/// The identity of a node of the model, as the TypeScript `Map`s key objects by reference.
fn id<T>(node: &T) -> usize {
    std::ptr::from_ref(node) as usize
}

/// The names a file gives the inline objects and object unions under its root.
#[derive(Debug, Default)]
pub struct Named<'a> {
    /// Inline object -> class name.
    names: HashMap<usize, String>,
    /// Union of objects -> alias name.
    aliases: HashMap<usize, String>,
    /// Pre-order, for `__all__`: (name, node, is alias).
    preorder: Vec<(String, Node<'a>)>,
    /// Post-order, the order they are declared in.
    postorder: Vec<(String, Node<'a>)>,
}

#[derive(Debug, Clone, Copy)]
enum Node<'a> {
    Class(&'a SdkObject),
    Alias(&'a SdkType),
}

impl<'a> Named<'a> {
    /// The class name of an inline object.
    #[must_use]
    pub fn name_of(&self, obj: &SdkObject) -> Option<&str> {
        self.names.get(&id(obj)).map(String::as_str)
    }

    /// The alias name of a union of objects.
    #[must_use]
    pub fn alias_of(&self, union: &SdkType) -> Option<&str> {
        self.aliases.get(&id(union)).map(String::as_str)
    }

    fn decls<'s>(list: &'s [(String, Node<'a>)]) -> Vec<Decl<'s>>
    where
        'a: 's,
    {
        list.iter()
            .map(|(name, node)| match *node {
                Node::Class(obj) => Decl::Class { name, obj },
                Node::Alias(union) => Decl::Alias { name, union },
            })
            .collect()
    }

    /// Pre-order, for `__all__`.
    #[must_use]
    pub fn preorder(&self) -> Vec<Decl<'_>> {
        Self::decls(&self.preorder)
    }

    /// Post-order, the order they are declared in.
    #[must_use]
    pub fn postorder(&self) -> Vec<Decl<'_>> {
        Self::decls(&self.postorder)
    }

    /// Names `obj` `name` as the root of the file: first in pre-order, last in post-order.
    pub fn add_root(&mut self, obj: &'a SdkObject, name: &str) {
        self.names.insert(id(obj), name.to_owned());
        self.preorder.insert(0, (name.to_owned(), Node::Class(obj)));
        self.postorder.push((name.to_owned(), Node::Class(obj)));
    }

    fn visit_type(&mut self, t: &'a SdkType, base: &str) {
        match t {
            SdkType::Object(obj) => self.visit_object(obj, base),
            SdkType::Array(items) => self.visit_type(items, base),
            SdkType::Map(values) => self.visit_type(values, base),
            SdkType::Union { variants, .. } => {
                let objects: Vec<&SdkObject> = variants
                    .iter()
                    .filter_map(|v| match v {
                        SdkType::Object(o) => Some(o),
                        _ => None,
                    })
                    .collect();
                if objects.len() <= 1 {
                    for v in variants {
                        self.visit_type(v, base);
                    }
                    return;
                }
                self.aliases.insert(id(t), base.to_owned());
                self.preorder.push((base.to_owned(), Node::Alias(t)));
                for (i, v) in objects.into_iter().enumerate() {
                    let hint = v
                        .hint
                        .as_ref()
                        .map_or_else(|| format!("Variant{i}"), |h| pascal_case(&split_words(h)));
                    self.visit_object(v, &format!("{base}{hint}"));
                }
                self.postorder.push((base.to_owned(), Node::Alias(t)));
            }
            _ => {}
        }
    }

    fn visit_object(&mut self, obj: &'a SdkObject, name: &str) {
        if self.names.contains_key(&id(obj)) {
            return;
        }
        self.names.insert(id(obj), name.to_owned());
        self.preorder.push((name.to_owned(), Node::Class(obj)));
        for f in sort_fields(&obj.fields) {
            self.visit_field(f, name);
        }
        self.postorder.push((name.to_owned(), Node::Class(obj)));
    }

    fn visit_field<F: FieldLike>(&mut self, f: &'a F, prefix: &str) {
        let words = split_words(f.wire_name());
        let in_array = matches!(f.ty(), SdkType::Array(_));
        let segment = class_segment(&if in_array { singular(&words) } else { words });
        self.visit_type(f.ty(), &format!("{prefix}{segment}"));
    }
}

/// Names the inline objects of a method's parameters: a field's object is named after the field
/// (singular inside an array), prefixed by its parent's name below the top level; the variants
/// of a union of objects are named after the schemas they come from.
#[must_use]
pub fn name_params<'a, F: FieldLike>(params: &[&'a F], root_prefix: &str) -> Named<'a> {
    let mut named = Named::default();
    for &p in &sort_fields(params.iter().copied()) {
        named.visit_field(p, root_prefix);
    }
    named
}

/// The annotation context of a params file, or of method arguments using its names.
pub struct ParamsContext<'n, 'a> {
    pub named: &'n Named<'a>,
    /// Prefix of every inline name, such as `stop_list_params.`.
    pub qualify: String,
    /// The module of the file being printed, for relative imports.
    pub here: String,
    /// Names the file's fields use (`shadowed` in the TypeScript context).
    pub shadowed: HashSet<String>,
}

impl AnnotationContext for ParamsContext<'_, '_> {
    fn object_name(&self, obj: &SdkObject) -> String {
        format!("{}{}", self.qualify, self.named.name_of(obj).unwrap_or("object"))
    }
    fn ref_name(&self, p: &mut PythonPrinter, r: &SdkRef) -> String {
        p.import_from(&rel_import(&self.here, &format!("types.{}", r.module)), &r.name);
        r.name.clone()
    }
    fn runtime(&self, p: &mut PythonPrinter, module: &str, name: &str) {
        p.import_from(&rel_import(&self.here, module), name);
    }
    fn input(&self) -> bool {
        true
    }
    fn shadowed(&self, name: &str) -> bool {
        self.shadowed.contains(name)
    }
    fn union_name(&self, union: &SdkType) -> Option<String> {
        self.named.alias_of(union).map(|n| format!("{}{n}", self.qualify))
    }
}

fn is_date(t: &SdkType) -> bool {
    matches!(t, SdkType::Primitive(SdkPrimitive::Datetime | SdkPrimitive::Date))
}

fn field_annotation<F: FieldLike>(p: &mut PythonPrinter, f: &F, ctx: &dyn AnnotationContext) -> String {
    let mut ann = to_annotation(f.ty(), p, ctx);
    let mut info: Vec<String> = Vec::new();
    if f.py_name() != f.wire_name() {
        info.push(format!("alias={}", py_string(f.wire_name())));
    }
    if is_date(f.ty()) {
        info.push("format=\"iso8601\"".to_owned());
    }
    if !info.is_empty() {
        p.import_from("typing_extensions", "Annotated");
        ctx.runtime(p, "_utils", "PropertyInfo");
        ann = format!("Annotated[{ann}, PropertyInfo({})]", info.join(", "));
    }
    if f.required() {
        p.import_from("typing_extensions", "Required");
        ann = format!("Required[{ann}]");
    }
    ann
}

fn fields_of<F: FieldLike>(p: &mut PythonPrinter, fields: &[&F], ctx: &dyn AnnotationContext) {
    for (i, f) in sort_fields(fields.iter().copied()).into_iter().enumerate() {
        if i > 0 {
            p.blank();
        }
        let ann = field_annotation(p, f, ctx);
        p.line(&format!("{}: {ann}", f.py_name()));
        for l in docstring(f.description(), "") {
            p.line(&l);
        }
    }
}

#[must_use]
pub fn emit_params_file(resource: Option<&SdkResource>, method: &SdkMethod) -> String {
    let r = method_ref(resource, method, "Params");
    let here = format!("types.{}", r.module);
    let mut p = PythonPrinter::new(&[HEADER]);
    p.import_from("__future__", "annotations");
    p.import_from("typing_extensions", "TypedDict");
    let params = method_params(method);
    let named = name_params(&params, "");
    let ctx = ParamsContext {
        named: &named,
        qualify: String::new(),
        here,
        shadowed: params.iter().map(|q| q.py_name.clone()).collect(),
    };
    let mut exported = vec![r.name.clone()];
    exported.extend(named.preorder().iter().map(|d| d.name().to_owned()));
    for l in all_list(&exported) {
        p.line(&l);
    }
    p.blank().blank();
    p.block(&format!("class {}(TypedDict, total=False):", r.name), |p| {
        fields_of(p, &params, &ctx);
    });
    for decl in named.postorder() {
        p.blank().blank();
        match decl {
            Decl::Alias { name, union } => {
                p.import_from("typing_extensions", "TypeAlias");
                p.import_from("typing", "Union");
                let plain = WithoutUnionNames(&ctx);
                let variants = match union {
                    SdkType::Union { variants, .. } => variants
                        .iter()
                        .map(|v| to_annotation(v, &mut p, &plain).into())
                        .collect(),
                    _ => Vec::new(),
                };
                print_expr(
                    &mut p,
                    &bracket("Union[", variants, false, "]").into(),
                    &format!("{name}: TypeAlias = "),
                    "",
                );
            }
            Decl::Class { name, obj } => {
                p.block(&format!("class {name}(TypedDict, total=False):"), |p| {
                    let doc = docstring(obj.description.as_deref().unwrap_or(""), "");
                    for l in &doc {
                        p.line(l);
                    }
                    if !doc.is_empty() && !obj.fields.is_empty() {
                        p.blank();
                    }
                    let fields: Vec<_> = obj.fields.iter().collect();
                    fields_of(p, &fields, &ctx);
                });
            }
        }
    }
    p.render()
}
