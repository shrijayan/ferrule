//! The durable goldens: the OneBusAway full snapshot and the config oracle digests (ADR 0010),
//! plus the oracle regressions. Ports of converter/test/onebusaway.test.ts,
//! converter/test/oracle-regressions.test.ts and the Phase 2 gate's comparison
//! (`digest` in `scripts/oracles/converter_oracles.py`). Snapshots are read, never written; a pair
//! whose oracle files are not cached is skipped.

use ferrule_converter::value::parse_json;
use ferrule_converter::{ConvertOutput, Severity, Value, convert_text};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn snapshots() -> PathBuf {
    root().join("converter/test/__snapshots__")
}

// ---------------------------------------------------------------------------------------------
// Python `json.loads` / `json.dumps(indent=2, ensure_ascii=False)`, order-preserving, so the
// digest is built exactly as the Python gate builds it.

#[derive(Debug, Clone)]
enum Py {
    Null,
    Bool(bool),
    /// The number as written: integers keep their digits, floats print as `repr`.
    Num(String),
    Str(String),
    Arr(Vec<Py>),
    Obj(Vec<(String, Py)>),
}

impl PartialEq for Py {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Py::Null, Py::Null) => true,
            (Py::Bool(a), Py::Bool(b)) => a == b,
            (Py::Num(a), Py::Num(b)) => {
                a == b
                    || a.parse::<f64>()
                        .ok()
                        .zip(b.parse::<f64>().ok())
                        .is_some_and(|(x, y)| x.to_bits() == y.to_bits())
            }
            (Py::Str(a), Py::Str(b)) => a == b,
            (Py::Arr(a), Py::Arr(b)) => a == b,
            (Py::Obj(a), Py::Obj(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.iter().any(|(k2, v2)| k == k2 && v == v2))
            }
            _ => false,
        }
    }
}

struct Reader<'a> {
    s: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Py {
        self.ws();
        match self.s[self.i] {
            b'{' => {
                self.i += 1;
                let mut out: Vec<(String, Py)> = Vec::new();
                loop {
                    self.ws();
                    if self.s[self.i] == b'}' {
                        self.i += 1;
                        return Py::Obj(out);
                    }
                    if self.s[self.i] == b',' {
                        self.i += 1;
                        continue;
                    }
                    let key = self.string();
                    self.ws();
                    self.i += 1; // ':'
                    let v = self.value();
                    // Python dicts keep the last value of a repeated key, at the first position.
                    if let Some(slot) = out.iter_mut().find(|(k, _)| *k == key) {
                        slot.1 = v;
                    } else {
                        out.push((key, v));
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut out = Vec::new();
                loop {
                    self.ws();
                    if self.s[self.i] == b']' {
                        self.i += 1;
                        return Py::Arr(out);
                    }
                    if self.s[self.i] == b',' {
                        self.i += 1;
                        continue;
                    }
                    out.push(self.value());
                }
            }
            b'"' => Py::Str(self.string()),
            b't' => {
                self.i += 4;
                Py::Bool(true)
            }
            b'f' => {
                self.i += 5;
                Py::Bool(false)
            }
            b'n' => {
                self.i += 4;
                Py::Null
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    self.i += 1;
                }
                Py::Num(String::from_utf8_lossy(&self.s[start..self.i]).into_owned())
            }
        }
    }

    fn string(&mut self) -> String {
        // Reuse the converter's JSON string reader on the quoted slice.
        let start = self.i;
        self.i += 1;
        while self.s[self.i] != b'"' {
            if self.s[self.i] == b'\\' {
                self.i += 1;
            }
            self.i += 1;
        }
        self.i += 1;
        let raw = std::str::from_utf8(&self.s[start..self.i]).expect("utf-8");
        match parse_json(raw) {
            Some(Value::String(s)) => s,
            other => panic!("bad string {raw}: {other:?}"),
        }
    }
}

fn loads(text: &str) -> Py {
    Reader {
        s: text.as_bytes(),
        i: 0,
    }
    .value()
}

/// Python `repr(float)` for a JSON number that is not an integer literal.
fn py_float(raw: &str) -> String {
    if !raw.contains(['.', 'e', 'E']) {
        return raw.to_owned();
    }
    let n: f64 = raw.parse().expect("number");
    let sci = format!("{:e}", n.abs());
    let (mantissa, exp) = sci.split_once('e').expect("exponent");
    let exp: i32 = exp.parse().expect("exponent");
    let sign = if n < 0.0 { "-" } else { "" };
    if (-4..16).contains(&exp) {
        let fixed = format!("{}", n.abs());
        let fixed = if fixed.contains('.') {
            fixed
        } else {
            format!("{fixed}.0")
        };
        format!("{sign}{fixed}")
    } else {
        format!("{sign}{mantissa}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    }
}

fn dump(v: &Py, level: usize, out: &mut String) {
    let pad = |n: usize| "  ".repeat(n);
    match v {
        Py::Null => out.push_str("null"),
        Py::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Py::Num(n) => out.push_str(&py_float(n)),
        Py::Str(s) => out.push_str(&ferrule_converter::value::quote(s)),
        Py::Arr(a) if a.is_empty() => out.push_str("[]"),
        Py::Obj(o) if o.is_empty() => out.push_str("{}"),
        Py::Arr(a) => {
            out.push_str("[\n");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&pad(level + 1));
                dump(x, level + 1, out);
            }
            let _ = write!(out, "\n{}]", pad(level));
        }
        Py::Obj(o) => {
            out.push_str("{\n");
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                let _ = write!(out, "{}{}: ", pad(level + 1), ferrule_converter::value::quote(k));
                dump(x, level + 1, out);
            }
            let _ = write!(out, "\n{}}}", pad(level));
        }
    }
}

fn members(v: Option<&Py>) -> Vec<(String, Py)> {
    match v {
        Some(Py::Obj(o)) => o.clone(),
        _ => Vec::new(),
    }
}

fn sorted(mut items: Vec<(String, Py)>) -> Vec<(String, Py)> {
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

fn get<'a>(items: &'a [(String, Py)], key: &str) -> Option<&'a Py> {
    items.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// `digest` of `scripts/oracles/converter_oracles.py`.
fn digest(converted: &str) -> String {
    let spec = members(Some(&loads(converted)));
    let mut ops: Vec<(String, Vec<(String, Py)>)> = Vec::new();
    for (route, item) in sorted(members(get(&spec, "paths"))) {
        for (verb, op) in sorted(members(Some(&item))) {
            let ext: Vec<(String, Py)> = sorted(members(Some(&op)))
                .into_iter()
                .filter(|(k, _)| k.starts_with("x-"))
                .collect();
            if !ext.is_empty() {
                ops.push((format!("{} {route}", verb.to_uppercase()), ext));
            }
        }
    }
    let root_ext: Vec<(String, Py)> = sorted(spec.clone())
        .into_iter()
        .filter(|(k, _)| k.starts_with("x-ferrule"))
        .collect();
    let mut mirrored: Vec<(String, Py)> = Vec::new();
    for key in ["x-ferrule-client", "x-ferrule-models", "x-ferrule-package"] {
        let values: Vec<&Py> = ops.iter().filter_map(|(_, op)| get(op, key)).collect();
        if let Some(first) = values.first()
            && values.iter().all(|v| v == first)
        {
            mirrored.push((key.to_owned(), (*first).clone()));
            for (_, op) in &mut ops {
                op.retain(|(k, _)| k != key);
            }
        }
    }
    let hash: String = Sha256::digest(converted.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        });
    let body = Py::Obj(vec![
        ("sha256".to_owned(), Py::Str(hash)),
        ("root".to_owned(), Py::Obj(root_ext)),
        ("mirrored".to_owned(), Py::Obj(mirrored)),
        (
            "operations".to_owned(),
            Py::Obj(ops.into_iter().map(|(k, v)| (k, Py::Obj(v))).collect()),
        ),
    ]);
    let mut out = String::new();
    dump(&body, 0, &mut out);
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------------------------
// The oracle pairs (oracles/manifest.yaml: comments, then one JSON document).

struct Pair {
    name: String,
    spec: PathBuf,
    config: PathBuf,
}

fn config_pairs() -> Vec<Pair> {
    let cache = root().join("oracles/cache");
    let text = std::fs::read_to_string(root().join("oracles/manifest.yaml")).expect("manifest");
    let json: String = text
        .lines()
        .map(|l| if l.starts_with('#') { "" } else { l })
        .collect::<Vec<_>>()
        .join("\n");
    let data = parse_json(&json).expect("manifest JSON");
    let entries = data
        .get("oracles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries
        .iter()
        .filter(|e| e.get("kind").and_then(Value::as_str) == Some("config"))
        .filter_map(|e| {
            let name = e.get("name")?.as_str()?.to_owned();
            let config = cache.join(&name).join(e.get("config_path")?.as_str()?);
            let spec = if let Some(p) = e.get("spec").and_then(|s| s.get("path")).and_then(Value::as_str) {
                cache.join(&name).join(p)
            } else {
                let mut found: Vec<PathBuf> = std::fs::read_dir(cache.join("specs"))
                    .ok()?
                    .filter_map(|f| f.ok().map(|f| f.path()))
                    .filter(|p| {
                        p.file_name()
                            .and_then(|f| f.to_str())
                            .is_some_and(|f| f.starts_with(&format!("{name}.")))
                    })
                    .collect();
                found.sort();
                found.into_iter().next()?
            };
            Some(Pair { name, spec, config })
        })
        .collect()
}

fn convert_files(spec: &Path, config: &Path) -> ConvertOutput {
    let spec = std::fs::read_to_string(spec).expect("spec");
    let config = std::fs::read_to_string(config).expect("config");
    convert_text(&spec, &config).expect("convert")
}

#[test]
fn config_oracles_match_their_digest_snapshots() {
    let mut checked = 0;
    for pair in config_pairs() {
        let snap = snapshots().join("oracles").join(format!("{}.json.snap", pair.name));
        if !pair.spec.exists() || !pair.config.exists() {
            eprintln!("skipped {}: oracle files not cached", pair.name);
            continue;
        }
        let expected = std::fs::read_to_string(&snap).unwrap_or_else(|_| panic!("{}: no snapshot", pair.name));
        let result = convert_files(&pair.spec, &pair.config);
        let errors: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors, Vec::<&ferrule_converter::Diagnostic>::new(), "{}", pair.name);
        assert!(
            digest(&result.output) == expected,
            "{}: differs from {}",
            pair.name,
            snap.display()
        );
        checked += 1;
    }
    eprintln!("checked {checked} oracle digests");
}

#[test]
fn digest_matches_python_formatting() {
    let out = digest(
        "{\n  \"paths\": {\n    \"/a\": {\n      \"get\": {\"x-b\": 0.5, \"x-a\": [], \"y\": 1}\n    }\n  },\n  \"x-ferrule-config\": {}\n}\n",
    );
    assert!(out.starts_with("{\n  \"sha256\": \""));
    assert!(out.ends_with("\"root\": {\n    \"x-ferrule-config\": {}\n  },\n  \"mirrored\": {},\n  \"operations\": {\n    \"GET /a\": {\n      \"x-a\": [],\n      \"x-b\": 0.5\n    }\n  }\n}\n"));
    assert_eq!(py_float("1e-07"), "1e-07");
    assert_eq!(py_float("100000000000000000000.5"), "1e+20");
    assert_eq!(py_float("2.50"), "2.5");
}

// ---------------------------------------------------------------------------------------------
// OneBusAway (converter/test/onebusaway.test.ts): a full snapshot (Apache-2.0).

fn onebusaway() -> Option<ConvertOutput> {
    let dir = root().join("oracles/cache/onebusaway-config");
    let spec = dir.join("openapi.yml");
    let config = dir.join("stainless-config.yml");
    (spec.exists() && config.exists()).then(|| convert_files(&spec, &config))
}

#[test]
fn onebusaway_converts_without_diagnostics() {
    let Some(result) = onebusaway() else { return };
    assert_eq!(result.diagnostics, Vec::new());
}

#[test]
fn onebusaway_annotates_agencies_with_coverage_list_without_pagination() {
    let Some(result) = onebusaway() else { return };
    let out = parse_json(&result.output).expect("output");
    let op = out
        .get("paths")
        .and_then(|p| p.get("/api/where/agencies-with-coverage.json"))
        .and_then(|i| i.get("get"))
        .expect("operation");
    assert_eq!(
        op.get("x-ferrule-resource"),
        Some(&Value::from(
            serde_json::json!({"path": ["agencies_with_coverage"], "order": [0]})
        ))
    );
    assert_eq!(op.get("x-ferrule-method-name"), Some(&Value::from("list")));
    assert_eq!(op.get("x-ferrule-method-order"), Some(&Value::Number(0.0)));
    assert!(op.get("x-ferrule-pagination").is_none());
    let client = op.get("x-ferrule-client").expect("client");
    assert_eq!(client.get("name"), Some(&Value::from("onebusaway-sdk")));
    assert_eq!(client.get("envPrefix"), Some(&Value::from("ONEBUSAWAY_SDK")));
    assert_eq!(
        client.get("environments"),
        Some(&Value::from(
            serde_json::json!([{"name": "production", "url": "https://api.pugetsound.onebusaway.org"}])
        ))
    );
    assert_eq!(
        client.get("query"),
        Some(&Value::from(
            serde_json::json!({"nestedFormat": "brackets", "arrayFormat": "repeat"})
        ))
    );
}

#[test]
fn onebusaway_matches_the_committed_snapshot() {
    let Some(result) = onebusaway() else { return };
    let expected = std::fs::read_to_string(snapshots().join("onebusaway.json.snap")).expect("snapshot");
    assert!(
        result.output == expected,
        "output differs from converter/test/__snapshots__/onebusaway.json.snap"
    );
}

// ---------------------------------------------------------------------------------------------
// converter/test/oracle-regressions.test.ts: minimal reductions of failed config oracles.

const HEAD: &str = "organization: { name: acme }
environments: { production: https://api.acme.test }
targets: { python: { package_name: acme, publish: { pypi: false } } }
readme:
  example_requests:
    default: { type: request, endpoint: get /a, params: {} }
    headline: { type: request, endpoint: get /a, params: {} }
";

fn regression_spec() -> String {
    ferrule_converter::value::to_json(&Value::from(serde_json::json!({
        "openapi": "3.1.0",
        "paths": {"/a": {"get": {"responses": {"200": {
            "description": "ok",
            "content": {"application/json": {"schema": {"oneOf": [{"type": "string"}, {"type": "integer"}]}}}
        }}}}},
        "components": {"schemas": {"App": {"type": "object"}}}
    })))
}

fn errors(r: &ConvertOutput) -> Vec<&ferrule_converter::Diagnostic> {
    r.diagnostics.iter().filter(|d| d.severity == Severity::Error).collect()
}

#[test]
fn opencode_a_model_naming_a_missing_schema_is_skipped_with_a_warning() {
    let config = format!(
        "{HEAD}resources:\n  a:\n    models: {{ app: App, log_level: LogLevel }}\n    methods: {{ list: get /a }}\n"
    );
    let r = convert_text(&regression_spec(), &config).expect("convert");
    assert_eq!(errors(&r), Vec::<&ferrule_converter::Diagnostic>::new());
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "FER1005" && d.severity == Severity::Warning)
    );
    let out = parse_json(&r.output).expect("output");
    let models = out
        .get("paths")
        .and_then(|p| p.get("/a"))
        .and_then(|i| i.get("get"))
        .and_then(|o| o.get("x-ferrule-models"));
    let keys: Vec<&str> = models
        .and_then(Value::as_object)
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default();
    assert_eq!(keys, ["App"]);
}

#[test]
fn imagekit_webhook_unwrap_methods_have_no_endpoint_and_are_accepted() {
    let config = format!(
        "{HEAD}resources:\n  webhooks:\n    methods:\n      unwrap: {{ type: webhook_unwrap, discriminator: type, webhook_key_opt: webhook_secret }}\n  a:\n    methods: {{ list: get /a }}\n"
    );
    let r = convert_text(&regression_spec(), &config).expect("convert");
    assert_eq!(errors(&r), Vec::<&ferrule_converter::Diagnostic>::new());
}

#[test]
fn ogx_the_legacy_one_of_to_any_of_transformation_turns_every_one_of_into_any_of() {
    let config = format!(
        "{HEAD}openapi:\n  transformations:\n    - command: oneOfToAnyOf\n      reason: prism\nresources:\n  a:\n    methods: {{ list: get /a }}\n"
    );
    let r = convert_text(&regression_spec(), &config).expect("convert");
    assert_eq!(errors(&r), Vec::<&ferrule_converter::Diagnostic>::new());
    assert!(!r.output.contains("\"oneOf\""));
    assert!(r.output.contains("\"anyOf\""));
}
