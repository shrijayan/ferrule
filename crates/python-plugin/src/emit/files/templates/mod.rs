//! Every project file emitted from a template: output path -> template text (tokens per
//! [`super::render`]). Port of `python-plugin/src/emit/files/templates/`.
//!
//! Each `<name>.txt` is derived from the file at the output path in
//! OneBusAway/python-sdk@4c20fa834c2a (Apache-2.0).
//! Modified by The Ferrule Authors: per-SDK values replaced by %%TOKENS%%, branding per
//! `python-plugin/src/emit/files/BRANDING.md`. The text is byte-identical to the TypeScript
//! templates, which `python-plugin/test/files/extract-templates.ts` generates; do not edit by
//! hand.

/// Output path -> template text, in the order of the TypeScript `TEMPLATES` map.
pub const TEMPLATES: &[(&str, &str)] = &[
    (
        ".devcontainer/Dockerfile",
        include_str!("dot_devcontainer_Dockerfile.txt"),
    ),
    (
        ".devcontainer/devcontainer.json",
        include_str!("dot_devcontainer_devcontainer_json.txt"),
    ),
    (
        ".github/workflows/ci.yml",
        include_str!("dot_github_workflows_ci_yml.txt"),
    ),
    (
        ".github/workflows/publish-pypi.yml",
        include_str!("dot_github_workflows_publish_pypi_yml.txt"),
    ),
    (
        ".github/workflows/release-doctor.yml",
        include_str!("dot_github_workflows_release_doctor_yml.txt"),
    ),
    (".gitignore", include_str!("dot_gitignore.txt")),
    (".python-version", include_str!("dot_python_version.txt")),
    (
        ".release-please-manifest.json",
        include_str!("dot_release_please_manifest_json.txt"),
    ),
    (".vscode/settings.json", include_str!("dot_vscode_settings_json.txt")),
    ("Brewfile", include_str!("Brewfile.txt")),
    ("CONTRIBUTING.md", include_str!("CONTRIBUTING_md.txt")),
    ("LICENSE", include_str!("LICENSE.txt")),
    ("README.md", include_str!("README_md.txt")),
    ("SECURITY.md", include_str!("SECURITY_md.txt")),
    (
        "bin/check-release-environment",
        include_str!("bin_check_release_environment.txt"),
    ),
    ("bin/publish-pypi", include_str!("bin_publish_pypi.txt")),
    ("examples/.keep", include_str!("examples_keep.txt")),
    ("noxfile.py", include_str!("noxfile_py.txt")),
    ("pyproject.toml", include_str!("pyproject_toml.txt")),
    (
        "release-please-config.json",
        include_str!("release_please_config_json.txt"),
    ),
    ("requirements-dev.lock", include_str!("requirements_dev_lock.txt")),
    ("requirements.lock", include_str!("requirements_lock.txt")),
    ("scripts/bootstrap", include_str!("scripts_bootstrap.txt")),
    ("scripts/format", include_str!("scripts_format.txt")),
    ("scripts/lint", include_str!("scripts_lint.txt")),
    ("scripts/mock", include_str!("scripts_mock.txt")),
    ("scripts/test", include_str!("scripts_test.txt")),
    (
        "scripts/utils/ruffen-docs.py",
        include_str!("scripts_utils_ruffen_docs_py.txt"),
    ),
    ("tests/__init__.py", include_str!("tests_init_py.txt")),
    (
        "tests/api_resources/__init__.py",
        include_str!("tests_api_resources_init_py.txt"),
    ),
    ("tests/conftest.py", include_str!("tests_conftest_py.txt")),
    ("tests/sample_file.txt", include_str!("tests_sample_file_txt.txt")),
    ("tests/test_client.py", include_str!("tests_test_client_py.txt")),
    (
        "tests/test_extract_files.py",
        include_str!("tests_test_extract_files_py.txt"),
    ),
    ("tests/test_files.py", include_str!("tests_test_files_py.txt")),
    ("tests/test_models.py", include_str!("tests_test_models_py.txt")),
    ("tests/test_qs.py", include_str!("tests_test_qs_py.txt")),
    (
        "tests/test_required_args.py",
        include_str!("tests_test_required_args_py.txt"),
    ),
    ("tests/test_response.py", include_str!("tests_test_response_py.txt")),
    ("tests/test_streaming.py", include_str!("tests_test_streaming_py.txt")),
    ("tests/test_transform.py", include_str!("tests_test_transform_py.txt")),
    (
        "tests/test_utils/test_datetime_parse.py",
        include_str!("tests_test_utils_test_datetime_parse_py.txt"),
    ),
    (
        "tests/test_utils/test_json.py",
        include_str!("tests_test_utils_test_json_py.txt"),
    ),
    (
        "tests/test_utils/test_path.py",
        include_str!("tests_test_utils_test_path_py.txt"),
    ),
    (
        "tests/test_utils/test_proxy.py",
        include_str!("tests_test_utils_test_proxy_py.txt"),
    ),
    (
        "tests/test_utils/test_typing.py",
        include_str!("tests_test_utils_test_typing_py.txt"),
    ),
    ("tests/utils.py", include_str!("tests_utils_py.txt")),
];
