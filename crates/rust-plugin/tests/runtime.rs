//! The runtime the plugin embeds is the whole template (`runtime/rust/template`), unchanged.

mod common;

use ferrule_rust_plugin::runtime::FILES;

#[test]
fn embeds_every_template_file() {
    let template = common::root().join("runtime/rust/template");
    let embedded: Vec<&str> = FILES.iter().map(|(p, _)| *p).collect();
    assert_eq!(common::walk(&template), embedded);
    for (path, text) in FILES {
        let on_disk = std::fs::read_to_string(template.join(path)).expect("template file");
        assert_eq!(on_disk, text, "{path}");
    }
}
