# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath

import pytest

from ferrule.errors import FerruleError
from ferrule.generate_ruby import (
    RUBY_RUNTIME_OPTIONS,
    RubyRequest,
    format_steps,
    generate_ruby,
    hide_aliases,
    load_ruby_renderer,
    plan_ruby_steps,
    restore_aliases,
)

OPTIONS = {
    "gem_name": "acme",
    "module_name": "Acme",
    "array_format": "comma",
    "auth_in": "headers",
    "base_model_example": "",
    "enum_example": "",
    "union_example": "",
}

MODELS_RBS = "module Acme\n  class WidgetParams = Acme::Models::WidgetParams\nend\n"


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]], seen: dict[str, str], fail_on: str = ""):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "lib/acme.rb", "# frozen_string_literal: true\n")
            write(out / "rbi/acme/models.rbi", "# typed: strong\n")
            write(out / "sig/acme/models.rbs", MODELS_RBS)
            write(out / ".rubocop.yml", "AllCops: {}\n")
            write(out / RUBY_RUNTIME_OPTIONS, json.dumps(OPTIONS))
        if "write" in argv and "--plugin=rbs" in argv:
            # What stree is handed: the class alias hidden from it.
            seen["rbs"] = Path(argv[-1]).read_text()
        code = 1 if fail_on and fail_on in argv else 0
        return subprocess.CompletedProcess(argv, code, "", "boom" if code else "")

    return run


def renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
    assert options == OPTIONS
    return {"lib/acme/errors.rb": "# frozen_string_literal: true\n", "rbi/acme/errors.rbi": ""}


@pytest.fixture
def request_(tmp_path: Path) -> RubyRequest:
    return RubyRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        gen=write(tmp_path / "tools/ferrule-gen"),
        stree=(sys.executable, "-m", "stree"),
        rubocop=(sys.executable, "-m", "rubocop"),
    )


def test_plans_convert_then_generate_for_ruby(request_: RubyRequest, tmp_path: Path) -> None:
    convert, gen = plan_ruby_steps(request_, tmp_path / "work")
    assert convert.argv[1] == "convert"
    assert gen.argv[1:] == (
        "generate",
        str(tmp_path / "work/annotated.json"),
        "--target",
        "ruby",
        "--out",
        str(request_.out),
    )


def test_formats_each_view_as_rake_format_does(request_: RubyRequest) -> None:
    files = [PurePosixPath(p) for p in ("lib/acme.rb", "rbi/acme/a.rbi", "sig/acme/a.rbs", "x.md")]
    rubocop, rbi, rbs = format_steps(request_, files)
    out = request_.out
    assert rubocop.argv[:2] == ("env", "RUBYOPT=--encoding=UTF-8")
    assert "--autocorrect" in rubocop.argv
    assert rubocop.argv[-1] == str(out / "lib/acme.rb")
    assert rubocop.argv[rubocop.argv.index("--config") + 1] == str(out / ".rubocop.yml")
    assert rbi.argv[-3:] == ("write", "--", str(out / "rbi/acme/a.rbi"))
    assert rbs.argv[-4:] == ("write", "--plugin=rbs", "--", str(out / "sig/acme/a.rbs"))
    assert format_steps(request_, [PurePosixPath("README.md")]) == []


def test_hides_class_aliases_from_syntax_tree() -> None:
    hidden = hide_aliases(MODELS_RBS)
    assert "class WidgetParams =" not in hidden
    assert "  WidgetParams: Acme::Models::WidgetParams" in hidden
    assert restore_aliases(hidden) == MODELS_RBS
    assert restore_aliases("module A\nend\n") == "module A\nend\n"


def test_formats_then_renders_the_runtime(request_: RubyRequest) -> None:
    calls: list[tuple[str, ...]] = []
    seen: dict[str, str] = {}
    result = generate_ruby(request_, fake_runner(calls, seen), renderer)
    names = [c[1] if c[0] != "env" else c[4] for c in calls]
    assert names == ["convert", "generate", "rubocop", "stree", "stree"]
    assert "ferrule-rbs-alias" in seen["rbs"]
    assert (request_.out / "sig/acme/models.rbs").read_text() == MODELS_RBS
    assert (request_.out / "lib/acme/errors.rb").is_file()
    assert not (request_.out / ".ferrule").exists()
    assert (result.package, result.runtime_files, result.files) == ("acme", 2, 6)
    # The runtime is rendered after formatting: no formatter is handed a runtime file.
    assert not any("errors" in arg for call in calls for arg in call)


def test_reports_a_formatter_failure_and_restores_the_aliases(request_: RubyRequest) -> None:
    with pytest.raises(FerruleError, match="stree failed: boom"):
        generate_ruby(request_, fake_runner([], {}, fail_on="--plugin=rbs"), renderer)
    assert (request_.out / "sig/acme/models.rbs").read_text() == MODELS_RBS


def test_needs_the_formatters(request_: RubyRequest) -> None:
    missing = RubyRequest(
        spec=request_.spec,
        config=request_.config,
        out=request_.out,
        gen=request_.gen,
        stree=("no-such-stree-binary",),
    )
    with pytest.raises(FerruleError, match="FERRULE_STREE"):
        generate_ruby(missing, fake_runner([], {}), renderer)


def test_needs_the_ruby_renderer(request_: RubyRequest) -> None:
    with pytest.raises(FerruleError, match="render_ruby_files is unavailable"):
        generate_ruby(request_, fake_runner([], {}), None)


def test_loads_the_runtime_renderer() -> None:
    render = load_ruby_renderer()
    assert render is not None
    files = render(OPTIONS)
    assert "lib/acme/internal/util.rb" in files
    assert "module Acme" in files["lib/acme/errors.rb"]
