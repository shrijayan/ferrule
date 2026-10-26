//! Forge stays behind the adapter: no file of this crate's `src/` names Forge (ADR 0002,
//! ADR 0017). The plugin reads `ferrule_sdk_model` only.

use std::path::Path;

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable src") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_source_file_imports_forge() {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    assert_ne!(files.len(), 0, "no sources found");
    for f in files {
        let text = std::fs::read_to_string(&f).expect("readable");
        for (i, line) in text.lines().enumerate() {
            let code = line.trim_start();
            assert!(
                !(code.starts_with("use forge") || code.contains("forge::") || code.starts_with("extern crate forge")),
                "{}:{} names Forge: {line}",
                f.display(),
                i + 1
            );
        }
    }
}
