# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import shutil
import subprocess
import sys
from collections.abc import Sequence
from dataclasses import replace
from pathlib import Path, PurePosixPath

import pytest

from ferrule import cli
from ferrule.errors import FerruleError
from ferrule.generate import (
    GenerateRequest,
    GenerateResult,
    Tools,
    ensure_inside,
    format_step,
    generate,
    is_runtime_path,
    load_renderer,
    pick_package,
    plan_steps,
    relative_files,
    resolve_tools,
    run_subprocess,
    runtime_copies,
    symlinks,
    tree_digest,
)

P = PurePosixPath


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]]):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if "--target" in argv:
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/acme/__init__.py", "from acme._client import Acme\n")
            write(out / "src/acme/_client.py", "class Acme: ...\n")
            write(out / "src/acme/_types.py", "# generated wins\n")
        return subprocess.CompletedProcess(argv, 0, "", "")

    return run


@pytest.fixture
def inputs(tmp_path: Path) -> tuple[GenerateRequest, Path]:
    tools = Tools(
        node=sys.executable,
        converter=write(tmp_path / "tools/cli.ts"),
        forge=write(tmp_path / "tools/forge"),
        plugin=write(tmp_path / "tools/plugin.wasm"),
        ruff=sys.executable,
    )
    sdk = tmp_path / "sdk"
    write(sdk / "src/acme/__init__.py", "old\n")
    write(sdk / "src/acme/_client.py", "old\n")
    write(sdk / "src/acme/_types.py", "runtime\n")
    write(sdk / "src/acme/_qs.py", "runtime qs\n")
    write(sdk / "src/acme/_utils/_utils.py", "utils\n")
    write(sdk / "src/acme/_utils/__pycache__/x.pyc", "junk")
    write(sdk / "src/acme/resources/foo.py", "old\n")
    request = GenerateRequest(
        spec=write(tmp_path / "openapi.yml", "openapi: 3.1.0\n"),
        config=write(tmp_path / "stainless.yml", "{}\n"),
        out=tmp_path / "out",
        tools=tools,
        runtime_from=sdk,
    )
    return request, tmp_path


def test_resolve_tools_precedence(tmp_path: Path) -> None:
    env = {"FERRULE_FORGE": "/env/forge", "FERRULE_RUFF": "myruff"}
    tools = resolve_tools(env, root=tmp_path, plugin=Path("/arg/plugin.wasm"))
    assert tools.forge == Path("/env/forge")
    assert tools.plugin == Path("/arg/plugin.wasm")
    assert tools.ruff == "myruff"
    assert tools.node == "node"
    assert tools.converter == tmp_path / "converter/src/cli.ts"
    assert resolve_tools({}, root=tmp_path).forge == tmp_path / ".cache/forge/target/release/forge"


def test_plan_steps(inputs: tuple[GenerateRequest, Path]) -> None:
    request, tmp = inputs
    convert, forge = plan_steps(request, tmp / "work")
    assert convert.argv[-2:] == ("-o", str(tmp / "work/annotated.json"))
    assert forge.argv[0] == str(request.tools.forge)
    assert f"wasm://{request.tools.plugin}" in forge.argv
    assert forge.argv[-1] == str(request.out)
    assert format_step(request.out, request.tools).argv[1:3] == ("format", "--no-cache")


@pytest.mark.parametrize(
    ("rel", "expected"),
    [
        ("_base_client.py", True),
        ("_utils/_transform.py", True),
        ("_client.py", False),
        ("resources/foo.py", False),
        ("_utils/__pycache__/x.pyc", False),
        (".", False),
    ],
)
def test_is_runtime_path(rel: str, *, expected: bool) -> None:
    assert is_runtime_path(P(rel)) is expected


def test_runtime_copies_generated_wins() -> None:
    have = [P("_types.py"), P("_qs.py"), P("_client.py")]
    assert runtime_copies(have, [P("_types.py")]) == [P("_qs.py")]


def test_pick_package() -> None:
    assert pick_package(["acme", "not-a-package"], "x") == "acme"
    with pytest.raises(FerruleError, match="FER4005"):
        pick_package([], "x")
    with pytest.raises(FerruleError, match="found a, b"):
        pick_package(["b", "a"], "x")


def test_generate_copies_runtime_and_formats(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    calls: list[tuple[str, ...]] = []
    result = generate(request, runner=fake_runner(calls))
    assert [c[1] for c in calls[::2]] == [str(request.tools.converter), "format"]
    assert len(calls) == 3
    files = relative_files(request.out)
    assert files == [
        P("src/acme/__init__.py"),
        P("src/acme/_client.py"),
        P("src/acme/_qs.py"),
        P("src/acme/_types.py"),
        P("src/acme/_utils/_utils.py"),
    ]
    assert (request.out / "src/acme/_types.py").read_text() == "# generated wins\n"
    assert result == GenerateResult("acme", 5, 2, tree_digest(request.out))


def test_generate_is_deterministic(inputs: tuple[GenerateRequest, Path]) -> None:
    request, tmp = inputs
    first = generate(request, runner=fake_runner([]))
    second_request = replace(request, out=tmp / "out2")
    second = generate(second_request, runner=fake_runner([]))
    assert first.digest == second.digest
    write(tmp / "out2/src/acme/extra.py")
    assert tree_digest(tmp / "out2") != first.digest


def test_generate_with_renderer(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    request = replace(request, runtime_from=None)
    seen: list[tuple[str, Path]] = []

    def render(package: str, package_dir: Path, /) -> None:
        seen.append((package, package_dir))
        write(package_dir / "_base_client.py", "x\n")

    result = generate(request, runner=fake_runner([]), renderer=render)
    assert seen == [("acme", request.out / "src/acme")]
    assert result.runtime_files == 1


def test_generate_without_runtime(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    request = replace(request, runtime_from=None)
    with pytest.raises(FerruleError, match="FER4004"):
        generate(request, runner=fake_runner([]))


def test_generate_step_failure(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs

    def broken(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(argv, 3, "", "line\nFER101 bad resource\n")

    with pytest.raises(
        FerruleError, match=r"FER4003 at convert: convert failed: FER101 bad resource"
    ):
        generate(request, runner=broken)

    def silent(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(argv, 4, "", "")

    request = replace(request, out=request.out.parent / "o3")
    with pytest.raises(FerruleError, match="exit code 4"):
        generate(request, runner=silent)


@pytest.mark.parametrize(
    ("field", "value", "code"),
    [
        ("spec", Path("/nope.yml"), "FER4001"),
        ("runtime_from", Path("/nope"), "FER4001"),
    ],
)
def test_generate_input_errors(
    inputs: tuple[GenerateRequest, Path],
    field: str,
    value: Path,
    code: str,
) -> None:
    request, _ = inputs
    with pytest.raises(FerruleError, match=code):
        generate(replace(request, **{field: value}), runner=fake_runner([]))


def test_generate_tool_errors(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    tools = request.tools
    for bad in (
        replace(tools, forge=Path("/nope")),
        replace(tools, ruff="definitely-not-a-ruff-binary"),
    ):
        with pytest.raises(FerruleError, match="FER4002"):
            generate(replace(request, tools=bad), runner=fake_runner([]))


def test_generate_out_not_empty(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    write(request.out / "x")
    with pytest.raises(FerruleError, match="FER4006"):
        generate(request, runner=fake_runner([]))


def test_relative_files_missing_dir(tmp_path: Path) -> None:
    assert relative_files(tmp_path / "nope") == []


def test_load_renderer(monkeypatch: pytest.MonkeyPatch) -> None:
    import ferrule_runtime  # noqa: PLC0415

    def render(package: str, package_dir: Path, /) -> None:
        del package, package_dir

    monkeypatch.setattr(ferrule_runtime, "render", render, raising=False)
    assert load_renderer() is not None
    monkeypatch.delattr(ferrule_runtime, "render")
    assert load_renderer() is None
    monkeypatch.setitem(sys.modules, "ferrule_runtime", None)
    assert load_renderer() is None


def test_run_subprocess() -> None:
    proc = run_subprocess([sys.executable, "-c", "print('hi')"])
    assert proc.returncode == 0
    assert proc.stdout == "hi\n"


def test_cli_generate(
    inputs: tuple[GenerateRequest, Path],
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    request, _ = inputs

    def fake(request: GenerateRequest, **_: object) -> GenerateResult:
        del request
        return GenerateResult("acme", 5, 2, "abc")

    monkeypatch.setattr(cli, "generate", fake)
    argv = ["generate", "--spec", str(request.spec), "--config", str(request.config), "--out", "o"]
    assert cli.main(argv) == 0
    assert "generated acme: 5 files (2 runtime) sha256 abc" in capsys.readouterr().out


def test_cli_generate_error(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    argv = ["generate", "--spec", "nope.yml", "--config", "c.yml", "--out", str(tmp_path / "o")]
    assert cli.main(argv) == 1
    assert "FER4001 at --spec" in capsys.readouterr().err


@pytest.mark.parametrize("kind", ["file", "dir"])
def test_runtime_from_refuses_symlinks(inputs: tuple[GenerateRequest, Path], kind: str) -> None:
    request, tmp = inputs
    secret = write(tmp / "outside/secret/id_rsa", "PRIVATE\n")
    pkg = tmp / "sdk/src/acme"
    if kind == "file":
        (pkg / "_qs.py").unlink()
        (pkg / "_qs.py").symlink_to(secret)
    else:
        for f in (pkg / "_utils").rglob("*"):
            if f.is_file():
                f.unlink()
        shutil.rmtree(pkg / "_utils")
        (pkg / "_utils").symlink_to(secret.parent, target_is_directory=True)
    with pytest.raises(FerruleError, match="FER4007"):
        generate(request, runner=fake_runner([]))
    copied = [p for p in request.out.rglob("*") if p.is_file()]
    assert all("PRIVATE" not in p.read_text() for p in copied)


def test_symlink_helpers(tmp_path: Path) -> None:
    root = tmp_path / "root"
    write(root / "a/b.py", "x\n")
    (root / "link.py").symlink_to(root / "a/b.py")
    (tmp_path / "rootlink").symlink_to(root, target_is_directory=True)
    assert symlinks(root) == [P("link.py")]
    assert symlinks(tmp_path / "rootlink") == [P(".")]
    assert relative_files(root) == [P("a/b.py")]
    assert ensure_inside(root, P("a/b.py")) == root / "a/b.py"
    with pytest.raises(FerruleError, match="FER4007"):
        ensure_inside(root, P("link.py"))
