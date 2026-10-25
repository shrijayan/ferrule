//! `--runtime-from <sdk>`: the adopted SDK's runtime decides the generator style the plugin
//! emits (`x-ferrule-style`). Reduced from the oracles' runtimes: honcho-python-core and
//! papr-python have `omit` but no `_utils/_path.py` and a `_client.py` without
//! `cached_property`; knock-python and opencode-sdk-python also have no `omit`; every newer
//! oracle has all three, and its annotated spec is the same as without the flag.

use ferrule_converter::cli::main;
use ferrule_converter::value::parse_json;
use ferrule_converter::{Map, Value};
use std::path::{Path, PathBuf};

const CONFIG: &str = "organization: { name: acme }
environments: { production: https://api.acme.test }
targets:
  python: { package_name: acme, production_repo: acme/acme-python, publish: { pypi: false } }
readme:
  example_requests:
    default: { type: request, endpoint: get /a, params: {} }
    headline: { type: request, endpoint: get /a, params: {} }
resources:
  a:
    methods:
      list: get /a
      create: post /a
";

const SPEC: &str = "openapi: 3.1.0
info: { title: acme, version: '1' }
paths:
  /a:
    get: { responses: {} }
    post: { responses: {} }
";

/// `_types.py` lines: the sentinels a runtime defines.
const TYPES_NOT_GIVEN: &str = "class NotGiven:\n    pass\n\n\nNotGivenOr = Union[_T, NotGiven]\nNOT_GIVEN = NotGiven()\n\n\nclass Omit:\n    pass\n";
const TYPES_OMIT: &str = "class NotGiven:\n    pass\n\n\nnot_given = NotGiven()\nNOT_GIVEN = NotGiven()\n\n\nclass Omit:\n    pass\n\n\nomit = Omit()\n";
const UTILS_OLD: &str = "from ._utils import is_given as is_given\n";
const UTILS_NEW: &str = "from ._path import path_template as path_template\nfrom ._utils import is_given as is_given\n";
const CLIENT_EAGER: &str = "class Acme(SyncAPIClient):\n    a: a.AResource\n\n    def __init__(self) -> None:\n        self.a = a.AResource(self)\n";
const CLIENT_LAZY: &str =
    "class Acme(SyncAPIClient):\n    @cached_property\n    def a(self) -> AResource:\n        return AResource(self)\n";

struct Run {
    code: i32,
    err: Vec<String>,
}

fn run(words: &[&str]) -> Run {
    let args: Vec<String> = words.iter().map(|s| (*s).to_owned()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = main(&args, &mut out, &mut err);
    Run {
        code,
        err: String::from_utf8_lossy(&err).lines().map(str::to_owned).collect(),
    }
}

fn s(p: &Path) -> &str {
    p.to_str().expect("utf-8 path")
}

/// A spec, a config and an SDK checkout whose runtime is `(types, utils, client)`.
fn fixture(name: &str, runtime: (&str, &str, &str)) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrule-style-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pkg = dir.join("sdk/src/acme");
    std::fs::create_dir_all(pkg.join("_utils")).expect("temp dir");
    std::fs::write(dir.join("openapi.yml"), SPEC).expect("spec");
    std::fs::write(dir.join("stainless.yml"), CONFIG).expect("config");
    std::fs::write(pkg.join("__init__.py"), "").expect("init");
    std::fs::write(pkg.join("_types.py"), runtime.0).expect("types");
    std::fs::write(pkg.join("_utils/__init__.py"), runtime.1).expect("utils");
    std::fs::write(pkg.join("_client.py"), runtime.2).expect("client");
    dir
}

/// Converts the fixture, with `--runtime-from` when `runtime` is set; the output text.
fn convert(dir: &Path, runtime: bool) -> String {
    let target = dir.join(if runtime { "styled.json" } else { "plain.json" });
    let (spec, config, sdk) = (dir.join("openapi.yml"), dir.join("stainless.yml"), dir.join("sdk"));
    let mut words = vec![s(&spec), s(&config), "-o", s(&target)];
    if runtime {
        words.extend(["--runtime-from", s(&sdk)]);
    }
    let r = run(&words);
    assert_eq!(r.code, 0, "{:?}", r.err);
    std::fs::read_to_string(&target).expect("output")
}

/// `x-ferrule-style` at the root and on each operation (`None` where it is absent).
fn styles(text: &str) -> Vec<Option<Value>> {
    let doc = parse_json(text).expect("json");
    let mut out = vec![doc.get("x-ferrule-style").cloned()];
    for verb in ["get", "post"] {
        let op = doc.get("paths").and_then(|p| p.get("/a")).and_then(|i| i.get(verb));
        out.push(op.and_then(|o| o.get("x-ferrule-style")).cloned());
    }
    out
}

/// The style object `pairs` spell, once for the root and each of the two operations.
fn everywhere(pairs: &[(&str, &str)]) -> Vec<Option<Value>> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(*k, Value::from(*v));
    }
    vec![Some(Value::Object(m)); 3]
}

#[test]
fn writes_the_style_of_a_runtime_without_omit() {
    let dir = fixture("knock", (TYPES_NOT_GIVEN, UTILS_OLD, CLIENT_EAGER));
    let want = everywhere(&[("paths", "fstring"), ("resources", "eager"), ("sentinel", "NOT_GIVEN")]);
    assert_eq!(styles(&convert(&dir, true)), want);
}

#[test]
fn writes_the_style_of_a_runtime_without_path_template() {
    let dir = fixture("honcho", (TYPES_OMIT, UTILS_OLD, CLIENT_EAGER));
    let want = everywhere(&[("paths", "fstring"), ("resources", "eager")]);
    assert_eq!(styles(&convert(&dir, true)), want);
}

#[test]
fn leaves_the_spec_as_is_for_todays_runtime() {
    let dir = fixture("today", (TYPES_OMIT, UTILS_NEW, CLIENT_LAZY));
    assert_eq!(convert(&dir, true), convert(&dir, false));
}

#[test]
fn refuses_a_runtime_without_a_package() {
    let dir = fixture("nopkg", (TYPES_OMIT, UTILS_NEW, CLIENT_LAZY));
    std::fs::remove_file(dir.join("sdk/src/acme/__init__.py")).expect("remove");
    let target = dir.join("out.json");
    let (spec, config, sdk) = (dir.join("openapi.yml"), dir.join("stainless.yml"), dir.join("sdk"));
    let r = run(&[s(&spec), s(&config), "-o", s(&target), "--runtime-from", s(&sdk)]);
    assert_eq!(r.code, 2);
    assert!(r.err[0].starts_with("ferrule-convert: "), "{:?}", r.err);
    assert!(r.err[0].contains("--runtime-from"), "{:?}", r.err);
    assert!(!target.exists());
}

/// honcho-python-core's `_files.py` has no `deepcopy_with_paths`: its multipart methods copy the
/// body with `_utils.deepcopy_minimal`. A `_files.py` with it (ogx-client-python) changes nothing.
#[test]
fn writes_the_multipart_copy_of_a_runtime_without_deepcopy_with_paths() {
    let dir = fixture("multipart", (TYPES_OMIT, UTILS_NEW, CLIENT_LAZY));
    let files = dir.join("sdk/src/acme/_files.py");
    std::fs::write(&files, "def to_httpx_files(files):\n    return files\n").expect("files");
    assert_eq!(
        styles(&convert(&dir, true)),
        everywhere(&[("multipart", "deepcopy_minimal")])
    );
    std::fs::write(&files, "def deepcopy_with_paths(item, paths):\n    return item\n").expect("files");
    assert_eq!(convert(&dir, true), convert(&dir, false));
}
