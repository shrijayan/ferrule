# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
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
    GenVersion,
    Tools,
    ensure_inside,
    format_step,
    gen_version,
    generate,
    is_generate_step,
    is_runtime_path,
    kept_copies,
    load_renderer,
    parse_gen_version,
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
REV = "91f52a40c746e8a2ac0271d8be0b7c82cdf28798"


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def fake_runner(calls: list[tuple[str, ...]]):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        calls.append(tuple(argv))
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/acme/__init__.py", "from acme._client import Acme\n")
            write(out / "src/acme/_client.py", "class Acme: ...\n")
            write(out / "src/acme/_types.py", "# generated wins\n")
        return subprocess.CompletedProcess(argv, 0, "", "")

    return run


@pytest.fixture
def inputs(tmp_path: Path) -> tuple[GenerateRequest, Path]:
    tools = Tools(gen=write(tmp_path / "tools/ferrule-gen"), ruff=sys.executable)
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
    env = {"FERRULE_GEN": "/env/ferrule-gen", "FERRULE_RUFF": "myruff"}
    tools = resolve_tools(env, root=tmp_path)
    assert tools == Tools(gen=Path("/env/ferrule-gen"), ruff="myruff")
    assert resolve_tools(env, gen=Path("/arg/gen")).gen == Path("/arg/gen")
    default = resolve_tools({"FERRULE_GEN": ""}, root=tmp_path)
    assert default == Tools(gen=tmp_path / "target/release/ferrule-gen", ruff="ruff")


def test_plan_steps(inputs: tuple[GenerateRequest, Path]) -> None:
    request, tmp = inputs
    convert, gen = plan_steps(request, tmp / "work")
    annotated = str(tmp / "work/annotated.json")
    assert convert.argv == (
        str(request.tools.gen),
        "convert",
        str(request.spec),
        str(request.config),
        "-o",
        annotated,
        # The adopted SDK's runtime decides older generator styles (ADR 0018).
        "--runtime-from",
        str(request.runtime_from),
    )
    fresh, _ = plan_steps(replace(request, runtime_from=None), tmp / "work")
    assert fresh.argv == convert.argv[:-2]
    assert gen.name == "generate"
    assert gen.argv == (str(request.tools.gen), "generate", annotated, "--out", str(request.out))
    assert is_generate_step(gen.argv, request.tools)
    assert not is_generate_step(convert.argv, request.tools)
    assert format_step(request.out, request.tools).argv[1:3] == ("format", "--no-cache")


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        (f"ferrule-gen 0.1.0 forge {REV}\n", GenVersion("0.1.0", REV)),
        (f"ferrule-gen 1.2.3-rc.1 forge {REV}", GenVersion("1.2.3-rc.1", REV)),
        ("ferrule-gen 0.0.0\n", None),
        (f"ferrule-gen 0.0.0 forge {REV[:12]}", None),
        (f"ferrule-gen 0.0.0 forge {REV.upper()}", None),
        (f"forge 0.0.0 forge {REV}", None),
        (f"ferrule-gen 0.0.0 forge {REV} extra", None),
        ("", None),
    ],
)
def test_parse_gen_version(text: str, expected: GenVersion | None) -> None:
    assert parse_gen_version(text) == expected


def test_gen_version(inputs: tuple[GenerateRequest, Path]) -> None:
    request, _ = inputs
    tools = request.tools

    def printing(stdout: str, code: int = 0):  # noqa: ANN202
        def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
            assert tuple(argv) == (str(tools.gen), "--version")
            return subprocess.CompletedProcess(argv, code, stdout, "boom" if code else "")

        return run

    version = gen_version(tools, printing(f"ferrule-gen 0.0.0 forge {REV}\n"))
    assert version.generator == "ferrule-gen 0.0.0"
    assert version.forge == REV
    old = r"FER4002 at FERRULE_GEN: .*printed 'ferrule-gen 0\.0\.0'"
    with pytest.raises(FerruleError, match=old):
        gen_version(tools, printing("ferrule-gen 0.0.0\n"))
    with pytest.raises(
        FerruleError, match=r"printed 'nothing'.*cargo build --release -p ferrule-gen"
    ):
        gen_version(tools, printing(""))
    with pytest.raises(FerruleError, match="FER4003 at version: version failed: boom"):
        gen_version(tools, printing("", 1))
    with pytest.raises(FerruleError, match="FER4002 at FERRULE_GEN"):
        gen_version(replace(tools, gen=Path("/nope")), printing(""))


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


def test_kept_copies_keep_every_file_the_generator_did_not_write() -> None:
    # Stainless leaves modules it no longer generates in place (OneBusAway 1.30.0 still ships
    # types/search_for_route_retrieve_params.py), and lib/ is hand-written: both survive.
    have = [
        P("_qs.py"),
        P("_client.py"),
        P("lib/helpers.py"),
        P("types/search_for_route_retrieve_params.py"),
        P("types/__init__.py"),
    ]
    generated = [P("_client.py"), P("types/__init__.py")]
    assert kept_copies(have, generated) == [
        P("_qs.py"),
        P("lib/helpers.py"),
        P("types/search_for_route_retrieve_params.py"),
    ]


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
    assert len(calls) == 4
    assert calls[0][:2] == (str(request.tools.gen), "convert")
    assert calls[1][:2] == (str(request.tools.gen), "generate")
    # Imports are sorted (as Stainless's own `ruff check --fix` does) before formatting.
    assert calls[2][1:5] == ("check", "--select", "I", "--fix")
    assert calls[3][1] == "format"
    files = relative_files(request.out)
    assert files == [
        P("src/acme/__init__.py"),
        P("src/acme/_client.py"),
        P("src/acme/_qs.py"),
        P("src/acme/_types.py"),
        P("src/acme/_utils/_utils.py"),
        # No longer generated, so kept from the existing SDK (as Stainless does).
        P("src/acme/resources/foo.py"),
    ]
    assert (request.out / "src/acme/_types.py").read_text() == "# generated wins\n"
    assert result == GenerateResult("acme", 6, 3, tree_digest(request.out))


def test_generate_marks_scripts_executable(inputs: tuple[GenerateRequest, Path]) -> None:
    # Stainless SDKs ship scripts/* as 100755 (onebusaway-python); written fresh they were 644.
    request, _ = inputs
    calls: list[tuple[str, ...]] = []
    base = fake_runner(calls)

    def runner(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        done = base(argv)
        if argv[1] == "generate":
            write(Path(argv[argv.index("--out") + 1]) / "scripts/test", "#!/usr/bin/env bash\n")
        return done

    generate(request, runner=runner)
    assert (request.out / "scripts/test").stat().st_mode & 0o111 == 0o111
    assert (request.out / "src/acme/_client.py").stat().st_mode & 0o111 == 0


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

    def render(package: str, package_dir: Path, /, **_: object) -> None:
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
    for bad, fix in (
        (replace(tools, gen=Path("/nope")), "cargo build --release -p ferrule-gen"),
        (replace(tools, ruff="definitely-not-a-ruff-binary"), "set FERRULE_RUFF"),
    ):
        with pytest.raises(FerruleError, match=f"FER4002.*{fix}"):
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

    def render(package: str, package_dir: Path, /, **_: object) -> None:
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
    seen: list[Tools] = []

    def fake(request: GenerateRequest, **_: object) -> GenerateResult:
        seen.append(request.tools)
        return GenerateResult("acme", 5, 2, "abc")

    monkeypatch.setattr(cli, "generate", fake)
    monkeypatch.setenv("FERRULE_GEN", "/env/ferrule-gen")
    argv = ["generate", "--spec", str(request.spec), "--config", str(request.config), "--out", "o"]
    assert cli.main(argv) == 0
    assert "generated acme: 5 files (2 runtime) sha256 abc" in capsys.readouterr().out
    assert cli.main([*argv, "--gen", "/arg/ferrule-gen"]) == 0
    assert [t.gen for t in seen] == [Path("/env/ferrule-gen"), Path("/arg/ferrule-gen")]


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


def foreign_sdk(root: Path) -> Path:
    """An SDK whose runtime another generator wrote: no ``_utils/`` (opencode-python-sdk)."""
    sdk = root / "foreign"
    write(sdk / "src/acme/__init__.py", "old\n")
    write(sdk / "src/acme/_base_client.py", "class RequestOptions: ...\n")
    write(sdk / "src/acme/_types.py", "NOT_GIVEN = object()\n")
    write(sdk / "src/acme/_rpc.py", "custom rpc\n")
    write(sdk / "src/acme/types/_generated.py", "old types\n")
    return sdk


def test_generate_renders_ferrule_runtime_for_a_foreign_runtime(
    inputs: tuple[GenerateRequest, Path],
) -> None:
    # The runtime of a non-Stainless SDK can't run Stainless-style resources: Ferrule's own
    # runtime is rendered, the generator's style is today's, and the SDK's other modules stay.
    request, tmp = inputs
    request = replace(request, runtime_from=foreign_sdk(tmp))
    calls: list[tuple[str, ...]] = []
    rendered: list[str] = []

    def render(package: str, package_dir: Path, /, **_: object) -> None:
        rendered.append(package)
        write(package_dir / "_base_client.py", "ferrule runtime\n")
        write(package_dir / "_utils/__init__.py", "ferrule utils\n")

    result = generate(request, runner=fake_runner(calls), renderer=render)
    assert "--runtime-from" not in calls[0]
    assert rendered == ["acme"]
    out = request.out / "src/acme"
    assert (out / "_base_client.py").read_text() == "ferrule runtime\n"
    assert (out / "_types.py").read_text() == "# generated wins\n"
    assert (out / "_rpc.py").read_text() == "custom rpc\n"
    assert (out / "types/_generated.py").read_text() == "old types\n"
    assert result.runtime_files == 2


def test_generate_foreign_runtime_needs_the_renderer(inputs: tuple[GenerateRequest, Path]) -> None:
    request, tmp = inputs
    request = replace(request, runtime_from=foreign_sdk(tmp))
    with pytest.raises(FerruleError, match="FER4004"):
        generate(request, runner=fake_runner([]))


def test_generate_renders_the_configured_pagination(inputs: tuple[GenerateRequest, Path]) -> None:
    # A rendered runtime (no SDK to copy it from) gets pagination.py from the page schemes the
    # converter wrote on the annotated spec.
    request, tmp = inputs
    request = replace(request, runtime_from=foreign_sdk(tmp))
    entry: dict[str, object] = {
        "name": "cursor_page",
        "kind": "cursor",
        "request": [],
        "response": [],
    }
    base = fake_runner([])

    def runner(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        if argv[1] == "convert":
            write(Path(argv[argv.index("-o") + 1]), json.dumps({"x-ferrule-page-schemes": [entry]}))
        return base(argv)

    seen: list[object] = []

    def render(_package: str, package_dir: Path, /, **options: object) -> None:
        seen.append(options.get("pagination"))
        write(package_dir / "_utils/__init__.py")

    generate(request, runner=runner, renderer=render)
    assert seen == [[entry]]
