//! ADR 0002: Forge stays behind the adapter. Port of `test/boundary.test.ts`.

use std::fs;
use std::path::{Path, PathBuf};

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).expect("directory") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            out.extend(files(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn only_the_adapter_and_the_plugin_entry_name_forge() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = files(&src)
        .into_iter()
        .map(|f| {
            f.strip_prefix(&src)
                .expect("under src")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .filter(|f| !f.starts_with("adapter/") && f != "plugin.rs")
        .filter(|f| {
            let text = fs::read_to_string(src.join(f)).expect("source file");
            text.contains("forge::") || text.contains("use forge")
        })
        .collect();
    offenders.sort();
    assert_eq!(offenders, Vec::<String>::new());
}
