# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import stat
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath

import pytest

from ferrule.cli import main
from ferrule.errors import FerruleError
from ferrule.generate_jvm import (
    JVM_RUNTIME_OPTIONS,
    KTFMT_BATCH,
    JvmRequest,
    generate_jvm,
    ktfmt_steps,
    load_java_renderer,
    plan_jvm_steps,
    resolve_ktfmt,
)

OPTIONS = {
    "package_name": "com.acme.api",
    "client_name": "Acme",
    "artifact_name": "acme-java",
    "repo": "acme/acme-java",
    "target": "java",
    "array_format": "comma",
    "empty_responses": False,
}


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]], *, ktfmt_fails: bool = False):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "build.gradle.kts", 'allprojects {\n    group = "com.acme.api"\n}\n')
            write(out / "acme-java-core/src/main/kotlin/com/acme/api/client/AcmeClient.kt", "x\n")
            write(out / "scripts/test", "#!/usr/bin/env bash\n")
            write(out / JVM_RUNTIME_OPTIONS, json.dumps(OPTIONS))
        if "--kotlinlang-style" in argv and ktfmt_fails:
            return subprocess.CompletedProcess(argv, 1, "", "AcmeClient.kt:1:1: error: Expecting\n")
        return subprocess.CompletedProcess(argv, 0, "", "")

    return run


def renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
    assert options == OPTIONS
    return {"acme-java-core/src/main/kotlin/com/acme/api/core/Values.kt": "package x\n"}


@pytest.fixture
def request_(tmp_path: Path) -> JvmRequest:
    return JvmRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        gen=write(tmp_path / "tools/ferrule-gen"),
        target="java",
        ktfmt=(sys.executable,),
    )


def test_plans_convert_then_generate_for_the_target(request_: JvmRequest, tmp_path: Path) -> None:
    convert, gen = plan_jvm_steps(request_, tmp_path / "work")
    assert convert.argv[1] == "convert"
    assert gen.argv[1:] == (
        "generate",
        str(tmp_path / "work/annotated.json"),
        "--target",
        "java",
        "--out",
        str(request_.out),
    )


def test_runs_ktfmt_in_batches(request_: JvmRequest) -> None:
    files = [PurePosixPath(f"a{i}.kt") for i in range(KTFMT_BATCH + 1)]
    steps = ktfmt_steps(request_, files)
    assert len(steps) == 2
    assert steps[0].argv[:2] == (sys.executable, "--kotlinlang-style")
    assert steps[1].argv[2:] == (str(request_.out / f"a{KTFMT_BATCH}.kt"),)


def test_resolves_ktfmt_from_the_environment() -> None:
    assert resolve_ktfmt({}) == ("ktfmt",)
    assert resolve_ktfmt({"FERRULE_KTFMT": "/opt/ktfmt"}) == ("/opt/ktfmt",)
    jar = {"FERRULE_KTFMT_JAR": "/k.jar", "FERRULE_JAVA": "/jdk/java"}
    assert resolve_ktfmt(jar) == ("/jdk/java", "-jar", "/k.jar")


def test_formats_the_plugin_kotlin_then_renders_the_runtime(request_: JvmRequest) -> None:
    calls: list[tuple[str, ...]] = []
    result = generate_jvm(request_, fake_runner(calls), renderer)
    assert [c[1] for c in calls] == ["convert", "generate", "--kotlinlang-style"]
    formatted = calls[2][2:]
    assert formatted == (
        str(request_.out / "acme-java-core/src/main/kotlin/com/acme/api/client/AcmeClient.kt"),
    )
    assert result.package == "com.acme.api"
    assert result.runtime_files == 1
    assert not (request_.out / ".ferrule").exists()
    assert (request_.out / "scripts/test").stat().st_mode & stat.S_IXUSR
    assert result.files == 4


def test_fails_when_ktfmt_fails(request_: JvmRequest) -> None:
    with pytest.raises(FerruleError, match="ktfmt failed"):
        generate_jvm(request_, fake_runner([], ktfmt_fails=True), renderer)


def test_needs_the_java_renderer(request_: JvmRequest) -> None:
    with pytest.raises(FerruleError, match="render_java_files is unavailable"):
        generate_jvm(request_, fake_runner([]), None)


def test_loads_the_runtime_renderer() -> None:
    render = load_java_renderer()
    assert render is not None
    files = render(OPTIONS)
    values = "acme-java-core/src/main/kotlin/com/acme/api/core/Values.kt"
    assert files[values].startswith("package com.acme.api.core\n")


def test_rejects_runtime_from_for_jvm_targets(tmp_path: Path) -> None:
    code = main(
        [
            "generate",
            "--spec",
            str(tmp_path / "a.yml"),
            "--config",
            str(tmp_path / "b.yml"),
            "--out",
            str(tmp_path / "out"),
            "--target",
            "kotlin",
            "--runtime-from",
            str(tmp_path),
        ]
    )
    assert code == 1
