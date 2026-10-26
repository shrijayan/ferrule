//! `types/<resource>_<method>_params.py`: the `TypedDict` of a method's query and body parameters,
//! followed by the `TypedDicts` (and union aliases) of the inline objects inside them.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use super::common::{
    FieldLike, HEADER, keyword_path_params, method_params, method_ref, py_string, rel_import, sort_fields, typed_extra,
};
use super::format::{bracket, print_expr};
use super::models::{all_list, docstring};
use crate::model::{SdkLiteral, SdkMethod, SdkObject, SdkParam, SdkPrimitive, SdkRef, SdkResource, SdkType};
use crate::naming::{class_segment, schema_segment, singular, split_words};
use crate::printer::PythonPrinter;
use crate::type_mapper::{AnnotationContext, WithoutUnionNames, to_annotation, union_members};

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

    /// Names the inline objects under the type of an alias module `name` (whose root the caller
    /// prints): the items of an array or the values of a map are `<name>Item`; the object
    /// variants of a union are named after the schemas they come from (`<name>Variant<i>`
    /// without one), without the root's name, as Stainless names them (`ConditionGroupAllMatch`).
    pub fn add_alias_root(&mut self, ty: &'a SdkType, name: &str) {
        match ty {
            SdkType::Array(items) | SdkType::Map(items) => self.visit_type(items, &format!("{name}Item")),
            SdkType::Union {
                variants,
                discriminator,
            } => {
                let mut i = 0;
                for v in variants {
                    if let SdkType::Object(o) = v {
                        let segment =
                            variant_segment(o, discriminator.as_deref()).unwrap_or_else(|| format!("{name}Variant{i}"));
                        self.visit_object(o, &segment);
                        i += 1;
                    } else {
                        self.visit_type(v, &format!("{name}Item"));
                    }
                }
            }
            _ => {}
        }
    }

    /// Names the inline objects under `params` (in field order) with `prefix`, as below a class
    /// of that name.
    pub fn add_fields<F: FieldLike>(&mut self, params: &[&'a F], prefix: &str) {
        for &p in &sort_fields(params.iter().copied()) {
            self.visit_field(p, prefix);
        }
    }

    /// How many declarations there are in pre-order and in post-order.
    fn counts(&self) -> (usize, usize) {
        (self.preorder.len(), self.postorder.len())
    }

    fn visit_type(&mut self, t: &'a SdkType, base: &str) {
        match t {
            SdkType::Object(obj) => self.visit_object(obj, base),
            SdkType::Array(items) => self.visit_type(items, base),
            SdkType::Map(values) => self.visit_type(values, base),
            SdkType::Union {
                variants,
                discriminator,
            } => {
                let objects: Vec<&SdkObject> = variants
                    .iter()
                    .filter_map(|v| match v {
                        SdkType::Object(o) => Some(o),
                        _ => None,
                    })
                    .collect();
                // A union with a named model or a named object among two or more members is an
                // alias too, as Stainless declares it (imagekit `AssetListResponseItem: TypeAlias =
                // Union[File, Folder]`, knock-mgmt `Recipient: TypeAlias = Union[str,
                // RecipientObjectRecipientReference]`), unless it would have to make up a name for
                // its one inline object.
                let members = variants
                    .iter()
                    .filter(|v| !matches!(v, SdkType::Primitive(SdkPrimitive::Null)))
                    .count();
                let with_model = members >= 2
                    && variants
                        .iter()
                        .any(|v| matches!(v, SdkType::Ref(_) | SdkType::Object(_)))
                    && objects
                        .iter()
                        .all(|o| variant_segment(o, discriminator.as_deref()).is_some());
                if objects.len() <= 1 && !with_model {
                    for v in variants {
                        // An array beside other members, named after a schema or title, names its
                        // items after it (knock-mgmt `HeadersRequestTemplateHeadersArray`).
                        if members >= 2
                            && let SdkType::Array(items) = v
                            && let SdkType::Object(o) = &**items
                            && let Some(h) = &o.hint
                        {
                            self.visit_object(o, &format!("{base}{}", schema_segment(h)));
                        } else {
                            self.visit_type(v, base);
                        }
                    }
                    return;
                }
                self.aliases.insert(id(t), base.to_owned());
                self.preorder.push((base.to_owned(), Node::Alias(t)));
                for (i, v) in objects.into_iter().enumerate() {
                    let segment = variant_segment(v, discriminator.as_deref()).unwrap_or_else(|| format!("Variant{i}"));
                    self.visit_object(v, &format!("{base}{segment}"));
                }
                // The inline objects under an array member are named `List` after it, as ogx
                // names the array member of `InterleavedContent`; other members, `Item`.
                for v in variants {
                    match v {
                        SdkType::Object(_) => {}
                        SdkType::Array(items) => self.visit_type(items, &format!("{base}List")),
                        other => self.visit_type(other, &format!("{base}Item")),
                    }
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
        // `T[]`, `T[] | null` and a map of `T[]` alike: the item class is named in the singular
        // (knock-mgmt `example_payloads: Dict[str, List[ExamplePayload]]`).
        let is_list = |t: &SdkType| match t {
            SdkType::Array(_) => true,
            SdkType::Map(values) => matches!(**values, SdkType::Array(_)),
            _ => false,
        };
        let in_array = match f.ty() {
            SdkType::Union { variants, .. } => {
                let mut rest = variants
                    .iter()
                    .filter(|v| !matches!(v, SdkType::Primitive(SdkPrimitive::Null)));
                matches!((rest.next(), rest.next()), (Some(t), None) if is_list(t))
            }
            t => is_list(t),
        };
        let segment = class_segment(&if in_array { singular(&words) } else { words });
        self.visit_type(f.ty(), &format!("{prefix}{segment}"));
    }
}

/// The name of an inline object variant of a union: after the schema it comes from, else after
/// the one value its discriminator property takes (`S3_COMPATIBLE` is `S3Compatible`, imagekit
/// `OriginResponse`), else `None`.
///
/// Stainless prefers the value over the schema name when the discriminator has no `mapping`
/// (imagekit `ExtensionItemRemoveBg` from `RemovedotBGExtension`) and the schema name when it
/// has one (knock-mgmt `AudienceStaticAudienceRequest`, opencode `PatchPart`). Forge does not say
/// whether a mapping was written, so the schema name comes first.
fn variant_segment(obj: &SdkObject, discriminator: Option<&str>) -> Option<String> {
    if let Some(h) = &obj.hint {
        return Some(schema_segment(h));
    }
    if let Some(d) = discriminator
        && let Some(f) = obj.fields.iter().find(|f| f.wire_name == d)
        && let SdkType::Literal(values) = &f.ty
        && let [SdkLiteral::String(value)] = values.as_slice()
    {
        let words: Vec<String> = split_words(value).iter().map(|w| w.to_ascii_lowercase()).collect();
        let segment = class_segment(&words);
        if !segment.is_empty() {
            return Some(segment);
        }
    }
    None
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

/// One variant of a union body, as its params file declares it.
#[derive(Debug)]
pub struct UnionVariant<'a> {
    /// Its `TypedDict`, such as `S3` or `GuideBooleanActivationParams`.
    pub name: String,
    /// Its keyword arguments: the query parameters and the variant's fields, in the oracle order.
    pub args: Vec<&'a SdkParam>,
    /// Its inline objects in [`Named::preorder`] and [`Named::postorder`].
    pre: Range<usize>,
    post: Range<usize>,
}

/// The names of a union body (`body_variants`): a `TypedDict` per variant, named after the
/// schema it comes from or its discriminator value (`Variant<i>` without either), and its inline
/// objects named below it (`ChangePublicationStatusPublish`, imagekit `file_update_params.py`).
#[derive(Debug)]
pub struct UnionBody<'a> {
    pub named: Named<'a>,
    pub variants: Vec<UnionVariant<'a>>,
    /// The implementation's keyword arguments: every variant's, in the order they first appear.
    pub args: Vec<&'a SdkParam>,
}

impl UnionBody<'_> {
    /// The inline objects of variant `i` in pre-order, for `__all__`.
    #[must_use]
    pub fn preorder(&self, i: usize) -> Vec<Decl<'_>> {
        self.named.preorder()[self.variants[i].pre.clone()].to_vec()
    }

    /// The inline objects of variant `i` in post-order, the order they are declared in.
    #[must_use]
    pub fn postorder(&self, i: usize) -> Vec<Decl<'_>> {
        self.named.postorder()[self.variants[i].post.clone()].to_vec()
    }
}

/// [`UnionBody`] of a method, or `None` when its body is not a union.
#[must_use]
pub fn name_union_body(method: &SdkMethod) -> Option<UnionBody<'_>> {
    if method.body_variants.is_empty() {
        return None;
    }
    let mut named = Named::default();
    let mut variants = Vec::new();
    let mut args: Vec<&SdkParam> = Vec::new();
    for (i, v) in method.body_variants.iter().enumerate() {
        let name =
            variant_segment(&v.object, method.body_discriminator.as_deref()).unwrap_or_else(|| format!("Variant{i}"));
        let mut own: Vec<&SdkParam> = keyword_path_params(method).iter().collect();
        own.extend(sort_fields(method.query_params.iter().chain(&v.params)));
        let (pre, post) = named.counts();
        named.add_fields(&own, &name);
        let (pre_end, post_end) = named.counts();
        for q in &own {
            if !args.iter().any(|a| a.py_name == q.py_name) {
                args.push(q);
            }
        }
        variants.push(UnionVariant {
            name,
            args: own,
            pre: pre..pre_end,
            post: post..post_end,
        });
    }
    Some(UnionBody { named, variants, args })
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
    fields_in_order(p, &sort_fields(fields.iter().copied()), ctx);
}

/// [`fields_of`] in the order given.
fn fields_in_order<F: FieldLike>(p: &mut PythonPrinter, fields: &[&F], ctx: &dyn AnnotationContext) {
    for (i, &f) in fields.iter().enumerate() {
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
    if let Some(body) = name_union_body(method) {
        emit_union_params(&mut p, &r.name, &body, here);
        return p.render();
    }
    let params = method_params(method);
    let named = name_params(&params, "");
    // The path parameters before the last lead the fields, in path order.
    let mut fields: Vec<&SdkParam> = keyword_path_params(method).iter().collect();
    fields.extend(&params);
    let ctx = ParamsContext {
        named: &named,
        qualify: String::new(),
        here,
        shadowed: fields.iter().map(|q| q.py_name.clone()).collect(),
    };
    let mut exported = vec![r.name.clone()];
    exported.extend(named.preorder().iter().map(|d| d.name().to_owned()));
    for l in all_list(&exported) {
        p.line(&l);
    }
    p.blank().blank();
    p.block(&format!("class {}(TypedDict, total=False):", r.name), |p| {
        fields_in_order(p, &fields, &ctx);
    });
    for decl in named.postorder() {
        print_decl(&mut p, decl, &ctx);
    }
    p.render()
}

/// Declares one inline object or object-union alias of a params file.
fn print_decl(p: &mut PythonPrinter, decl: Decl<'_>, ctx: &ParamsContext<'_, '_>) {
    p.blank().blank();
    match decl {
        Decl::Alias { name, union } => {
            p.import_from("typing_extensions", "TypeAlias");
            p.import_from("typing", "Union");
            let plain = WithoutUnionNames(ctx);
            // The alias leaves `None` out; a nullable field is `Optional[<alias>]`.
            let variants = match union {
                SdkType::Union { variants, .. } => union_members(variants)
                    .0
                    .into_iter()
                    .map(|v| to_annotation(v, p, &plain).into())
                    .collect(),
                _ => Vec::new(),
            };
            print_expr(
                p,
                &bracket("Union[", variants, false, "]").into(),
                &format!("{name}: TypeAlias = "),
                "",
            );
        }
        Decl::Class { name, obj } => {
            // An object that takes other keys too is its fields' `<name>Typed`, without a
            // docstring, or a dict (knock `ObjectTyped`, papr `MemoryMetadataParamTyped`).
            let extra = typed_extra(obj);
            let class = if extra.is_some() {
                format!("{name}Typed")
            } else {
                name.to_owned()
            };
            p.block(&format!("class {class}(TypedDict, total=False):"), |p| {
                let doc = if extra.is_some() {
                    Vec::new()
                } else {
                    docstring(obj.description.as_deref().unwrap_or(""), "")
                };
                for l in &doc {
                    p.line(l);
                }
                if !doc.is_empty() && !obj.fields.is_empty() {
                    p.blank();
                }
                let fields: Vec<_> = obj.fields.iter().collect();
                fields_of(p, &fields, ctx);
            });
            if let Some(extra) = extra {
                p.import_from("typing_extensions", "TypeAlias");
                p.import_from("typing", "Union");
                p.import_from("typing", "Dict");
                let values = to_annotation(extra, p, ctx);
                p.blank().blank();
                p.line(&format!("{name}: TypeAlias = Union[{class}, Dict[str, {values}]]"));
            }
        }
    }
}

/// The params file of a union body: a `TypedDict` per variant, each followed by its inline
/// objects, then `<Params>: TypeAlias = Union[...]` (imagekit `file_update_params.py`).
fn emit_union_params(p: &mut PythonPrinter, name: &str, body: &UnionBody<'_>, here: String) {
    let ctx = ParamsContext {
        named: &body.named,
        qualify: String::new(),
        here,
        shadowed: body.args.iter().map(|q| q.py_name.clone()).collect(),
    };
    let mut exported = vec![name.to_owned()];
    for (i, v) in body.variants.iter().enumerate() {
        exported.push(v.name.clone());
        exported.extend(body.preorder(i).iter().map(|d| d.name().to_owned()));
    }
    for l in all_list(&exported) {
        p.line(&l);
    }
    for (i, v) in body.variants.iter().enumerate() {
        p.blank().blank();
        p.block(&format!("class {}(TypedDict, total=False):", v.name), |p| {
            fields_in_order(p, &v.args, &ctx);
        });
        for decl in body.postorder(i) {
            print_decl(p, decl, &ctx);
        }
    }
    p.blank().blank();
    p.import_from("typing_extensions", "TypeAlias");
    p.import_from("typing", "Union");
    let variants = body.variants.iter().map(|v| v.name.clone().into()).collect();
    print_expr(
        p,
        &bracket("Union[", variants, false, "]").into(),
        &format!("{name}: TypeAlias = "),
        "",
    );
}
