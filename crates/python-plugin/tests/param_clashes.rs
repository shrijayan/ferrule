//! Methods whose parameters would share a Python name, a `SyntaxError` that sinks every test
//! importing the module (Tier 1 corpus run, 2026-10-26). Each fixture directory's README says
//! where it was reduced from and which Stainless SDK shows the expected output.

use std::fs;
use std::path::PathBuf;

use ferrule_python_plugin::plugin::code_files;

fn generate(fixture: &str) -> Vec<(String, String)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join(fixture).join("annotated.json");
    let spec = fs::read(path).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None).unwrap_or_else(|e| panic!("does not generate: {e}"))
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files.iter().find(|(p, _)| p == path).map_or_else(|| panic!("no {path}"), |(_, t)| t.as_str())
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn merges_an_allof_property_declared_twice_into_one_parameter() {
    let files = generate("allof_merge");
    let resource = file(&files, "src/fx/resources/completions.py");
    // Once in the sync and once in the async method.
    assert_eq!(count(resource, "        top_logprobs: Optional[int] | Omit = omit,\n"), 2, "{resource}");
    assert_eq!(count(resource, "top_logprobs: int"), 0, "{resource}");
    assert_eq!(count(resource, "top_logprobs: Outer description."), 2, "{resource}");
    let params = file(&files, "src/fx/types/completion_create_params.py");
    assert_eq!(count(params, "    top_logprobs: Optional[int]\n    \"\"\"Outer description.\"\"\"\n"), 1, "{params}");
    assert_eq!(count(params, "top_logprobs:"), 1, "{params}");
}

#[test]
fn prefixes_parameters_that_share_a_name_across_locations() {
    let files = generate("location_clash");
    let zones = file(&files, "src/fx/resources/zones.py");
    for want in [
        "        self,\n        path_name: str,\n        *,\n        body_name: str,\n        contact: str | Omit = omit,\n",
        "          body_name: name of DNS zone\n",
        "if not path_name:",
        "path_template(\"/zones/{path_name}\", path_name=path_name)",
        "                    \"body_name\": body_name,\n",
    ] {
        assert_eq!(count(zones, want), 2, "{want}\n{zones}");
    }
    let params = file(&files, "src/fx/types/zone_replace_params.py");
    assert!(
        params.contains("    body_name: Required[Annotated[str, PropertyInfo(alias=\"name\")]]\n"),
        "{params}"
    );

    let recipients = file(&files, "src/fx/resources/recipients.py");
    let want = "        share_id: str,\n        *,\n        path_account_id: str,\n        body_account_id: str | Omit = omit,\n";
    assert_eq!(count(recipients, want), 2, "{recipients}");
    assert_eq!(count(recipients, "\"/accounts/{path_account_id}/shares/{share_id}/recipients\""), 2, "{recipients}");
    let params = file(&files, "src/fx/types/recipient_create_params.py");
    for want in [
        "    path_account_id: Required[Annotated[str, PropertyInfo(alias=\"account_id\")]]\n",
        "    body_account_id: Annotated[str, PropertyInfo(alias=\"account_id\")]\n",
    ] {
        assert!(params.contains(want), "{want}\n{params}");
    }

    let pods = file(&files, "src/fx/resources/pods.py");
    for want in ["        body_dry_run: SequenceNotStr[str] | Omit = omit,\n", "        query_dry_run: str | Omit = omit,\n"] {
        assert_eq!(count(pods, want), 2, "{want}\n{pods}");
    }
    assert_eq!(count(pods, "\"query_dry_run\": query_dry_run"), 2, "{pods}");
    let params = file(&files, "src/fx/types/pod_delete_params.py");
    for want in [
        "    body_dry_run: Annotated[SequenceNotStr[str], PropertyInfo(alias=\"dryRun\")]\n",
        "    query_dry_run: Annotated[str, PropertyInfo(alias=\"dryRun\")]\n",
    ] {
        assert!(params.contains(want), "{want}\n{params}");
    }
}
