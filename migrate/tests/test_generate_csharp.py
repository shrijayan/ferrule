# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path

import pytest

from ferrule.cli import build_parser
from ferrule.errors import FerruleError
from ferrule.generate_csharp import (
    CSHARP_RUNTIME_OPTIONS,
    FORMATTER_BIN,
    CSharpRequest,
    generate_csharp,
    load_csharp_renderer,
    plan_csharp_steps,
)
from ferrule.generate_ruby import resolve_formatter

OPTIONS = {
    "namespace": "Acme",
    "client_name": "Acme",
    "env_prefix": "ACME",
    "auth_headers": "",
}


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]], seen: dict[str, list[str]], fail_on: str = ""):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/Acme/AcmeClient.cs", "namespace Acme;\n")
            write(out / ".editorconfig", "root = true\n")
            write(out / CSHARP_RUNTIME_OPTIONS, json.dumps(OPTIONS))
        if "format" in argv:
            # What csharpier is handed: the generated tree, without the runtime or its options.
            root = Path(argv[-1])
            seen["formatted"] = sorted(
                p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()
            )
        code = 1 if fail_on and fail_on in argv else 0
        return subprocess.CompletedProcess(argv, code, "", "boom" if code else "")

    return run


def renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
    assert options == OPTIONS
    return {
        "src/Acme/Core/ApiEnum.cs": "namespace Acme.Core;\r\n",
        "src/Acme/Exceptions/AcmeException.cs": "",
    }


@pytest.fixture
def request_(tmp_path: Path) -> CSharpRequest:
    return CSharpRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        gen=write(tmp_path / "tools/ferrule-gen"),
        csharpier=(sys.executable, "-m", "csharpier"),
    )


def test_plans_convert_generate_then_csharpier(request_: CSharpRequest, tmp_path: Path) -> None:
    convert, gen, csharpier = plan_csharp_steps(request_, tmp_path / "work")
    assert convert.argv[1] == "convert"
    assert gen.argv[1:] == (
        "generate",
        str(tmp_path / "work/annotated.json"),
        "--target",
        "csharp",
        "--out",
        str(request_.out),
    )
    assert csharpier.argv[-2:] == ("format", str(request_.out))


def test_formats_then_renders_the_runtime(request_: CSharpRequest) -> None:
    calls: list[tuple[str, ...]] = []
    seen: dict[str, list[str]] = {}
    result = generate_csharp(request_, fake_runner(calls, seen), renderer)
    assert [c[1] if c[0] != sys.executable else c[3] for c in calls] == [
        "convert",
        "generate",
        "format",
    ]
    assert seen["formatted"] == [".editorconfig", "src/Acme/AcmeClient.cs"]
    assert (request_.out / "src/Acme/Core/ApiEnum.cs").read_bytes() == b"namespace Acme.Core;\r\n"
    assert not (request_.out / ".ferrule").exists()
    assert (result.package, result.runtime_files, result.files) == ("Acme", 2, 4)


def test_reports_a_csharpier_failure(request_: CSharpRequest) -> None:
    with pytest.raises(FerruleError, match="csharpier failed: boom"):
        generate_csharp(request_, fake_runner([], {}, fail_on="format"), renderer)


def test_needs_csharpier(request_: CSharpRequest) -> None:
    missing = CSharpRequest(
        spec=request_.spec,
        config=request_.config,
        out=request_.out,
        gen=request_.gen,
        csharpier=("no-such-csharpier-binary",),
    )
    with pytest.raises(FerruleError, match="FERRULE_CSHARPIER"):
        generate_csharp(missing, fake_runner([], {}), renderer)


def test_needs_the_csharp_renderer(request_: CSharpRequest) -> None:
    with pytest.raises(FerruleError, match="render_csharp_files is unavailable"):
        generate_csharp(request_, fake_runner([], {}), None)


def test_defaults_to_the_repository_wrapper() -> None:
    csharpier = resolve_formatter({}, "FERRULE_CSHARPIER", "csharpier", FORMATTER_BIN)
    assert csharpier[0].endswith("crates/csharp-plugin/tools/bin/csharpier")
    env = {"FERRULE_CSHARPIER": "dotnet csharpier"}
    assert resolve_formatter(env, "FERRULE_CSHARPIER", "csharpier", FORMATTER_BIN) == (
        "dotnet",
        "csharpier",
    )


def test_loads_the_runtime_renderer() -> None:
    render = load_csharp_renderer()
    assert render is not None
    files = render(OPTIONS)
    assert "src/Acme/Core/ApiEnum.cs" in files
    assert "\r\n" in files["src/Acme/Core/ApiEnum.cs"]


def test_accepts_the_csharp_target() -> None:
    args = build_parser().parse_args(
        ["generate", "--spec", "a.yml", "--config", "s.yml", "--out", "o", "--target", "csharp"]
    )
    assert args.target == "csharp"
