# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path

import pytest

from ferrule.errors import FerruleError
from ferrule.generate_php import (
    FORMATTER_BIN,
    PHP_RUNTIME_OPTIONS,
    PhpRequest,
    format_step,
    generate_php,
    load_php_renderer,
    plan_php_steps,
)
from ferrule.generate_ruby import resolve_formatter

OPTIONS = {
    "namespace": "Acme",
    "timeout": 60.0,
    "max_retries": 2,
    "initial_retry_delay": 0.5,
    "max_retry_delay": 8.0,
}


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]], fail_on: str = ""):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/Client.php", "<?php\n")
            write(out / ".php-cs-fixer.dist.php", "<?php\n")
            write(out / PHP_RUNTIME_OPTIONS, json.dumps(OPTIONS))
        code = 1 if fail_on and fail_on in argv else 0
        return subprocess.CompletedProcess(argv, code, "", "boom" if code else "")

    return run


def renderer(options: Mapping[str, object], /) -> Mapping[str, str]:
    assert options == OPTIONS
    return {"src/Core/Util.php": "<?php\n", "src/RequestOptions.php": "<?php\n"}


@pytest.fixture
def request_(tmp_path: Path) -> PhpRequest:
    return PhpRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        gen=write(tmp_path / "tools/ferrule-gen"),
        fixer=(sys.executable, "-m", "php_cs_fixer"),
    )


def test_plans_convert_then_generate_for_php(request_: PhpRequest, tmp_path: Path) -> None:
    convert, gen = plan_php_steps(request_, tmp_path / "work")
    assert convert.argv[1] == "convert"
    assert gen.argv[1:] == (
        "generate",
        str(tmp_path / "work/annotated.json"),
        "--target",
        "php",
        "--out",
        str(request_.out),
    )


def test_formats_in_the_package_directory(request_: PhpRequest) -> None:
    step = format_step(request_)
    assert step.argv[:4] == ("sh", "-c", 'cd "$0" && exec "$@"', str(request_.out))
    assert step.argv[-2:] == ("fix", "--using-cache=no")


def test_formats_then_renders_the_runtime(request_: PhpRequest) -> None:
    calls: list[tuple[str, ...]] = []
    result = generate_php(request_, fake_runner(calls), renderer)
    assert [c[1] if c[0] != "sh" else "php-cs-fixer" for c in calls] == [
        "convert",
        "generate",
        "php-cs-fixer",
    ]
    assert (request_.out / "src/Core/Util.php").is_file()
    assert not (request_.out / ".ferrule").exists()
    assert (result.package, result.runtime_files, result.files) == ("Acme", 2, 4)


def test_reports_a_formatter_failure(request_: PhpRequest) -> None:
    with pytest.raises(FerruleError, match="php-cs-fixer failed: boom"):
        generate_php(request_, fake_runner([], fail_on="fix"), renderer)


def test_needs_the_formatter(request_: PhpRequest) -> None:
    missing = PhpRequest(
        spec=request_.spec,
        config=request_.config,
        out=request_.out,
        gen=request_.gen,
        fixer=("no-such-php-cs-fixer",),
    )
    with pytest.raises(FerruleError, match="FERRULE_PHP_CS_FIXER"):
        generate_php(missing, fake_runner([]), renderer)


def test_defaults_to_the_repository_wrapper() -> None:
    fixer = resolve_formatter({}, "FERRULE_PHP_CS_FIXER", "php-cs-fixer", FORMATTER_BIN)
    assert fixer[0].endswith("crates/php-plugin/tools/bin/php-cs-fixer")


def test_needs_the_php_renderer(request_: PhpRequest) -> None:
    with pytest.raises(FerruleError, match="render_php_files is unavailable"):
        generate_php(request_, fake_runner([]), None)


def test_loads_the_runtime_renderer() -> None:
    render = load_php_renderer()
    assert render is not None
    files = render(OPTIONS)
    assert "src/Core/BaseClient.php" in files
    assert (
        "class AcmeException extends \\Exception" in files["src/Core/Exceptions/AcmeException.php"]
    )
