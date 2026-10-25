//! Agent context for onebusaway. The golden comparison is in `tests/golden.rs`; every example call
//! renders its arguments with `api_tests::example_value`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use ferrule_python_plugin::emit::agent::{agent_context, emit_agent_files};
use ferrule_python_plugin::emit::files_input::placeholder_meta;
use ferrule_python_plugin::model::{
    PaginationScheme, ParamLocation, SdkClient, SdkPagination, SdkStreaming, SdkType, StreamFormat,
};
use ferrule_python_plugin::plugin::to_model;

fn model() -> SdkClient {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/onebusaway.json");
    let forge: forge::Forge = serde_json::from_slice(&fs::read(path).expect("fixture")).expect("a document");
    to_model(&forge).expect("converts").expect("has operations")
}

/// Every `python` fenced block of a Markdown file.
fn python_blocks(md: &str) -> Vec<String> {
    md.split("```python\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("```").map(|(code, _)| code.to_owned()))
        .collect()
}

/// Python sources that `ast.parse` rejects, with the error.
fn unparsable(sources: &[String]) -> Vec<String> {
    let script = "import ast, json, sys\nfor i, s in enumerate(json.load(sys.stdin)):\n    try: ast.parse(s)\n    except SyntaxError as e: print(i, e)\n";
    let mut child = Command::new("python3")
        .args(["-I", "-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("python3");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(serde_json::to_string(sources).expect("json").as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("runs");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn get<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files.iter().find(|(p, _)| p == path).map_or("", |(_, t)| t.as_str())
}

#[test]
fn emits_nothing_for_a_client_without_settings() {
    let mut client = model();
    client.settings = None;
    assert_eq!(emit_agent_files(&client, Some(&placeholder_meta())), Vec::new());
}

#[test]
fn ships_the_skill_agents_md_and_llms_txt_inside_the_package() {
    let files = emit_agent_files(&model(), Some(&placeholder_meta()));
    for name in ["SKILL.md", "AGENTS.md", "llms.txt"] {
        assert!(files.iter().any(|(p, _)| *p == format!("src/onebusaway/_agent/{name}")));
    }
    assert_eq!(
        get(&files, "skills/onebusaway/SKILL.md"),
        get(&files, "src/onebusaway/_agent/SKILL.md")
    );
    let skill = get(&files, "src/onebusaway/_agent/SKILL.md");
    assert!(skill.starts_with("---\nname: onebusaway\ndescription: "));
}

#[test]
fn writes_python_that_parses() {
    let files = emit_agent_files(&model(), Some(&placeholder_meta()));
    let blocks = python_blocks(get(&files, "src/onebusaway/_agent/SKILL.md"));
    assert!(blocks.len() > 3);
    let py: Vec<String> = files
        .iter()
        .filter(|(p, _)| Path::new(p).extension().is_some_and(|e| e == "py"))
        .map(|(_, t)| t.clone())
        .collect();
    assert!(py.len() > 1);
    assert_eq!(unparsable(&[blocks, py].concat()), Vec::<String>::new());
}

#[test]
fn emits_only_the_package_copies_without_project_metadata() {
    let paths: Vec<String> = emit_agent_files(&model(), None).into_iter().map(|(p, _)| p).collect();
    assert_eq!(
        paths,
        [
            "src/onebusaway/_agent/SKILL.md",
            "src/onebusaway/_agent/AGENTS.md",
            "src/onebusaway/_agent/llms.txt"
        ]
    );
}

#[test]
fn is_deterministic() {
    let meta = placeholder_meta();
    assert_eq!(
        emit_agent_files(&model(), Some(&meta)),
        emit_agent_files(&model(), Some(&meta))
    );
}

#[test]
fn documents_pagination_and_streaming_when_a_method_has_them() {
    let mut client = model();
    let first = &mut client.resources[0];
    let method = first.methods[0].clone();
    let mut paged = method.clone();
    paged.name = "list".into();
    paged.pagination = Some(SdkPagination {
        scheme: PaginationScheme::Cursor,
        page: "CursorPage".into(),
        request: Vec::new(),
        response: Vec::new(),
        param_location: ParamLocation::Query,
        headers: Vec::new(),
        item: SdkType::Unknown,
    });
    let mut streamed = method;
    streamed.name = "stream".into();
    streamed.streaming = Some(SdkStreaming {
        format: StreamFormat::Sse,
        param_discriminator: Some("stream".into()),
        event: None,
    });
    first.methods.splice(0..0, [paged, streamed]);
    let c = agent_context(&client, None).expect("a context");
    assert!(
        c.pagination
            .expect("pagination")
            .body
            .join("\n")
            .contains("for item in client.")
    );
    assert!(c.streaming.expect("streaming").body[0].contains("stream=True"));
    let files = emit_agent_files(&client, None);
    let skill = get(&files, "src/onebusaway/_agent/SKILL.md");
    assert!(skill.contains("## Pagination"));
    assert!(skill.contains("## Streaming"));
    assert_eq!(unparsable(&python_blocks(skill)), Vec::<String>::new());
}

/// The streaming snippet of a param-switched method passes the switch by its Python name
/// (stagehand `sessions.act(..., stream_response=True)`), not the wire name `streamResponse`.
#[test]
fn switches_streaming_on_by_the_python_name() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/headers/params/annotated.json");
    let forge: forge::Forge = serde_json::from_slice(&fs::read(path).expect("fixture")).expect("a document");
    let client = to_model(&forge).expect("converts").expect("has operations");
    let c = agent_context(&client, None).expect("a context");
    let body = c.streaming.expect("streaming").body.join("\n");
    assert!(body.contains("stream_response=True"), "{body}");
    assert!(!body.contains("streamResponse"), "{body}");
}
