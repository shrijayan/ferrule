//! Stainless `openapi.transforms`: update, append, merge, remove, copy and move, as defined by the
//! `openapi.transforms` items in the public schema.

pub mod annotate;
pub mod commands;
pub mod jsonpath;
pub mod targets;

use crate::Diagnostic;
use crate::diagnostic::{Seg, to_json_pointer};
use crate::value::{Map, Value, cmp_utf16, to_json};
use crate::zod::Z;
use commands::Command;

fn path_z() -> Z {
    Z::String
}

fn target_z() -> Z {
    let entry = Z::Union(vec![
        path_z(),
        Z::Object(vec![
            ("matches_schema", path_z(), false),
            ("ignore", Z::Union(vec![path_z(), Z::Array(Box::new(path_z()))]), true),
        ]),
    ]);
    Z::Union(vec![path_z(), Z::Array(Box::new(entry))])
}

struct Spec {
    args: Z,
    keys: &'static [&'static str],
    required: &'static [&'static str],
    run: Command,
}

/// The command names the schema defines, in schema order.
pub const TRANSFORM_COMMANDS: [&str; 6] = ["update", "append", "merge", "remove", "copy", "move"];

fn command_spec(name: &str, legacy: bool) -> Option<Spec> {
    Some(match name {
        "update" => Spec {
            args: Z::Object(vec![("target", target_z(), false), ("template", Z::Boolean, true)]),
            keys: &["target", "value", "template"],
            required: &["target", "value"],
            run: commands::update,
        },
        "append" => Spec {
            args: Z::Object(vec![("target", target_z(), false)]),
            keys: &["target", "value"],
            required: &["target", "value"],
            run: commands::append,
        },
        "merge" => Spec {
            args: Z::Object(vec![
                ("target", target_z(), false),
                ("value", Z::Record(Box::new(Z::Unknown)), false),
            ]),
            keys: &["target", "value"],
            required: &["target", "value"],
            run: commands::merge,
        },
        "remove" => Spec {
            args: Z::Object(vec![
                ("target", target_z(), false),
                ("keys", Z::Array(Box::new(Z::String)), true),
            ]),
            keys: &["target", "keys"],
            required: &["target"],
            run: commands::remove,
        },
        "copy" => Spec {
            args: Z::Object(vec![("from", path_z(), false), ("to", target_z(), false)]),
            keys: &["from", "to"],
            required: &["from", "to"],
            run: commands::copy,
        },
        "move" => Spec {
            args: Z::Object(vec![("from", path_z(), false), ("to", path_z(), false)]),
            keys: &["from", "to"],
            required: &["from", "to"],
            run: commands::move_,
        },
        // Commands only the deprecated `openapi.transformations` list accepts (no args).
        "oneOfToAnyOf" if legacy => Spec {
            args: Z::Object(vec![]),
            keys: &[],
            required: &[],
            run: commands::one_of_to_any_of,
        },
        _ => return None,
    })
}

/// The result of [`apply_transforms`].
#[derive(Debug, Clone)]
pub struct ApplyTransformsResult {
    pub spec: Value,
    pub diagnostics: Vec<Diagnostic>,
}

fn invalid(pointer: impl Into<String>, message: impl Into<String>, suggestion: impl Into<String>) -> Diagnostic {
    Diagnostic::error("FER1203", pointer, message, suggestion)
}

fn unknown_keys(value: &Map, known: &[&str], pointer: &str) -> Vec<Diagnostic> {
    let mut keys: Vec<&String> = value.keys().filter(|k| !known.contains(&k.as_str())).collect();
    keys.sort_by(|a, b| cmp_utf16(a, b));
    keys.into_iter()
        .map(|k| {
            Diagnostic::warning(
                "FER1204",
                format!("{pointer}{}", to_json_pointer(&[Seg::Key(k.clone())])),
                format!("Unknown key \"{k}\" is ignored."),
                format!("Remove it; this place accepts: {}.", known.join(", ")),
            )
        })
        .collect()
}

/// Validate one transform entry; returns the command and args to run, and diagnostics.
fn validate(entry: &Value, pointer: &str, legacy: bool) -> (Option<(Spec, Map)>, Vec<Diagnostic>) {
    let commands_list = TRANSFORM_COMMANDS.join(", ");
    let Value::Object(entry) = entry else {
        return (
            None,
            vec![invalid(
                pointer,
                "A transform must be an object.",
                "Write { command, reason, args }.",
            )],
        );
    };
    let command = entry.get("command");
    let spec = command.and_then(Value::as_str).and_then(|c| command_spec(c, legacy));
    let Some(spec) = spec else {
        let shown = command.map_or_else(|| "undefined".to_owned(), to_json);
        // JSON.stringify(undefined) is undefined, which the template prints as "undefined".
        return (
            None,
            vec![invalid(
                format!("{pointer}/command"),
                format!("Unknown transform command {shown}."),
                format!("Use one of: {commands_list}."),
            )],
        );
    };
    let command = command.and_then(Value::as_str).unwrap_or("");
    // A command with no required args may leave `args` out.
    let args = match entry.get("args") {
        None if spec.required.is_empty() => Some(Value::object()),
        other => other.cloned(),
    };
    let mut diagnostics = unknown_keys(entry, &["command", "reason", "args"], pointer);
    let reason_ok = entry
        .get("reason")
        .and_then(Value::as_str)
        .is_some_and(|r| !r.trim_matches(crate::value::is_js_space).is_empty());
    if !reason_ok {
        diagnostics.push(Diagnostic::warning(
            "FER1204",
            format!("{pointer}/reason"),
            format!("The {command} transform has no reason."),
            "Add a reason saying why the spec needs this change, so it can be removed once fixed.",
        ));
    }
    let at = format!("{pointer}/args");
    let Some(Value::Object(args)) = args else {
        diagnostics.push(invalid(
            &at,
            format!("The {command} transform needs an args object."),
            format!("Give args: {}.", spec.required.join(", ")),
        ));
        return (None, diagnostics);
    };
    let missing: Vec<&str> = spec
        .required
        .iter()
        .copied()
        .filter(|k| !args.contains_key(k))
        .collect();
    for k in &missing {
        diagnostics.push(invalid(
            format!("{at}/{k}"),
            format!("The {command} transform is missing args.{k}."),
            format!("Add args.{k}."),
        ));
    }
    for issue in spec.args.issues(&Value::Object(args.clone())) {
        let first = issue.path.first().map(ToString::to_string);
        if issue.code == "invalid_type" && first.is_some_and(|f| missing.contains(&f.as_str())) {
            continue;
        }
        diagnostics.push(invalid(
            format!("{at}{}", to_json_pointer(&issue.path)),
            format!("Invalid {command} args: {}.", issue.message),
            "Fix the argument to match the openapi.transforms schema.",
        ));
    }
    diagnostics.extend(unknown_keys(&args, spec.keys, &at));
    if diagnostics.iter().any(Diagnostic::is_error) {
        return (None, diagnostics);
    }
    (Some((spec, args)), diagnostics)
}

/// Apply Stainless `openapi.transforms` to a copy of `spec`, in order. Pure. A transform with
/// any error diagnostic is skipped as a whole; later transforms still run.
#[must_use]
pub fn apply_transforms(spec: &Value, transforms: &[Value], pointer: Option<&str>) -> ApplyTransformsResult {
    let base = pointer.unwrap_or("/openapi/transforms");
    let mut root = spec.clone();
    let mut diagnostics = Vec::new();
    for (i, entry) in transforms.iter().enumerate() {
        let pointer = format!("{base}/{i}");
        let (checked, ds) = validate(entry, &pointer, base.ends_with("/transformations"));
        diagnostics.extend(ds);
        if let Some((spec, args)) = checked {
            diagnostics.extend((spec.run)(&mut root, &args, &format!("{pointer}/args")));
        }
    }
    ApplyTransformsResult {
        spec: root,
        diagnostics,
    }
}

/// Read `openapi.transforms` (or its deprecated alias `openapi.transformations`) from a config
/// and apply it. Pure.
#[must_use]
pub fn apply_config_transforms(spec: &Value, config: &Value) -> ApplyTransformsResult {
    let Some(Value::Object(openapi)) = config.get("openapi") else {
        return ApplyTransformsResult {
            spec: spec.clone(),
            diagnostics: Vec::new(),
        };
    };
    let mut diagnostics = Vec::new();
    let has_new = openapi.contains_key("transforms");
    let has_old = openapi.contains_key("transformations");
    let key = if has_new { "transforms" } else { "transformations" };
    if has_old {
        diagnostics.push(Diagnostic::new(
            "FER1212",
            if has_new {
                crate::Severity::Error
            } else {
                crate::Severity::Warning
            },
            if has_new {
                "Both openapi.transforms and openapi.transformations are set; transformations is ignored."
            } else {
                "openapi.transformations is deprecated."
            },
            "/openapi/transformations",
            if has_new {
                "Move every entry into openapi.transforms and delete openapi.transformations."
            } else {
                "Rename openapi.transformations to openapi.transforms."
            },
        ));
    }
    match openapi.get(key) {
        None | Some(Value::Null) => ApplyTransformsResult {
            spec: spec.clone(),
            diagnostics,
        },
        Some(Value::Array(list)) => {
            let result = apply_transforms(spec, list, Some(&format!("/openapi/{key}")));
            diagnostics.extend(result.diagnostics);
            ApplyTransformsResult {
                spec: result.spec,
                diagnostics,
            }
        }
        Some(_) => {
            diagnostics.push(invalid(
                format!("/openapi/{key}"),
                format!("openapi.{key} must be a list."),
                "Write a YAML list of { command, reason, args } entries.",
            ));
            ApplyTransformsResult {
                spec: spec.clone(),
                diagnostics,
            }
        }
    }
}
