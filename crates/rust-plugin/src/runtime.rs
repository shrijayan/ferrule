//! The runtime template (`runtime/rust/template`, ADR 0019), embedded so that `generate` gives
//! a crate that builds. The template has no tokens or directives: rendering it with the shared
//! engine (`ferrule_runtime.render_rust_files`) gives these files unchanged, which
//! `runtime/tests/test_render_rust.py` checks, and `tests/runtime.rs` checks this list against
//! the template directory.

/// Every runtime file, by path from the crate root.
pub const FILES: [(&str, &str); 10] = [
    (
        "src/rt/error.rs",
        include_str!("../../../runtime/rust/template/src/rt/error.rs"),
    ),
    (
        "src/rt/http.rs",
        include_str!("../../../runtime/rust/template/src/rt/http.rs"),
    ),
    (
        "src/rt/mod.rs",
        include_str!("../../../runtime/rust/template/src/rt/mod.rs"),
    ),
    (
        "src/rt/multipart.rs",
        include_str!("../../../runtime/rust/template/src/rt/multipart.rs"),
    ),
    (
        "src/rt/pagination.rs",
        include_str!("../../../runtime/rust/template/src/rt/pagination.rs"),
    ),
    (
        "src/rt/query.rs",
        include_str!("../../../runtime/rust/template/src/rt/query.rs"),
    ),
    (
        "src/rt/request.rs",
        include_str!("../../../runtime/rust/template/src/rt/request.rs"),
    ),
    (
        "src/rt/streaming.rs",
        include_str!("../../../runtime/rust/template/src/rt/streaming.rs"),
    ),
    (
        "src/testing/mod.rs",
        include_str!("../../../runtime/rust/template/src/testing/mod.rs"),
    ),
    (
        "src/testing/server.rs",
        include_str!("../../../runtime/rust/template/src/testing/server.rs"),
    ),
];
