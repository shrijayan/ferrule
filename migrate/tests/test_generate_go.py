# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path

import pytest

from ferrule.errors import FerruleError
from ferrule.generate_go import (
    GO_RUNTIME_OPTIONS,
    GoRequest,
    generate_go,
    gofmt_step,
    load_go_renderer,
    plan_go_steps,
)

OPTIONS = {
    "module_path": "github.com/acme/acme-go",
    "package_name": "acme",
    "client_name": "Acme",
    "org_name": "acme",
    "max_retries": 2,
    "config_fields": "",
    "config_clone": "",
    "client_options": "",
}


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]], gofmt_out: str = ""):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "go.mod", "module github.com/acme/acme-go\n\ngo 1.22\n")
            write(out / "client.go", "package acme\n")
            write(out / GO_RUNTIME_OPTIONS, json.dumps(OPTIONS))
        stdout = gofmt_out if argv[1] == "-l" else ""
        return subprocess.CompletedProcess(argv, 0, stdout, "")

    return run


def renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
    assert options == OPTIONS
    return {"field.go": "package acme\n", "internal/apijson/port.go": "package apijson\n"}


@pytest.fixture
def request_(tmp_path: Path) -> GoRequest:
    return GoRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        gen=write(tmp_path / "tools/ferrule-gen"),
        gofmt=sys.executable,
    )


def test_plans_convert_then_generate_for_go(request_: GoRequest, tmp_path: Path) -> None:
    convert, gen = plan_go_steps(request_, tmp_path / "work")
    assert convert.argv[1] == "convert"
    assert gen.argv[1:] == (
        "generate",
        str(tmp_path / "work/annotated.json"),
        "--target",
        "go",
        "--out",
        str(request_.out),
    )
    assert gofmt_step(request_).argv == (sys.executable, "-l", str(request_.out))


def test_renders_the_runtime_from_the_plugin_options(request_: GoRequest) -> None:
    calls: list[tuple[str, ...]] = []
    result = generate_go(request_, fake_runner(calls), renderer)
    assert [c[1] for c in calls] == ["convert", "generate", "-l"]
    assert result.package == "github.com/acme/acme-go"
    assert result.runtime_files == 2
    assert (request_.out / "internal/apijson/port.go").read_text() == "package apijson\n"
    assert not (request_.out / ".ferrule").exists()
    assert result.files == 4


def test_fails_when_gofmt_would_change_a_file(request_: GoRequest) -> None:
    with pytest.raises(FerruleError, match="gofmt would change"):
        generate_go(request_, fake_runner([], gofmt_out="client.go\n"), renderer)


def test_needs_the_go_renderer(request_: GoRequest) -> None:
    with pytest.raises(FerruleError, match="render_go_files is unavailable"):
        generate_go(request_, fake_runner([]), None)


def test_loads_the_runtime_renderer() -> None:
    render = load_go_renderer()
    assert render is not None
    files = render(OPTIONS)
    assert "option/requestoption.go" in files
    assert "github.com/acme/acme-go/internal/requestconfig" in files["option/requestoption.go"]
