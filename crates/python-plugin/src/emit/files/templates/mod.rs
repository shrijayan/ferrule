//! Every project file emitted from a template: output path -> template text (tokens per
//! [`super::render`]). Port of `python-plugin/src/emit/files/templates/`.
//!
//! Each `<name>.tmpl` is derived from the file at the output path in
//! OneBusAway/python-sdk@4c20fa834c2a (Apache-2.0).
//! Modified by The Ferrule Authors: per-SDK values replaced by %%TOKENS%%, branding per
//! `src/emit/files/BRANDING.md`. The text is byte-identical to the TypeScript templates it was
//! ported from; do not edit by hand. The `.tmpl` extension keeps GitHub's dependency graph from
//! reading the generated SDK's `requirements*.lock` templates as Ferrule's own manifests.

/// Output path -> template text, in the order of the TypeScript `TEMPLATES` map.
pub const TEMPLATES: &[(&str, &str)] = &[
    (
        ".devcontainer/Dockerfile",
        include_str!("dot_devcontainer_Dockerfile.tmpl"),
    ),
    (
        ".devcontainer/devcontainer.json",
        include_str!("dot_devcontainer_devcontainer_json.tmpl"),
    ),
    (
        ".github/workflows/ci.yml",
        include_str!("dot_github_workflows_ci_yml.tmpl"),
    ),
    (
        ".github/workflows/publish-pypi.yml",
        include_str!("dot_github_workflows_publish_pypi_yml.tmpl"),
    ),
    (
        ".github/workflows/release-doctor.yml",
        include_str!("dot_github_workflows_release_doctor_yml.tmpl"),
    ),
    (".gitignore", include_str!("dot_gitignore.tmpl")),
    (".python-version", include_str!("dot_python_version.tmpl")),
    (
        ".release-please-manifest.json",
        include_str!("dot_release_please_manifest_json.tmpl"),
    ),
    (".vscode/settings.json", include_str!("dot_vscode_settings_json.tmpl")),
    ("Brewfile", include_str!("Brewfile.tmpl")),
    ("CONTRIBUTING.md", include_str!("CONTRIBUTING_md.tmpl")),
    ("LICENSE", include_str!("LICENSE.tmpl")),
    ("README.md", include_str!("README_md.tmpl")),
    ("SECURITY.md", include_str!("SECURITY_md.tmpl")),
    (
        "bin/check-release-environment",
        include_str!("bin_check_release_environment.tmpl"),
    ),
    ("bin/publish-pypi", include_str!("bin_publish_pypi.tmpl")),
    ("examples/.keep", include_str!("examples_keep.tmpl")),
    ("noxfile.py", include_str!("noxfile_py.tmpl")),
    ("pyproject.toml", include_str!("pyproject_toml.tmpl")),
    (
        "release-please-config.json",
        include_str!("release_please_config_json.tmpl"),
    ),
    ("requirements-dev.lock", include_str!("requirements_dev_lock.tmpl")),
    ("requirements.lock", include_str!("requirements_lock.tmpl")),
    ("scripts/bootstrap", include_str!("scripts_bootstrap.tmpl")),
    ("scripts/format", include_str!("scripts_format.tmpl")),
    ("scripts/lint", include_str!("scripts_lint.tmpl")),
    ("scripts/mock", include_str!("scripts_mock.tmpl")),
    ("scripts/test", include_str!("scripts_test.tmpl")),
    (
        "scripts/utils/ruffen-docs.py",
        include_str!("scripts_utils_ruffen_docs_py.tmpl"),
    ),
    ("tests/__init__.py", include_str!("tests_init_py.tmpl")),
    (
        "tests/api_resources/__init__.py",
        include_str!("tests_api_resources_init_py.tmpl"),
    ),
    ("tests/conftest.py", include_str!("tests_conftest_py.tmpl")),
    ("tests/sample_file.txt", include_str!("tests_sample_file_txt.tmpl")),
    ("tests/test_client.py", include_str!("tests_test_client_py.tmpl")),
    (
        "tests/test_extract_files.py",
        include_str!("tests_test_extract_files_py.tmpl"),
    ),
    ("tests/test_files.py", include_str!("tests_test_files_py.tmpl")),
    ("tests/test_models.py", include_str!("tests_test_models_py.tmpl")),
    ("tests/test_qs.py", include_str!("tests_test_qs_py.tmpl")),
    (
        "tests/test_required_args.py",
        include_str!("tests_test_required_args_py.tmpl"),
    ),
    ("tests/test_response.py", include_str!("tests_test_response_py.tmpl")),
    ("tests/test_streaming.py", include_str!("tests_test_streaming_py.tmpl")),
    ("tests/test_transform.py", include_str!("tests_test_transform_py.tmpl")),
    (
        "tests/test_utils/test_datetime_parse.py",
        include_str!("tests_test_utils_test_datetime_parse_py.tmpl"),
    ),
    (
        "tests/test_utils/test_json.py",
        include_str!("tests_test_utils_test_json_py.tmpl"),
    ),
    (
        "tests/test_utils/test_path.py",
        include_str!("tests_test_utils_test_path_py.tmpl"),
    ),
    (
        "tests/test_utils/test_proxy.py",
        include_str!("tests_test_utils_test_proxy_py.tmpl"),
    ),
    (
        "tests/test_utils/test_typing.py",
        include_str!("tests_test_utils_test_typing_py.tmpl"),
    ),
    ("tests/utils.py", include_str!("tests_utils_py.tmpl")),
];
