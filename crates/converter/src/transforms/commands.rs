//! The `openapi.transforms` commands. Every command checks first and mutates only when it found no
//! errors, so a failing transform leaves the document exactly as it was. `pointer` addresses the
//! transform's `args` object.

use super::jsonpath::query;
use super::targets::{
    Target, delete_at, describe_path, error, get_at, get_at_mut, is_prefix, mutation_order, no_match, parent_of,
    resolve_destinations, resolve_targets, set_at, try_path,
};
use crate::Diagnostic;
use crate::diagnostic::{Seg, escape_token};
use crate::value::{Map, Value};

/// A command: document, validated `args`, pointer of `args`.
pub type Command = fn(&mut Value, &Map, &str) -> Vec<Diagnostic>;

fn type_mismatch(pointer: &str, path: &[Seg], value: Option<&Value>, expected: &str, command: &str) -> Diagnostic {
    let actual = value.map_or("undefined", Value::type_name);
    error(
        "FER1205",
        pointer,
        format!("{command} needs {expected} at {}, found {actual}.", describe_path(path)),
        format!("Point the target at {expected}, or use a different command."),
    )
}

const ROOT_SUGGESTION: &str = "The document root must stay an object; target a node inside it.";

fn target_arg(args: &Map, key: &str) -> Target {
    Target::from_value(args.get(key).unwrap_or(&Value::Null))
}

/// `update`: replace (or template) every target.
pub fn update(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let resolved = resolve_targets(doc, &target_arg(args, "target"), &format!("{pointer}/target"));
    let mut diagnostics = resolved.diagnostics;
    let value = args.get("value").cloned().unwrap_or(Value::Null);
    let template = args.get("template") == Some(&Value::Bool(true));
    if template && !value.as_str().is_some_and(|s| s.contains("{{value}}")) {
        diagnostics.push(error(
            "FER1210",
            format!("{pointer}/value"),
            "A templated update needs a string value containing {{value}}.",
            "Include {{value}} where the current string goes, e.g. \"[BETA] {{value}}\", or drop template.",
        ));
    }
    for m in &resolved.matches {
        let current = get_at(doc, m);
        if template && !matches!(current, Some(Value::String(_))) {
            diagnostics.push(type_mismatch(
                &format!("{pointer}/target"),
                m,
                current,
                "a string",
                "A templated update",
            ));
        }
        if m.is_empty() && !template && !value.is_object() {
            diagnostics.push(error(
                "FER1205",
                format!("{pointer}/value"),
                "update $ needs an object value.",
                ROOT_SUGGESTION,
            ));
        }
    }
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    for m in mutation_order(&resolved.matches) {
        let next = match (&value, template) {
            (Value::String(t), true) => {
                let current = get_at(doc, &m).map(crate::value::js_string).unwrap_or_default();
                Value::String(t.split("{{value}}").collect::<Vec<_>>().join(&current))
            }
            _ => value.clone(),
        };
        set_at(doc, &m, next);
    }
    Vec::new()
}

/// `append`: push onto arrays, add new properties to objects.
pub fn append(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let resolved = resolve_targets(doc, &target_arg(args, "target"), &format!("{pointer}/target"));
    let mut diagnostics = resolved.diagnostics;
    let value = args.get("value").cloned().unwrap_or(Value::Null);
    for m in &resolved.matches {
        match get_at(doc, m) {
            Some(Value::Array(_)) => {}
            Some(Value::Object(existing)) => match &value {
                Value::Object(new) => {
                    for key in new.keys() {
                        if existing.contains_key(key) {
                            diagnostics.push(error(
                                "FER1206",
                                format!("{pointer}/value/{}", escape_token(key)),
                                format!("Property \"{key}\" already exists at {}.", describe_path(m)),
                                "The spec now has this property: delete the append, or use merge to override it on purpose.",
                            ));
                        }
                    }
                }
                other => diagnostics.push(type_mismatch(
                    &format!("{pointer}/value"),
                    m,
                    Some(other),
                    "an object value",
                    "Appending to an object",
                )),
            },
            other => diagnostics.push(type_mismatch(
                &format!("{pointer}/target"),
                m,
                other,
                "an array or object",
                "append",
            )),
        }
    }
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    for m in mutation_order(&resolved.matches) {
        match get_at_mut(doc, &m) {
            Some(Value::Array(items)) => match &value {
                Value::Array(many) => items.extend(many.iter().cloned()),
                one => items.push(one.clone()),
            },
            Some(Value::Object(node)) => {
                if let Value::Object(new) = &value {
                    for (k, v) in new {
                        node.insert(k.clone(), v.clone());
                    }
                }
            }
            _ => {}
        }
    }
    Vec::new()
}

fn deep_merge(into: &mut Map, from: &Map) {
    for (k, v) in from {
        match (into.get_mut(k), v) {
            (Some(Value::Object(existing)), Value::Object(inner)) => deep_merge(existing, inner),
            _ => {
                into.insert(k.clone(), v.clone());
            }
        }
    }
}

/// `merge`: deep merge an object into every target.
pub fn merge(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let resolved = resolve_targets(doc, &target_arg(args, "target"), &format!("{pointer}/target"));
    let mut diagnostics = resolved.diagnostics;
    for m in &resolved.matches {
        let current = get_at(doc, m);
        if !matches!(current, Some(Value::Object(_))) {
            diagnostics.push(type_mismatch(
                &format!("{pointer}/target"),
                m,
                current,
                "an object",
                "merge",
            ));
        }
    }
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    let empty = Map::new();
    let value = args.get("value").and_then(Value::as_object).unwrap_or(&empty);
    for m in mutation_order(&resolved.matches) {
        if let Some(Value::Object(node)) = get_at_mut(doc, &m) {
            deep_merge(node, value);
        }
    }
    Vec::new()
}

/// `remove`: delete targets, or some of their keys.
pub fn remove(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let resolved = resolve_targets(doc, &target_arg(args, "target"), &format!("{pointer}/target"));
    let mut diagnostics = resolved.diagnostics;
    let keys: Option<Vec<String>> = args
        .get("keys")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|k| k.as_str().unwrap_or("").to_owned()).collect());
    for m in &resolved.matches {
        match &keys {
            None => {
                if m.is_empty() {
                    diagnostics.push(error(
                        "FER1211",
                        format!("{pointer}/target"),
                        "remove cannot delete the document root.",
                        ROOT_SUGGESTION,
                    ));
                }
            }
            Some(keys) => match get_at(doc, m) {
                Some(Value::Object(node)) => {
                    for (i, key) in keys.iter().enumerate() {
                        if !node.contains_key(key) {
                            diagnostics.push(error(
                                "FER1209",
                                format!("{pointer}/keys/{i}"),
                                format!("Key \"{key}\" is not present at {}.", describe_path(m)),
                                "The spec no longer has this key: drop it from keys, or delete the transform.",
                            ));
                        }
                    }
                }
                other => {
                    diagnostics.push(type_mismatch(
                        &format!("{pointer}/target"),
                        m,
                        other,
                        "an object",
                        "remove with keys",
                    ));
                }
            },
        }
    }
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    for m in mutation_order(&resolved.matches) {
        match &keys {
            None => delete_at(doc, &m),
            Some(keys) => {
                if let Some(Value::Object(node)) = get_at_mut(doc, &m) {
                    for key in keys {
                        node.remove(key);
                    }
                }
            }
        }
    }
    Vec::new()
}

/// Resolve `from` to exactly one node: its path and value.
fn single(doc: &Value, from: &str, pointer: &str, diagnostics: &mut Vec<Diagnostic>) -> Option<(Vec<Seg>, Value)> {
    let found = try_path(pointer, from, diagnostics, query(doc, from))?;
    let Some(only) = found.first() else {
        diagnostics.push(no_match(pointer, from));
        return None;
    };
    if found.len() > 1 {
        diagnostics.push(error(
            "FER1207",
            pointer,
            format!("\"{from}\" must match exactly one node, found {}.", found.len()),
            "Narrow the path (avoid wildcards, recursive descent and filters) so it selects one node.",
        ));
        return None;
    }
    Some((only.path.clone(), only.value.clone()))
}

fn str_arg<'a>(args: &'a Map, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or("")
}

/// `copy`: copy one node to every destination.
pub fn copy(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let source = single(doc, str_arg(args, "from"), &format!("{pointer}/from"), &mut diagnostics);
    let dest = resolve_destinations(doc, &target_arg(args, "to"), &format!("{pointer}/to"));
    diagnostics.extend(dest.diagnostics);
    if let Some((_, v)) = &source
        && !v.is_object()
        && dest.matches.iter().any(Vec::is_empty)
    {
        diagnostics.push(error(
            "FER1205",
            format!("{pointer}/to"),
            "copy to $ needs an object source.",
            ROOT_SUGGESTION,
        ));
    }
    let Some((source_path, mut source_value)) = source else {
        return diagnostics;
    };
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    for m in mutation_order(&dest.matches) {
        let push =
            matches!((parent_of(doc, &m), m.last()), (Some(Value::Array(a)), Some(Seg::Index(i))) if *i == a.len());
        let copy = source_value.clone();
        if push {
            if let Some(Value::Array(a)) = super::targets::parent_of_mut(doc, &m) {
                a.push(copy.clone());
            }
        } else {
            set_at(doc, &m, copy.clone());
        }
        // The source is held by reference: a write inside it is seen by later copies.
        if is_prefix(&source_path, &m) && m.len() > source_path.len() {
            set_at(&mut source_value, &m[source_path.len()..], copy);
        }
    }
    Vec::new()
}

/// `move`: relocate one node.
pub fn move_(doc: &mut Value, args: &Map, pointer: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let to = str_arg(args, "to");
    let source = single(doc, str_arg(args, "from"), &format!("{pointer}/from"), &mut diagnostics);
    let dest = resolve_destinations(doc, &Target::One(to.to_owned()), &format!("{pointer}/to"));
    diagnostics.extend(dest.diagnostics);
    let target = dest.matches.first().cloned();
    if dest.matches.len() > 1 {
        diagnostics.push(error(
            "FER1207",
            format!("{pointer}/to"),
            format!(
                "\"{to}\" must address exactly one destination, found {}.",
                dest.matches.len()
            ),
            "Narrow the destination path so it selects one location.",
        ));
    }
    if let (Some((sp, sv)), Some(tp)) = (&source, &target) {
        if sp.is_empty() {
            diagnostics.push(error(
                "FER1211",
                format!("{pointer}/from"),
                "move cannot relocate the document root.",
                ROOT_SUGGESTION,
            ));
        } else if is_prefix(sp, tp) && sp.len() < tp.len() {
            diagnostics.push(error(
                "FER1211",
                format!("{pointer}/to"),
                format!("Cannot move {} into itself ({}).", describe_path(sp), describe_path(tp)),
                "Pick a destination outside the node being moved.",
            ));
        } else if tp.is_empty() && !sv.is_object() {
            diagnostics.push(error(
                "FER1205",
                format!("{pointer}/to"),
                "move to $ needs an object source.",
                ROOT_SUGGESTION,
            ));
        }
    }
    let (Some((sp, sv)), Some(tp)) = (source, target) else {
        return diagnostics;
    };
    if !diagnostics.is_empty() {
        return diagnostics;
    }
    relocate(doc, &sp, sv, &tp);
    Vec::new()
}

fn relocate(doc: &mut Value, from: &[Seg], value: Value, to: &[Seg]) {
    if from == to {
        return;
    }
    let same_parent = from.len() == to.len() && is_prefix(&from[..from.len() - 1], to);
    let from_key = from.last();
    let to_key = to.last();
    if same_parent {
        if let (Some(Seg::Key(fk)), Some(Seg::Key(tk))) = (from_key, to_key)
            && let Some(Value::Object(parent)) = super::targets::parent_of_mut(doc, from)
        {
            // Rename in place so the key keeps its position.
            let entries: Vec<(String, Value)> = std::mem::take(parent).into_iter().filter(|(k, _)| k != tk).collect();
            for (k, v) in entries {
                parent.insert(if &k == fk { tk.clone() } else { k }, v);
            }
            return;
        }
        if let (Some(Seg::Index(fi)), Some(Seg::Index(ti))) = (from_key, to_key)
            && let Some(Value::Array(parent)) = super::targets::parent_of_mut(doc, from)
        {
            if *fi < parent.len() {
                let item = parent.remove(*fi);
                let at = (*ti).min(parent.len());
                parent.insert(at, item);
            }
            return;
        }
    }
    // Write the destination first: it is never inside the source, so the source path stays valid,
    // unless the destination is an ancestor of the source, which the write already replaced.
    let push = matches!((parent_of(doc, to), to_key), (Some(Value::Array(a)), Some(Seg::Index(i))) if *i == a.len());
    if push {
        if let Some(Value::Array(a)) = super::targets::parent_of_mut(doc, to) {
            a.push(value);
        }
    } else {
        set_at(doc, to, value);
    }
    if !is_prefix(to, from) {
        delete_at(doc, from);
    }
}

/// Legacy `oneOfToAnyOf` (deprecated `openapi.transformations` only): rename every `oneOf`
/// keyword in the spec to `anyOf`.
pub fn one_of_to_any_of(doc: &mut Value, _args: &Map, _pointer: &str) -> Vec<Diagnostic> {
    fn visit(node: Value) -> Value {
        match node {
            Value::Array(a) => Value::Array(a.into_iter().map(visit).collect()),
            Value::Object(m) => Value::Object(
                m.into_iter()
                    .map(|(k, v)| (if k == "oneOf" { "anyOf".to_owned() } else { k }, visit(v)))
                    .collect(),
            ),
            other => other,
        }
    }
    *doc = visit(std::mem::take(doc));
    Vec::new()
}
