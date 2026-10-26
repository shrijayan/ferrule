//! The module files: `go.mod`, `go.sum` (the runtime's two dependencies, as every Stainless Go
//! SDK of a layout pins them) and the `.keep` files of the directories code generation leaves
//! alone.

use super::Gen;

/// `go.sum` of Stainless's Go v1 SDKs (OneBusAway go-sdk, knock-go): `tidwall/gjson` and `sjson`.
const GO_SUM: &str = include_str!("go.sum.txt");

/// `go.sum` of Stainless's Go v2 SDKs (knock-mgmt-go): `gjson` 1.18.0.
const GO_SUM_V2: &str = include_str!("go.sum.v2.txt");

/// The Go version `go.mod` declares (Stainless's 2026 Go v1 generation).
const GO_VERSION: &str = "1.22";

fn keep(purpose: &str) -> String {
    format!(
        "File generated from our OpenAPI spec by Ferrule.\n\nThis directory can be used to store {purpose}.\n\
         It is ignored by Ferrule code generation and its content (other than this keep file) won't be touched."
    )
}

/// `go.mod`, `go.sum`, `examples/.keep` and `lib/.keep`.
#[must_use]
pub fn project_files(g: &Gen<'_>) -> Vec<(String, String)> {
    let (gjson, sum) = if g.target.v2() {
        ("v1.18.0", GO_SUM_V2)
    } else {
        ("v1.14.4", GO_SUM)
    };
    let go_mod = format!(
        "module {}\n\ngo {GO_VERSION}\n\nrequire (\n\tgithub.com/tidwall/gjson {gjson}\n\tgithub.com/tidwall/sjson v1.2.5\n)\n\n\
         require (\n\tgithub.com/tidwall/match v1.1.1 // indirect\n\tgithub.com/tidwall/pretty v1.2.1 // indirect\n)\n",
        g.target.module_path
    );
    vec![
        ("go.mod".to_owned(), go_mod),
        ("go.sum".to_owned(), sum.to_owned()),
        (
            "examples/.keep".to_owned(),
            keep("example files demonstrating usage of this SDK"),
        ),
        ("lib/.keep".to_owned(), keep("custom files to expand the SDK")),
    ]
}
