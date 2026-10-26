//! The one type mapper: a field's PHP type ([`PType`]) and its spellings — the native type of a
//! property, of a constructor or setter parameter, the phpdoc types of `@var`, `@param` and the
//! `@phpstan-type` shapes, and the converter arguments of `#[Required]` / `#[Optional]`.
//!
//! Every class name goes through [`Imports`], which decides per file whether a class is imported
//! (`use`), named relative to an imported class, or fully qualified.

use ferrule_sdk_model::model::Literal;

/// A generated class, enum or union, by its path below the root namespace:
/// `["Files", "File", "AITag"]` for `Imagekit\Files\File\AITag`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassPath {
    pub segs: Vec<String>,
}

impl ClassPath {
    #[must_use]
    pub fn new(segs: Vec<String>) -> Self {
        Self { segs }
    }

    /// The last segment: the class name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.segs.last().map_or("", String::as_str)
    }

    /// A class nested in this one (`File` -> `File\AITag`).
    #[must_use]
    pub fn child(&self, name: &str) -> Self {
        let mut segs = self.segs.clone();
        segs.push(name.to_owned());
        Self { segs }
    }

    /// The namespace below the root: `["Files", "File"]` for `Files\File\AITag`.
    #[must_use]
    pub fn namespace(&self) -> &[String] {
        &self.segs[..self.segs.len().saturating_sub(1)]
    }

    /// The file below `src/`: `Files/File/AITag.php`.
    #[must_use]
    pub fn file(&self) -> String {
        format!("{}.php", self.segs.join("/"))
    }

    /// The fully qualified name without the leading backslash: `Imagekit\Files\File\AITag`.
    #[must_use]
    pub fn fqn(&self, root: &str) -> String {
        if self.segs.is_empty() {
            root.to_owned()
        } else {
            format!("{root}\\{}", self.segs.join("\\"))
        }
    }
}

/// A PHP type of a field or parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum PType {
    String,
    Int,
    Float,
    Bool,
    /// `date-time`: `\DateTimeInterface`.
    DateTime,
    /// `date`: a string (`2019-12-27`).
    Date,
    /// Any value.
    Mixed,
    /// A file upload (`binary`).
    File,
    List(Box<PType>),
    /// An object whose values have this type.
    Map(Box<PType>),
    /// A class using `SdkModel`.
    Model(ClassPath),
    /// A backed enum, and the type of its values.
    Enum(ClassPath, Backing),
    /// A class using `SdkUnion`, with its variants' types.
    Union(ClassPath, Vec<PType>),
    /// `inner` or null, inside a list, map or union (a field's own nullability is
    /// [`crate::tree::PField::nullable`]).
    Nullable(Box<PType>),
    /// One literal value (`'S3'`).
    Const(Literal),
    /// A reference to a model on a reference cycle: phpdoc and the native type say `mixed`
    /// (phpstan cannot resolve a recursive type alias); the converter keeps the class.
    Rec(Box<PType>),
}

/// The type of a backed enum's values: `enum Status: string`, `enum Level: int`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backing {
    String,
    Int,
}

impl Backing {
    /// The backing of an enum whose first member is `first`: `int` for a number.
    #[must_use]
    pub fn of(first: Option<&Literal>) -> Self {
        match first {
            Some(Literal::Number(_)) => Self::Int,
            _ => Self::String,
        }
    }

    #[must_use]
    pub fn native(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
        }
    }
}

/// A PHP literal: `'S3'`, `1`, `true`.
#[must_use]
pub fn literal(v: &Literal) -> String {
    match v {
        Literal::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        Literal::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                #[allow(clippy::cast_possible_truncation)]
                let i = *n as i64;
                i.to_string()
            } else {
                n.to_string()
            }
        }
        Literal::Boolean(b) => b.to_string(),
    }
}

/// The phpstan import of a type alias another class declares: `XShape from \Imagekit\X`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TypeImport {
    /// `XShape`.
    name: String,
    /// The class declaring it.
    from: ClassPath,
    /// The name it is used by in this file (`SchemaShape1` when two imports share a name).
    alias: String,
}

/// The names one file uses: its `use` imports and its `@phpstan-import-type`s.
///
/// A class is spelled by its short name and imported, unless the short name would mean another
/// class here (the file's own class, a class of its namespace, or one imported already): then
/// it is fully qualified, as Stainless writes it, and php-cs-fixer's `fully_qualified_strict_types`
/// imports or shortens it where it can (not in `@phpstan-type` lines).
#[derive(Debug, Clone)]
pub struct Imports {
    pub root: String,
    /// The file's own namespace below the root.
    pub namespace: Vec<String>,
    /// The file's own class, which needs no import.
    pub own: Option<ClassPath>,
    /// The names of every class in the file's namespace.
    siblings: Vec<String>,
    /// Imported classes: (short name, fully qualified name), in first-use order.
    uses: Vec<(String, String)>,
    /// Union `...Variants` imports, then `...Shape` imports, then the other imports.
    variants: Vec<TypeImport>,
    shapes: Vec<TypeImport>,
    other: Vec<TypeImport>,
}

/// Which list a phpstan type import goes to (the order the header lists them in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Variants,
    Shape,
    Other,
}

impl Imports {
    #[must_use]
    pub fn new(root: &str, namespace: &[String], own: Option<ClassPath>, siblings: Vec<String>) -> Self {
        Self {
            root: root.to_owned(),
            namespace: namespace.to_vec(),
            own,
            siblings,
            uses: Vec::new(),
            variants: Vec::new(),
            shapes: Vec::new(),
            other: Vec::new(),
        }
    }

    /// The imports of a file outside the root namespace (the tests, in `Tests\...`).
    #[must_use]
    pub fn outside(root: &str) -> Self {
        Self::new(root, &["\u{0}".to_owned()], None, Vec::new())
    }

    /// Imports a class outside the root namespace (`Psr\Http\Message\...`) and returns its short
    /// name.
    pub fn external(&mut self, fqn: &str) -> String {
        let short = fqn.rsplit('\\').next().unwrap_or(fqn).to_owned();
        if !self.uses.iter().any(|(_, f)| f == fqn) {
            self.uses.push((short.clone(), fqn.to_owned()));
        }
        short
    }

    /// Imports a runtime class (`Core\Concerns\SdkModel`) and returns its short name.
    pub fn runtime(&mut self, path: &[&str]) -> String {
        let c = ClassPath::new(path.iter().map(|s| (*s).to_owned()).collect());
        self.class(&c)
    }

    /// Whether `short` means another class than `fqn` in this file.
    fn taken(&self, short: &str, fqn: &str) -> bool {
        self.own.as_ref().is_some_and(|o| o.name().eq_ignore_ascii_case(short))
            || self.siblings.iter().any(|s| s.eq_ignore_ascii_case(short))
            || self.uses.iter().any(|(s, f)| s.eq_ignore_ascii_case(short) && f != fqn)
    }

    /// The name a generated class is spelled with (see [`Imports`]).
    pub fn class(&mut self, c: &ClassPath) -> String {
        let fqn = c.fqn(&self.root);
        let short = c.name().to_owned();
        if self.own.as_ref() == Some(c)
            || c.namespace() == self.namespace.as_slice()
            || self.uses.iter().any(|(_, f)| *f == fqn)
        {
            return short;
        }
        if self.taken(&short, &fqn) {
            // Relative to an imported class it is nested in, else fully qualified.
            for i in (1..c.segs.len()).rev() {
                let parent = ClassPath::new(c.segs[..i].to_vec());
                let pfqn = parent.fqn(&self.root);
                if self.uses.iter().any(|(_, f)| *f == pfqn) {
                    return format!("{}\\{}", parent.name(), c.segs[i..].join("\\"));
                }
            }
            return format!("\\{fqn}");
        }
        self.uses.push((short.clone(), fqn));
        short
    }

    /// The name of a phpstan type alias `name` declared by `from` (imported unless `from` is the
    /// file's own class; numbered when another alias here has the name).
    pub fn type_alias(&mut self, name: &str, from: &ClassPath, kind: ImportKind) -> String {
        if self.own.as_ref() == Some(from) {
            return name.to_owned();
        }
        let all = self.variants.iter().chain(&self.shapes).chain(&self.other);
        if let Some(t) = all.clone().find(|t| t.name == name && t.from == *from) {
            return t.alias.clone();
        }
        let own = self.own.as_ref().is_some_and(|o| {
            [format!("{}Shape", o.name()), format!("{}Variants", o.name())].contains(&name.to_owned())
        });
        let count = all.filter(|t| t.name == name).count() + usize::from(own);
        let alias = if count == 0 {
            name.to_owned()
        } else {
            format!("{name}{count}")
        };
        let t = TypeImport {
            name: name.to_owned(),
            from: from.clone(),
            alias: alias.clone(),
        };
        match kind {
            ImportKind::Variants => self.variants.push(t),
            ImportKind::Shape => self.shapes.push(t),
            ImportKind::Other => self.other.push(t),
        }
        alias
    }

    /// The `use` lines, sorted as php-cs-fixer sorts them.
    #[must_use]
    pub fn use_lines(&self) -> Vec<String> {
        let mut out: Vec<String> = self.uses.iter().map(|(_, f)| format!("use {f};")).collect();
        out.sort_by_key(|l| l.replace('\\', " ").to_ascii_lowercase());
        out
    }

    /// The `@phpstan-import-type` lines (without the ` * ` prefix).
    #[must_use]
    pub fn type_import_lines(&self) -> Vec<String> {
        self.variants
            .iter()
            .chain(&self.shapes)
            .chain(&self.other)
            .map(|t| {
                let from = t.from.fqn(&self.root);
                if t.alias == t.name {
                    format!("@phpstan-import-type {} from \\{from}", t.name)
                } else {
                    format!("@phpstan-import-type {} from \\{from} as {}", t.name, t.alias)
                }
            })
            .collect()
    }
}

/// Joins type alternatives with `|`, dropping repeats (first one wins).
#[must_use]
pub fn join_unique(parts: Vec<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    for p in parts {
        for alt in split_top(&p) {
            if !out.contains(&alt) {
                out.push(alt);
            }
        }
    }
    out.join("|")
}

/// The top-level `|` alternatives of a type (not inside `<...>` or `{...}`).
fn split_top(t: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in t.chars() {
        match c {
            '<' | '{' | '(' => depth += 1,
            '>' | '}' | ')' => depth -= 1,
            '|' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out
}

impl PType {
    /// The alias of a union's variants type (`XVariants`) and of a model's or union's shape
    /// (`XShape`).
    fn alias(c: &ClassPath, suffix: &str) -> String {
        format!("{}{suffix}", c.name())
    }

    /// The native type of a property or a value inside a union, without nullability.
    pub fn native(&self, imp: &mut Imports) -> String {
        match self {
            // A file is stored as its contents; an enum as its value.
            Self::String | Self::Date | Self::File => "string".to_owned(),
            Self::Enum(_, b) => b.native().to_owned(),
            Self::Int => "int".to_owned(),
            Self::Float => "float".to_owned(),
            Self::Bool => "bool".to_owned(),
            Self::DateTime => "\\DateTimeInterface".to_owned(),
            Self::Mixed | Self::Rec(_) => "mixed".to_owned(),
            Self::List(_) | Self::Map(_) => "array".to_owned(),
            Self::Model(c) => imp.class(c),
            Self::Const(v) => match v {
                Literal::String(_) => "string".to_owned(),
                Literal::Number(n) if n.fract() == 0.0 => "int".to_owned(),
                Literal::Number(_) => "float".to_owned(),
                Literal::Boolean(_) => "bool".to_owned(),
            },
            Self::Union(_, variants) => join_unique(variants.iter().map(|v| v.native(imp)).collect()),
            Self::Nullable(inner) => format!("{}|null", inner.native(imp)),
        }
    }

    /// The native type of a constructor or setter parameter: a model also takes its shape
    /// (`array`), an enum its value (`string`, `int`).
    pub fn param_native(&self, imp: &mut Imports) -> String {
        match self {
            Self::Model(c) => format!("{}|array", imp.class(c)),
            Self::File => format!("string|{}", imp.runtime(&["Core", "FileParam"])),
            Self::Enum(c, b) => format!("{}|{}", imp.class(c), b.native()),
            Self::Union(_, variants) => join_unique(variants.iter().map(|v| v.param_native(imp)).collect()),
            Self::Nullable(inner) => format!("{}|null", inner.param_native(imp)),
            Self::Rec(_) => "mixed".to_owned(),
            other => other.native(imp),
        }
    }

    /// Whether [`Self::native`] is one type, so that `?T` spells it nullable.
    fn single(native: &str) -> bool {
        !native.contains('|') && native != "mixed"
    }

    /// `?T`, or `T|U|null`, or `mixed`.
    #[must_use]
    pub fn nullable_native(native: &str) -> String {
        if native == "mixed" || native.ends_with("|null") {
            native.to_owned()
        } else if Self::single(native) {
            format!("?{native}")
        } else {
            format!("{native}|null")
        }
    }

    /// The type a property's `@var` declares: what the native type does not say. `None` when the
    /// native type says it all.
    pub fn var_doc(&self, imp: &mut Imports) -> Option<String> {
        match self {
            Self::List(_) | Self::Map(_) | Self::Enum(..) | Self::Union(..) | Self::Const(_) => Some(self.doc(imp)),
            _ => None,
        }
    }

    /// The phpdoc type of a value as stored (`list<AITag>`, `value-of<Status>`, `XVariants`).
    pub fn doc(&self, imp: &mut Imports) -> String {
        match self {
            Self::List(t) => format!("list<{}>", t.doc(imp)),
            Self::Map(t) => format!("array<string,{}>", t.doc(imp)),
            Self::Enum(c, _) => format!("value-of<{}>", imp.class(c)),
            Self::Union(c, _) => imp.type_alias(&Self::alias(c, "Variants"), c, ImportKind::Variants),
            Self::Const(v) => literal(v),
            Self::Nullable(t) => format!("{}|null", t.doc(imp)),
            Self::Rec(_) => "mixed".to_owned(),
            other => other.native(imp),
        }
    }

    /// The phpdoc type a parameter takes: a model or its shape, an enum or its value.
    pub fn param_doc(&self, imp: &mut Imports) -> String {
        match self {
            Self::List(t) => format!("list<{}>", t.param_doc(imp)),
            Self::Map(t) => format!("array<string,{}>", t.param_doc(imp)),
            Self::Model(c) => {
                let name = imp.class(c);
                let shape = imp.type_alias(&Self::alias(c, "Shape"), c, ImportKind::Shape);
                format!("{name}|{shape}")
            }
            Self::Enum(c, _) => {
                let name = imp.class(c);
                format!("{name}|value-of<{name}>")
            }
            Self::Union(c, _) => imp.type_alias(&Self::alias(c, "Shape"), c, ImportKind::Shape),
            Self::File => self.param_native(imp),
            Self::Nullable(t) => format!("{}|null", t.param_doc(imp)),
            Self::Rec(_) => "mixed".to_owned(),
            other => other.doc(imp),
        }
    }

    /// The type in a `@phpstan-type ...Shape = array{...}` entry, nullable (`?:` or `nil`) when
    /// `nullable`: one type is followed by `|null`, several are preceded by `null|`.
    pub fn shape_doc(&self, imp: &mut Imports, nullable: bool) -> String {
        let t = self.param_doc(imp);
        if !nullable || t == "mixed" {
            t
        } else if split_top(&t).len() == 1 {
            format!("{t}|null")
        } else {
            format!("null|{t}")
        }
    }

    /// The converter arguments of `#[Required]` / `#[Optional]` beyond the wire name:
    /// `list: 'string'`, `map: AITag::class`, `enum: Status::class`, `union: X::class`.
    pub fn attr_args(&self, imp: &mut Imports) -> Option<String> {
        match self {
            Self::List(t) => Some(format!("list: {}", t.converter(imp))),
            Self::Map(t) => Some(format!("map: {}", t.converter(imp))),
            Self::Enum(c, _) => Some(format!("enum: {}::class", imp.class(c))),
            Self::Union(c, variants) if variants.iter().any(Self::needs_converter) => {
                Some(format!("union: {}::class", imp.class(c)))
            }
            Self::Rec(t) => match &**t {
                Self::Union(c, _) => Some(format!("union: {}::class", imp.class(c))),
                other => other.attr_args(imp),
            },
            _ => None,
        }
    }

    /// Whether a union variant needs the union's converter (the native type cannot coerce it).
    fn needs_converter(&self) -> bool {
        !matches!(
            self,
            Self::String | Self::Date | Self::Int | Self::Float | Self::Bool | Self::Mixed | Self::Const(_)
        )
    }

    /// The phpdoc type a method returns: a union's variants spelled out (`list<File|Folder>`).
    pub fn return_doc(&self, imp: &mut Imports) -> String {
        match self {
            Self::List(t) => format!("list<{}>", t.return_doc(imp)),
            Self::Map(t) => format!("array<string,{}>", t.return_doc(imp)),
            Self::Union(_, variants) => join_unique(variants.iter().map(|v| v.return_doc(imp)).collect()),
            Self::Model(c) => imp.class(c),
            other => other.doc(imp),
        }
    }

    /// The converter a raw service passes as `convert:` (`File::class`, `new ListOf(...)`).
    pub fn response_converter(&self, imp: &mut Imports) -> String {
        match self {
            Self::List(t) => format!(
                "new {}({})",
                imp.runtime(&["Core", "Conversion", "ListOf"]),
                t.converter(imp)
            ),
            Self::Map(t) => format!(
                "new {}({})",
                imp.runtime(&["Core", "Conversion", "MapOf"]),
                t.converter(imp)
            ),
            other => other.converter(imp),
        }
    }

    /// The converter of a list item or map value: `'string'`, `AITag::class`,
    /// `\DateTimeInterface::class` (`'DateTimeInterface'`, the name `Conversion` matches).
    pub fn converter(&self, imp: &mut Imports) -> String {
        match self {
            Self::Model(c) | Self::Enum(c, _) | Self::Union(c, _) => format!("{}::class", imp.class(c)),
            Self::DateTime => "\\DateTimeInterface::class".to_owned(),
            Self::Nullable(t) | Self::Rec(t) => t.converter(imp),
            other => format!("'{}'", other.native(imp)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imports() -> Imports {
        Imports::new(
            "Imagekit",
            &["Files".to_owned()],
            Some(ClassPath::new(vec!["Files".into(), "File".into()])),
            vec!["File".into(), "Folder".into(), "Type".into()],
        )
    }

    #[test]
    fn imports_resolve_conflicts() {
        let mut imp = imports();
        let a = ClassPath::new(vec!["Files".into(), "File".into(), "AITag".into()]);
        let b = ClassPath::new(vec!["Files".into(), "Folder".into(), "Type".into()]);
        let folder = ClassPath::new(vec!["Files".into(), "Folder".into()]);
        assert_eq!(imp.class(&a), "AITag");
        assert_eq!(imp.class(&folder), "Folder");
        // A class of the namespace has the name.
        assert_eq!(imp.class(&b), "\\Imagekit\\Files\\Folder\\Type");
        assert_eq!(imp.use_lines(), ["use Imagekit\\Files\\File\\AITag;"]);
        let c = ClassPath::new(vec!["Other".into(), "AITag".into()]);
        assert_eq!(imp.class(&c), "\\Imagekit\\Other\\AITag");
    }

    #[test]
    fn type_aliases_are_numbered() {
        let mut imp = imports();
        let a = ClassPath::new(vec!["A".into(), "Schema".into()]);
        let b = ClassPath::new(vec!["B".into(), "Schema".into()]);
        assert_eq!(imp.type_alias("SchemaShape", &a, ImportKind::Shape), "SchemaShape");
        assert_eq!(imp.type_alias("SchemaShape", &b, ImportKind::Shape), "SchemaShape1");
        assert_eq!(
            imp.type_import_lines()[1],
            "@phpstan-import-type SchemaShape from \\Imagekit\\B\\Schema as SchemaShape1"
        );
        let nested = ClassPath::new(vec!["Files".into(), "File".into(), "File".into()]);
        assert_eq!(imp.type_alias("FileShape", &nested, ImportKind::Shape), "FileShape1");
    }

    #[test]
    fn joins_types_without_repeats() {
        assert_eq!(
            join_unique(vec!["A|array".into(), "B|array".into(), "string".into()]),
            "A|array|B|string"
        );
    }
}
