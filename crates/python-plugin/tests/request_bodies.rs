//! Request bodies that are not a plain object (`tests/request_bodies/<shape>/`), each emitted as
//! Stainless does. Every file under `tests/golden/request-bodies/<shape>/src/` is compared byte for
//! byte with the generated one; the other generated files are not pinned here. The expected files
//! follow the oracle SDKs named in `tests/request_bodies/README.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferrule_python_plugin::plugin::code_files;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn expected(shape: &str) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).expect("golden directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, fs::read_to_string(&path).expect("golden file"));
            }
        }
    }
    let root = dir().join("golden/request-bodies").join(shape);
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

fn generated(shape: &str) -> BTreeMap<String, String> {
    let spec = fs::read(dir().join("request_bodies").join(shape).join("annotated.json")).expect("fixture");
    let forge: forge::Forge = serde_json::from_slice(&spec).expect("an OpenAPI document Forge reads");
    code_files(&forge, None)
        .unwrap_or_else(|e| panic!("{shape} does not generate: {e}"))
        .into_iter()
        .collect()
}

fn check(shape: &str) {
    let actual = generated(shape);
    let expected = expected(shape);
    assert!(!expected.is_empty(), "no golden files for {shape}");
    for (path, want) in &expected {
        let got = actual.get(path).unwrap_or_else(|| panic!("{path} is not generated"));
        assert_eq!(got, want, "{path}");
    }
}

/// `anyOf: [$ref Object, null]` (honcho `workspaces.list`): the object's fields are keyword args.
#[test]
fn a_nullable_object_body_becomes_keyword_arguments() {
    check("nullable_object");
}

/// A map or array body (honcho `sessions.peers.add/remove`): one `body` keyword argument.
#[test]
fn a_map_or_array_body_is_one_body_argument() {
    check("map_and_array");
}

/// A union of objects (knock-mgmt `guides.activate`, imagekit `accounts.origins.create`): an
/// `@overload` per variant, `@required_args`, and a `TypedDict` per variant.
#[test]
fn a_union_body_is_one_overload_per_variant() {
    check("union_of_objects");
}

/// A `multipart/form-data` body (honcho `sessions.messages.upload`) is sent as multipart with
/// its binary fields as files, as ogx-client-python `files.create` writes it. Only the request is
/// pinned: Stainless types the binary field `FileTypes`, which needs a type the project-file
/// emitters know too (the field is still `object`).
#[test]
fn a_multipart_body_sends_its_binary_fields_as_files() {
    let files = generated("multipart");
    let resource = files.get("src/fx/resources/messages.py").expect("messages.py");
    let sync = r#"        if not session_id:
            raise ValueError(f"Expected a non-empty value for `session_id` but received {session_id!r}")
        body = deepcopy_with_paths(
            {
                "file": file,
                "peer_id": peer_id,
            },
            [["file"]],
        )
        files = extract_files(cast(Mapping[str, object], body), paths=[["file"]])
        # It should be noted that the actual Content-Type header that will be
        # sent to the server will contain a `boundary` parameter, e.g.
        # multipart/form-data; boundary=---abc--
        extra_headers = {"Content-Type": "multipart/form-data", **(extra_headers or {})}
        return self._post(
            path_template("/sessions/{session_id}/messages/upload", session_id=session_id),
            body=maybe_transform(body, message_upload_params.MessageUploadParams),
            files=files,
            options=make_request_options(
                extra_headers=extra_headers, extra_query=extra_query, extra_body=extra_body, timeout=timeout
            ),
            cast_to=MessageUploadResponse,
        )
"#;
    assert!(resource.contains(sync), "{resource}");
    assert!(
        resource.contains(
            "            body=await async_maybe_transform(body, message_upload_params.MessageUploadParams),\n            files=files,\n"
        ),
        "{resource}"
    );
    for import in [
        "from typing import Mapping, cast\n",
        "from .._files import deepcopy_with_paths\n",
        "from .._utils import extract_files, path_template, maybe_transform, async_maybe_transform\n",
    ] {
        assert!(resource.contains(import), "{import}");
    }
}

/// An `application/octet-stream` body (the opencode SDK's `file.write`, reduced in
/// `tests/request_bodies/README.md`): one required keyword argument `body`, any file content or
/// bytes, sent as the raw request content with the body's media type. The runtime takes raw bytes
/// through `content=` (passing them as `body=` is deprecated there); a path is read first.
#[test]
fn a_binary_body_is_sent_as_raw_content() {
    let files = generated("binary");
    let resource = files.get("src/fx/resources/file.py").expect("file.py");
    let sync = "        body: FileContent | BinaryTypes,\n        path: str,\n";
    let request = r#"        extra_headers = {"Content-Type": "application/octet-stream", **(extra_headers or {})}
        return self._post(
            "/fs/write",
            content=read_file_content(body) if isinstance(body, os.PathLike) else body,
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=maybe_transform({"path": path}, file_write_params.FileWriteParams),
            ),
            cast_to=FileWriteResponse,
        )
"#;
    let async_request = r#"            content=await async_read_file_content(body) if isinstance(body, os.PathLike) else body,
            options=make_request_options(
                extra_headers=extra_headers,
                extra_query=extra_query,
                extra_body=extra_body,
                timeout=timeout,
                query=await async_maybe_transform({"path": path}, file_write_params.FileWriteParams),
            ),
"#;
    for want in [
        sync,
        request,
        async_request,
        "        body: FileContent | AsyncBinaryTypes,\n",
    ] {
        assert!(resource.contains(want), "{want}\n---\n{resource}");
    }
    for import in [
        "import os\n",
        "from .._files import read_file_content, async_read_file_content\n",
    ] {
        assert!(resource.contains(import), "{import}\n---\n{resource}");
    }
    let params = files
        .get("src/fx/types/file_write_params.py")
        .expect("file_write_params.py");
    assert!(!params.contains("body"), "{params}");
}

/// A parameter named like a request option (the opencode SDK's `shell.create` body field
/// `timeout`) is prefixed `api_`, as browserbase-python's `sessions.create(api_timeout=...)`
/// sends `"timeout"`: the params `TypedDict` declares it with the wire name as its alias.
#[test]
fn a_parameter_named_like_a_request_option_is_prefixed_api() {
    let files = generated("option_names");
    let resource = files.get("src/fx/resources/shell.py").expect("shell.py");
    for want in [
        "        api_extra_query: str | Omit = omit,\n",
        "        api_timeout: int | Omit = omit,\n",
        "          api_timeout: Seconds before the command is stopped\n",
        "                    \"api_timeout\": api_timeout,\n",
        "                query=maybe_transform({\"api_extra_query\": api_extra_query}, shell_create_params.ShellCreateParams),\n",
    ] {
        assert!(resource.contains(want), "{want}\n---\n{resource}");
    }
    assert_eq!(resource.matches("\n        timeout: ").count(), 2, "{resource}");
    let params = files.get("src/fx/types/shell_create_params.py").expect("params");
    assert!(
        params.contains("    api_timeout: Annotated[int, PropertyInfo(alias=\"timeout\")]\n"),
        "{params}"
    );
}
