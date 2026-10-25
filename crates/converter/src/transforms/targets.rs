//! Resolving `target`, `to` and `from` arguments, and editing the document by path.

use super::jsonpath::{JsonPathError, Segment, Selector, evaluate_segments, parse_json_path, query};
use crate::Diagnostic;
use crate::diagnostic::Seg;
use crate::schema::json_schema::deep_equal;
use crate::value::{Value, cmp_utf16, quote};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Write as _;

/// One entry of a `target`/`to` argument: a JSONPath or a `matches_schema` query.
#[derive(Debug, Clone)]
pub enum TargetEntry {
    Path(String),
    MatchesSchema {
        matches_schema: String,
        ignore: Option<Ignore>,
    },
}

/// `ignore`: one path or a list.
#[derive(Debug, Clone)]
pub enum Ignore {
    One(String),
    Many(Vec<String>),
}

/// A `target`/`to` argument: one path, or a list of entries.
#[derive(Debug, Clone)]
pub enum Target {
    One(String),
    List(Vec<TargetEntry>),
}

impl Target {
    /// Read a target already checked against the args schema.
    #[must_use]
    pub fn from_value(v: &Value) -> Target {
        match v {
            Value::Array(items) => Target::List(
                items
                    .iter()
                    .map(|e| match e {
                        Value::Object(m) => TargetEntry::MatchesSchema {
                            matches_schema: m.get("matches_schema").and_then(Value::as_str).unwrap_or("").to_owned(),
                            ignore: match m.get("ignore") {
                                Some(Value::String(s)) => Some(Ignore::One(s.clone())),
                                Some(Value::Array(a)) => Some(Ignore::Many(
                                    a.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
                                )),
                                _ => None,
                            },
                        },
                        other => TargetEntry::Path(other.as_str().unwrap_or("").to_owned()),
                    })
                    .collect(),
            ),
            other => Target::One(other.as_str().unwrap_or("").to_owned()),
        }
    }
}

/// An error diagnostic (`error()`).
#[must_use]
pub fn error(
    code: &str,
    pointer: impl Into<String>,
    message: impl Into<String>,
    suggestion: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(code, pointer, message, suggestion)
}

fn syntax_error(pointer: &str, path: &str, e: &JsonPathError) -> Diagnostic {
    error(
        "FER1202",
        pointer,
        format!("Invalid or unsupported JSONPath \"{path}\": {}", e.0),
        "Write the path in the supported JSONPath Plus subset, e.g. $.components.schemas.User or $.paths..parameters[?(@.name == \"id\")].",
    )
}

/// Run a JSONPath step, turning a [`JsonPathError`] into a FER1202 diagnostic.
pub fn try_path<T>(
    pointer: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
    r: Result<T, JsonPathError>,
) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            diagnostics.push(syntax_error(pointer, path, &e));
            None
        }
    }
}

/// FER1201: the target matches nothing.
#[must_use]
pub fn no_match(pointer: &str, path: &str) -> Diagnostic {
    error(
        "FER1201",
        pointer,
        format!("Target \"{path}\" matches nothing in the OpenAPI spec."),
        "Check the path against the current spec; if the spec was fixed upstream, delete this transform.",
    )
}

/// Every node (in document order) that is an object deep-equal to `template`, not ignored.
fn equal_nodes(
    value: &Value,
    template: &Value,
    ignored: &HashSet<Vec<Seg>>,
    path: &mut Vec<Seg>,
    out: &mut Vec<Vec<Seg>>,
) {
    if value.is_object() && deep_equal(value, template) && !ignored.contains(path.as_slice()) {
        out.push(path.clone());
    }
    match value {
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                path.push(Seg::Index(i));
                equal_nodes(v, template, ignored, path, out);
                path.pop();
            }
        }
        Value::Object(m) => {
            for (k, v) in m {
                path.push(Seg::Key(k.clone()));
                equal_nodes(v, template, ignored, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn matches_schema(
    root: &Value,
    template_path: &str,
    ignore: Option<&Ignore>,
    pointer: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Vec<Seg>> {
    let at = format!("{pointer}/matches_schema");
    let Some(template) = try_path(&at, template_path, diagnostics, query(root, template_path)) else {
        return Vec::new();
    };
    if template.len() != 1 {
        diagnostics.push(error(
            "FER1207",
            at,
            format!(
                "matches_schema \"{template_path}\" must match exactly one node, found {}.",
                template.len()
            ),
            "Point matches_schema at the single template schema, e.g. $.components.schemas.ErrorTemplate.",
        ));
        return Vec::new();
    }
    let only = template[0].value;
    let ignore_paths: Vec<(String, String)> = match ignore {
        None => Vec::new(),
        Some(Ignore::One(p)) => vec![(p.clone(), format!("{pointer}/ignore"))],
        Some(Ignore::Many(list)) => list
            .iter()
            .enumerate()
            .map(|(i, p)| (p.clone(), format!("{pointer}/ignore/{i}")))
            .collect(),
    };
    let mut ignored: HashSet<Vec<Seg>> = HashSet::new();
    for (path, at) in &ignore_paths {
        let found = try_path(at, path, diagnostics, query(root, path)).unwrap_or_default();
        for m in found {
            ignored.insert(m.path);
        }
    }
    let mut out = Vec::new();
    equal_nodes(root, only, &ignored, &mut Vec::new(), &mut out);
    if out.is_empty() {
        diagnostics.push(no_match(pointer, &format!("matches_schema {template_path}")));
    }
    out
}

fn entries(target: &Target, pointer: &str) -> Vec<(TargetEntry, String)> {
    match target {
        Target::One(p) => vec![(TargetEntry::Path(p.clone()), pointer.to_owned())],
        Target::List(list) => list
            .iter()
            .enumerate()
            .map(|(i, e)| (e.clone(), format!("{pointer}/{i}")))
            .collect(),
    }
}

fn dedupe(matches: Vec<Vec<Seg>>) -> Vec<Vec<Seg>> {
    let mut seen = HashSet::new();
    matches.into_iter().filter(|m| seen.insert(m.clone())).collect()
}

/// Matched paths and the diagnostics found resolving them.
#[derive(Debug, Default)]
pub struct Resolved {
    pub matches: Vec<Vec<Seg>>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Resolve a `target` argument to existing nodes. Every entry must match something.
#[must_use]
pub fn resolve_targets(root: &Value, target: &Target, pointer: &str) -> Resolved {
    let mut diagnostics = Vec::new();
    let mut matches = Vec::new();
    for (entry, at) in entries(target, pointer) {
        match entry {
            TargetEntry::MatchesSchema {
                matches_schema: m,
                ignore,
            } => {
                matches.extend(matches_schema(root, &m, ignore.as_ref(), &at, &mut diagnostics));
            }
            TargetEntry::Path(path) => {
                let Some(found) = try_path(&at, &path, &mut diagnostics, query(root, &path)) else {
                    continue;
                };
                if found.is_empty() {
                    diagnostics.push(no_match(&at, &path));
                }
                matches.extend(found.into_iter().map(|m| m.path));
            }
        }
    }
    Resolved {
        matches: dedupe(matches),
        diagnostics,
    }
}

/// Resolve a destination (`to`) argument. A path ending in a plain name or index addresses that
/// child of every node its prefix selects, whether or not the child exists yet (the parent must).
/// Other paths, and `matches_schema` entries, address the existing nodes they select.
#[must_use]
pub fn resolve_destinations(root: &Value, target: &Target, pointer: &str) -> Resolved {
    let mut diagnostics = Vec::new();
    let mut matches = Vec::new();
    for (entry, at) in entries(target, pointer) {
        match entry {
            TargetEntry::MatchesSchema {
                matches_schema: m,
                ignore,
            } => {
                matches.extend(matches_schema(root, &m, ignore.as_ref(), &at, &mut diagnostics));
            }
            TargetEntry::Path(path) => {
                let Some(segments) = try_path(&at, &path, &mut diagnostics, parse_json_path(&path)) else {
                    continue;
                };
                let found = destinations(root, &segments);
                if found.is_empty() {
                    diagnostics.push(error(
                        "FER1208",
                        &at,
                        format!("Destination \"{path}\" has no existing parent object or array."),
                        "Create the parent first (for example with an earlier append or copy), or fix the path.",
                    ));
                }
                matches.extend(found);
            }
        }
    }
    Resolved {
        matches: dedupe(matches),
        diagnostics,
    }
}

fn destinations(root: &Value, segments: &[Segment]) -> Vec<Vec<Seg>> {
    let simple = segments.last().and_then(|last| {
        if last.descendant || last.selectors.len() != 1 {
            return None;
        }
        match &last.selectors[0] {
            s @ (Selector::Name(_) | Selector::Index(_)) => Some(s.clone()),
            _ => None,
        }
    });
    let Some(selector) = simple else {
        return evaluate_segments(root, segments).into_iter().map(|m| m.path).collect();
    };
    let mut out = Vec::new();
    for parent in evaluate_segments(root, &segments[..segments.len() - 1]) {
        match (parent.value, &selector) {
            (Value::Object(_), Selector::Name(name)) => {
                let mut p = parent.path.clone();
                p.push(Seg::Key(name.clone()));
                out.push(p);
            }
            (Value::Array(items), _) => {
                let raw = match &selector {
                    Selector::Index(i) => Some(*i),
                    Selector::Name(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => Some(
                        n.bytes()
                            .fold(0i64, |a, b| a.saturating_mul(10).saturating_add(i64::from(b - b'0'))),
                    ),
                    _ => None,
                };
                let Some(raw) = raw else { continue };
                let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
                let i = if raw < 0 { len + raw } else { raw };
                if (0..=len).contains(&i) {
                    let mut p = parent.path.clone();
                    p.push(Seg::Index(usize::try_from(i).unwrap_or(0)));
                    out.push(p);
                }
            }
            _ => {}
        }
    }
    out
}

fn cmp_paths(a: &[Seg], b: &[Seg]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        if x == y {
            continue;
        }
        return match (x, y) {
            (Seg::Index(i), Seg::Index(j)) => j.cmp(i),
            _ => {
                if cmp_utf16(&x.to_string(), &y.to_string()) == Ordering::Less {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        };
    }
    b.len().cmp(&a.len())
}

/// Order matches so mutating them one by one never invalidates a later path: descendants before
/// their ancestors and higher array indices before lower ones; sibling keys sort by code unit.
#[must_use]
pub fn mutation_order(matches: &[Vec<Seg>]) -> Vec<Vec<Seg>> {
    let mut out = matches.to_vec();
    out.sort_by(|p, q| cmp_paths(p, q));
    out
}

/// The node at `path`, if it exists.
#[must_use]
pub fn get_at<'a>(root: &'a Value, path: &[Seg]) -> Option<&'a Value> {
    let mut node = root;
    for seg in path {
        node = match (node, seg) {
            (Value::Array(a), Seg::Index(i)) => a.get(*i)?,
            (Value::Object(m), Seg::Key(k)) => m.get(k)?,
            _ => return None,
        };
    }
    Some(node)
}

/// The node at `path`, mutably.
pub fn get_at_mut<'a>(root: &'a mut Value, path: &[Seg]) -> Option<&'a mut Value> {
    let mut node = root;
    for seg in path {
        node = match (node, seg) {
            (Value::Array(a), Seg::Index(i)) => a.get_mut(*i)?,
            (Value::Object(m), Seg::Key(k)) => m.get_mut(k)?,
            _ => return None,
        };
    }
    Some(node)
}

/// The container holding `path`'s last segment, if it still exists.
pub fn parent_of_mut<'a>(root: &'a mut Value, path: &[Seg]) -> Option<&'a mut Value> {
    get_at_mut(root, &path[..path.len().saturating_sub(1)])
}

/// The container holding `path`'s last segment, if it still exists.
#[must_use]
pub fn parent_of<'a>(root: &'a Value, path: &[Seg]) -> Option<&'a Value> {
    get_at(root, &path[..path.len().saturating_sub(1)])
}

/// Set `path` to `value`; a no-op if its parent no longer exists.
pub fn set_at(root: &mut Value, path: &[Seg], value: Value) {
    let Some(key) = path.last() else {
        *root = value;
        return;
    };
    match (parent_of_mut(root, path), key) {
        (Some(Value::Array(a)), Seg::Index(i)) => {
            if *i < a.len() {
                a[*i] = value;
            } else {
                // Assigning past the end leaves holes, which JSON writes as null.
                a.resize(*i, Value::Null);
                a.push(value);
            }
        }
        (Some(Value::Object(m)), Seg::Key(k)) => {
            m.insert(k.clone(), value);
        }
        _ => {}
    }
}

/// Delete `path` (splicing arrays); a no-op if it no longer exists.
pub fn delete_at(root: &mut Value, path: &[Seg]) {
    let Some(key) = path.last() else { return };
    match (parent_of_mut(root, path), key) {
        (Some(Value::Array(a)), Seg::Index(i)) => {
            if *i < a.len() {
                a.remove(*i);
            }
        }
        (Some(Value::Object(m)), Seg::Key(k)) => {
            m.remove(k);
        }
        _ => {}
    }
}

/// Whether `prefix` is a prefix of `path`.
#[must_use]
pub fn is_prefix(prefix: &[Seg], path: &[Seg]) -> bool {
    prefix.len() <= path.len() && prefix.iter().zip(path).all(|(a, b)| a == b)
}

/// `$["a"][0]`.
#[must_use]
pub fn describe_path(path: &[Seg]) -> String {
    let mut out = String::from("$");
    for s in path {
        match s {
            Seg::Index(i) => {
                let _ = write!(out, "[{i}]");
            }
            Seg::Key(k) => {
                let _ = write!(out, "[{}]", quote(k));
            }
        }
    }
    out
}
