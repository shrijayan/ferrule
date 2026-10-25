//! Normalisation: rewrite constructs the pinned Forge OpenAPI reader rejects into equivalents it
//! accepts, without changing what a generator sees. Applied after transforms, before annotation.
//! Pure. Every rewritten node gets an `x-ferrule-normalized` note (a list of rule names) and a
//! `FER16xx` diagnostic. See docs/forge-gaps.md.

use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::unicode::{is_letter_or_number, is_lower, is_lower_or_number, is_upper};
use crate::value::{Map, Value, js_string};
use std::collections::{HashMap, HashSet};

/// `type` array holds a non-string entry (a YAML `null`), rewritten to the string `"null"`.
pub const TYPE_NULL_LITERAL: &str = "FER1601";
/// `discriminator` is a bare property name string, rewritten to `{ propertyName }`.
pub const DISCRIMINATOR_STRING: &str = "FER1602";
/// A query parameter's `style` is not `form`, the only one Forge reads; the style is dropped.
pub const QUERY_STYLE: &str = "FER1603";
/// Fields Forge names alike get a name extension spelling out their symbols (`+1` -> `plus_1`).
pub const EQUAL_NAMES_SPELLED: &str = "FER1604";
/// Fields Forge names alike with nothing to spell out: the extra ones are numbered.
pub const EQUAL_NAMES_NUMBERED: &str = "FER1605";
/// A `$ref` into components.schemas points to nothing; replaced by an unknown schema.
pub const DANGLING_REF: &str = "FER1606";
/// `items` is a list (a tuple, draft 4); rewritten to `prefixItems`.
pub const ITEMS_TUPLE: &str = "FER1607";

pub const NORMALIZED_KEY: &str = "x-ferrule-normalized";

/// Keys under which a schema keyword appears as data rather than as a schema.
const DATA_KEYS: [&str; 5] = ["example", "examples", "enum", "const", "default"];

/// Keys whose value maps names to nodes: a name such as `default` there is not a keyword.
const NAME_MAPS: [&str; 11] = [
    "properties",
    "patternProperties",
    "$defs",
    "definitions",
    "schemas",
    "parameters",
    "responses",
    "requestBodies",
    "headers",
    "content",
    "paths",
];

fn is_data_key(k: &str) -> bool {
    DATA_KEYS.contains(&k) || k.starts_with("x-")
}

/// The words Forge names a field by (`.cache/forge/src/name.rs` `Name::new`): split at
/// characters that are neither letters nor digits and at case changes, lower-cased.
#[must_use]
pub fn forge_words(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, words: &mut Vec<String>| {
        if !word.is_empty() {
            words.push(word.to_lowercase());
        }
        word.clear();
    };
    for (i, &c) in chars.iter().enumerate() {
        if !is_letter_or_number(c) {
            flush(&mut word, &mut words);
            continue;
        }
        // `word.at(-1)` is a UTF-16 unit: an astral character there is a lone surrogate, which
        // no category test matches.
        let prev = word.chars().last().filter(|p| u32::from(*p) < 0x10000);
        let has_prev = !word.is_empty();
        if has_prev && is_upper(c) {
            let next_lower = chars.get(i + 1).is_some_and(|n| is_lower(*n));
            let split = prev.is_some_and(|p| is_lower_or_number(p) || (is_upper(p) && next_lower));
            if split {
                flush(&mut word, &mut words);
            }
        }
        word.push(c);
    }
    flush(&mut word, &mut words);
    words
}

/// `wire` with its meaningful symbols spelled out, in Forge words.
fn spelled(wire: &str) -> Vec<String> {
    let chars: Vec<char> = wire.chars().collect();
    let mut text = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '-' && (i == 0 || chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            text.push_str(" minus ");
            continue;
        }
        let word = match c {
            '+' => Some("plus"),
            '<' => Some("lt"),
            '>' => Some("gt"),
            '=' => Some("eq"),
            '!' => Some("not"),
            _ => None,
        };
        match word {
            Some(w) => {
                text.push(' ');
                text.push_str(w);
                text.push(' ');
            }
            None => text.push(c),
        }
    }
    forge_words(&text)
}

fn note(node: &mut Map, rule: &str) {
    let next = match node.get(NORMALIZED_KEY) {
        Some(Value::Array(prev)) => {
            let mut v = prev.clone();
            v.push(Value::from(rule));
            v
        }
        _ => vec![Value::from(rule)],
    };
    node.insert(NORMALIZED_KEY, Value::Array(next));
}

struct Walker<'a> {
    schemas: Option<&'a Map>,
    diagnostics: Vec<Diagnostic>,
}

impl Walker<'_> {
    fn walk(&mut self, value: &Value, path: &mut Vec<Seg>) -> Value {
        match value {
            Value::Array(a) => Value::Array(
                a.iter()
                    .enumerate()
                    .map(|(i, v)| {
                        path.push(Seg::Index(i));
                        let out = self.walk(v, path);
                        path.pop();
                        out
                    })
                    .collect(),
            ),
            Value::Object(m) => Value::Object(self.walk_object(m, path)),
            other => other.clone(),
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the rewrite rules in the order of the TypeScript walk"
    )]
    fn walk_object(&mut self, value: &Map, path: &mut Vec<Seg>) -> Map {
        let mut out = Map::new();
        for (k, v) in value {
            if let (true, Value::Object(names)) = (NAME_MAPS.contains(&k.as_str()), v) {
                let mut inner = Map::new();
                for (n, c) in names {
                    path.push(Seg::Key(k.clone()));
                    path.push(Seg::Key(n.clone()));
                    let w = self.walk(c, path);
                    path.pop();
                    path.pop();
                    inner.insert(n.clone(), w);
                }
                out.insert(k.clone(), Value::Object(inner));
            } else if is_data_key(k) {
                out.insert(k.clone(), v.clone());
            } else {
                path.push(Seg::Key(k.clone()));
                let w = self.walk(v, path);
                path.pop();
                out.insert(k.clone(), w);
            }
        }
        let at = |path: &[Seg], key: &str| {
            let mut p = path.to_vec();
            p.push(Seg::from(key));
            to_json_pointer(&p)
        };
        let ty = out.get("type").cloned();
        let discriminator = out.get("discriminator").cloned();
        let reference = out.get("$ref").cloned();
        let items = out.get("items").cloned();
        if let Some(Value::Array(items)) = items {
            out.remove("items");
            out.insert("prefixItems", Value::Array(items));
            note(&mut out, "items-tuple");
            self.diagnostics.push(Diagnostic::info(
                ITEMS_TUPLE,
                at(path, "items"),
                "`items` is a list (a draft-4 tuple); rewritten to `prefixItems`, which Forge reads as an unknown tuple",
                "if a list of one of these was meant, write `items: { anyOf: [...] }` in the spec",
            ));
        }
        if out.get("in").and_then(Value::as_str) == Some("query")
            && let Some(Value::String(style)) = out.get("style").cloned()
            && style != "form"
        {
            out.remove("style");
            note(&mut out, &format!("query-style-{style}"));
            self.diagnostics.push(Diagnostic::info(
                        QUERY_STYLE,
                        at(path, "style"),
                        format!("query parameter style \"{style}\" is not read by Forge; dropped (the generated params type is the same, the runtime serialises queries)"),
                        "nothing to do; the original style is kept in x-ferrule-normalized",
                    ));
        }
        let schema_prefix = "#/components/schemas/";
        if let Some(Value::String(r)) = &reference
            && let Some(rest) = r.strip_prefix(schema_prefix)
        {
            let name = rest.replace("~1", "/").replace("~0", "~");
            if !self.schemas.is_some_and(|s| s.contains_key(&name)) {
                out.remove("$ref");
                note(&mut out, "dangling-ref");
                self.diagnostics.push(Diagnostic::warning(
                    DANGLING_REF,
                    at(path, "$ref"),
                    format!("$ref \"{r}\" points to nothing; replaced by an unknown schema"),
                    format!(
                        "add components.schemas.{name} to the spec, or fix the $ref with an openapi.transforms update"
                    ),
                ));
            }
        }
        if let Some(Value::Array(types)) = &ty
            && types.iter().any(Value::is_null)
        {
            let fixed = types
                .iter()
                .map(|t| if t.is_null() { Value::from("null") } else { t.clone() })
                .collect();
            out.insert("type", Value::Array(fixed));
            note(&mut out, "type-null-literal");
            self.diagnostics.push(Diagnostic::info(
                TYPE_NULL_LITERAL,
                at(path, "type"),
                "`type` array contains a null literal; rewritten to the string \"null\"",
                "quote it in the spec: `type: [array, \"null\"]`",
            ));
        }
        if let Some(Value::String(d)) = &discriminator {
            let mut obj = Map::new();
            obj.insert("propertyName", Value::from(d.as_str()));
            out.insert("discriminator", Value::Object(obj));
            note(&mut out, "discriminator-string");
            self.diagnostics.push(Diagnostic::info(
                DISCRIMINATOR_STRING,
                at(path, "discriminator"),
                "`discriminator` is a string; rewritten to a Discriminator Object",
                "write `discriminator: { propertyName: <name> }` in the spec",
            ));
        }
        out
    }
}

/// A field that may need a name extension.
struct NamedField {
    wire: String,
    words: Vec<String>,
    id: usize,
}

/// Give fields Forge would name alike distinct names, as the docs/forge-gaps.md G26 row says.
/// `rename(id, snake)` writes the name extension and returns the field's pointer.
fn disambiguate(
    fields: &[NamedField],
    diagnostics: &mut Vec<Diagnostic>,
    rename: &mut dyn FnMut(usize, &str) -> String,
) {
    let key = |w: &[String]| w.join(" ");
    let mut taken: HashSet<String> = fields.iter().map(|f| key(&f.words)).collect();
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<&NamedField>> = HashMap::new();
    for f in fields {
        let k = key(&f.words);
        if !groups.contains_key(&k) {
            order.push(k.clone());
        }
        groups.entry(k).or_default().push(f);
    }
    for k in order {
        let group = &groups[&k];
        if group.len() < 2 {
            continue;
        }
        let mut left: Vec<&NamedField> = Vec::new();
        for f in group {
            let words = spelled(&f.wire);
            if taken.contains(&key(&words)) {
                left.push(f);
                continue;
            }
            taken.insert(key(&words));
            let snake = words.join("_");
            let pointer = rename(f.id, &snake);
            diagnostics.push(Diagnostic::info(
                EQUAL_NAMES_SPELLED,
                pointer,
                format!("\"{}\" has the same Forge name as another field; named {snake}", f.wire),
                "set x-fern-property-name or x-fern-parameter-name to choose the name",
            ));
        }
        if left.len() < 2 {
            continue;
        }
        let keep = left
            .iter()
            .find(|f| f.wire == f.words.join("_"))
            .copied()
            .unwrap_or(left[0]);
        let mut n = 2usize;
        for f in &left {
            if std::ptr::eq(*f, keep) {
                continue;
            }
            let numbered = |n: usize| {
                let mut w = f.words.clone();
                w.push(n.to_string());
                w
            };
            while taken.contains(&key(&numbered(n))) {
                n += 1;
            }
            let words = numbered(n);
            taken.insert(key(&words));
            let snake = words.join("_");
            let pointer = rename(f.id, &snake);
            diagnostics.push(Diagnostic::warning(
                EQUAL_NAMES_NUMBERED,
                pointer,
                format!(
                    "\"{}\" and \"{}\" have the same Forge name; \"{}\" is named {snake}",
                    f.wire, keep.wire, f.wire
                ),
                "set x-fern-property-name or x-fern-parameter-name to choose the name",
            ));
        }
    }
}

const PROPERTY_NAME: &str = "x-fern-property-name";
const PARAMETER_NAME: &str = "x-fern-parameter-name";

fn name_of(node: &Value, wire: &str, ext: &str) -> Vec<String> {
    match node.get(ext) {
        Some(Value::String(own)) => forge_words(own),
        _ => forge_words(wire),
    }
}

/// Disambiguate property names everywhere, deepest first.
fn properties(value: &mut Value, path: &mut Vec<Seg>, diagnostics: &mut Vec<Diagnostic>) {
    match value {
        Value::Array(a) => {
            for (i, v) in a.iter_mut().enumerate() {
                path.push(Seg::Index(i));
                properties(v, path, diagnostics);
                path.pop();
            }
        }
        Value::Object(m) => {
            for (k, v) in m.iter_mut() {
                if let (true, Value::Object(names)) = (NAME_MAPS.contains(&k.as_str()), &mut *v) {
                    for (n, c) in names.iter_mut() {
                        path.push(Seg::Key(k.clone()));
                        path.push(Seg::Key(n.clone()));
                        properties(c, path, diagnostics);
                        path.pop();
                        path.pop();
                    }
                } else if !is_data_key(k) {
                    path.push(Seg::Key(k.clone()));
                    properties(v, path, diagnostics);
                    path.pop();
                }
            }
            let Some(Value::Object(props)) = m.get_mut("properties") else {
                return;
            };
            let fields: Vec<NamedField> = props
                .iter()
                .enumerate()
                .filter(|(_, (_, p))| p.is_object())
                .map(|(id, (wire, p))| NamedField {
                    wire: wire.clone(),
                    words: name_of(p, wire, PROPERTY_NAME),
                    id,
                })
                .collect();
            let wires: Vec<String> = props.keys().cloned().collect();
            let base = path.clone();
            disambiguate(&fields, diagnostics, &mut |id, snake| {
                let wire = &wires[id];
                if let Some(Value::Object(node)) = props.get_mut(wire) {
                    node.insert(PROPERTY_NAME, Value::from(snake));
                    note(node, "equal-names");
                }
                let mut p = base.clone();
                p.push(Seg::from("properties"));
                p.push(Seg::from(wire.as_str()));
                to_json_pointer(&p)
            });
        }
        _ => {}
    }
}

const METHODS: [&str; 8] = ["get", "put", "post", "delete", "options", "head", "patch", "trace"];

fn resolve_param<'a>(p: &'a Value, component_params: Option<&'a Value>) -> Option<&'a Value> {
    const PREFIX: &str = "#/components/parameters/";
    match p.get("$ref") {
        Some(Value::String(r)) if p.is_object() && r.starts_with(PREFIX) => match component_params {
            Some(Value::Object(cp)) => cp.get(&r[PREFIX.len()..]),
            _ => None,
        },
        _ => Some(p),
    }
}

fn where_key(r: Option<&Value>) -> String {
    match r {
        Some(Value::Object(m)) => {
            let s = |v: Option<&Value>| v.map_or_else(|| "undefined".to_owned(), js_string);
            format!("{}:{}", s(m.get("in")), s(m.get("name")))
        }
        _ => String::new(),
    }
}

/// A parameter declared for an operation: the node (resolved), and its index in the
/// operation's own list (`None`: shared from the path item).
struct Declared {
    resolved: Option<Value>,
    index: Option<usize>,
}

fn parameters(out: &mut Value, diagnostics: &mut Vec<Diagnostic>) {
    let component_params = out.get("components").and_then(|c| c.get("parameters")).cloned();
    let routes: Vec<String> = match out.get("paths") {
        Some(Value::Object(p)) => p.keys().cloned().collect(),
        _ => return,
    };
    for route in routes {
        let Some(Value::Object(item)) = out.get("paths").and_then(|p| p.get(&route)) else {
            continue;
        };
        let shared: Vec<Value> = match item.get("parameters") {
            Some(Value::Array(a)) => a.clone(),
            _ => Vec::new(),
        };
        for method in METHODS {
            let Some(Value::Object(op)) = out.get("paths").and_then(|p| p.get(&route)).and_then(|i| i.get(method))
            else {
                continue;
            };
            let mut own: Vec<Value> = match op.get("parameters") {
                Some(Value::Array(a)) => a.clone(),
                _ => Vec::new(),
            };
            let cp = component_params.as_ref();
            let own_keys: HashSet<String> = own.iter().map(|p| where_key(resolve_param(p, cp))).collect();
            let mut declared: Vec<Declared> = shared
                .iter()
                .filter(|p| !own_keys.contains(&where_key(resolve_param(p, cp))))
                .map(|p| Declared {
                    resolved: resolve_param(p, cp).cloned(),
                    index: None,
                })
                .collect();
            declared.extend(own.iter().enumerate().map(|(i, p)| Declared {
                resolved: resolve_param(p, cp).cloned(),
                index: Some(i),
            }));
            let mut dirty = false;
            for locations in [&["path", "query"][..], &["header"][..]] {
                let fields: Vec<NamedField> = declared
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| match &d.resolved {
                        Some(r @ Value::Object(m)) => {
                            let _ = r;
                            locations.contains(&m.get("in").map_or_else(|| "undefined".to_owned(), js_string).as_str())
                        }
                        _ => false,
                    })
                    .filter_map(|(id, d)| {
                        let r = d.resolved.as_ref()?;
                        let wire = r.get("name").and_then(Value::as_str)?;
                        Some(NamedField {
                            wire: wire.to_owned(),
                            words: name_of(r, wire, PARAMETER_NAME),
                            id,
                        })
                    })
                    .collect();
                disambiguate(&fields, diagnostics, &mut |id, snake| {
                    let d = &declared[id];
                    let mut node = d
                        .resolved
                        .as_ref()
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    node.insert(PARAMETER_NAME, Value::from(snake));
                    note(&mut node, "equal-names");
                    // An inline copy at the operation replaces a shared or $ref'd declaration.
                    let at = d.index.unwrap_or(own.len());
                    if at < own.len() {
                        own[at] = Value::Object(node);
                    } else {
                        own.push(Value::Object(node));
                    }
                    dirty = true;
                    to_json_pointer(&[
                        Seg::from("paths"),
                        Seg::from(route.as_str()),
                        Seg::from(method),
                        Seg::from("parameters"),
                        Seg::Index(at),
                    ])
                });
            }
            if dirty
                && let Some(Value::Object(op)) = out
                    .as_object_mut()
                    .and_then(|o| o.get_mut("paths"))
                    .and_then(|p| p.as_object_mut())
                    .and_then(|p| p.get_mut(&route))
                    .and_then(|i| i.as_object_mut())
                    .and_then(|i| i.get_mut(method))
            {
                op.insert("parameters", Value::Array(own));
            }
        }
    }
}

/// Normalise a copy of `spec`. Pure.
#[must_use]
pub fn normalize(spec: &Value) -> (Value, Vec<Diagnostic>) {
    let schemas = spec.get("components").and_then(|c| match c {
        Value::Object(m) => match m.get("schemas") {
            Some(Value::Object(s)) => Some(s),
            _ => None,
        },
        _ => None,
    });
    let mut walker = Walker {
        schemas,
        diagnostics: Vec::new(),
    };
    let mut out = walker.walk(spec, &mut Vec::new());
    let mut diagnostics = walker.diagnostics;
    properties(&mut out, &mut Vec::new(), &mut diagnostics);
    parameters(&mut out, &mut diagnostics);
    (out, diagnostics)
}
