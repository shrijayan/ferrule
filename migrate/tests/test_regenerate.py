# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

import pytest

from ferrule import cli
from ferrule.adopt import AdoptRequest, adopt
from ferrule.errors import FerruleError
from ferrule.generate import Tools
from ferrule.regenerate import RegenerateRequest, regenerate

REV = "91f52a40c746e8a2ac0271d8be0b7c82cdf28798"
ENV = {"FERRULE_FORGE_COMMIT": "f" * 40, "PATH": "/usr/bin:/bin", "HOME": "/nonexistent"}
CLIENT = "class Acme:\n    def ping(self) -> None: ...\n"


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def sh(repo: Path, *args: str) -> str:
    return subprocess.run(  # noqa: S603
        ["git", "-C", str(repo), "-c", "core.hooksPath=/dev/null", *args],  # noqa: S607
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def runner(foo: str):  # noqa: ANN201
    """``ferrule-gen`` and ruff, faked: the generator writes the client and ``foo.py``."""

    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        if tuple(argv[1:]) == ("--version",):
            return subprocess.CompletedProcess(argv, 0, f"ferrule-gen 0.0.0 forge {REV}\n", "")
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/acme/__init__.py", "from acme._client import Acme\n")
            write(out / "src/acme/_client.py", CLIENT)
            write(out / "src/acme/resources/foo.py", foo)
            write(out / "LICENSE", "Apache\n")
        return subprocess.CompletedProcess(argv, 0, "", "")

    return run


def render(package: str, package_dir: Path, /, **_: object) -> None:
    write(package_dir / "_utils/__init__.py", f"# {package} runtime\n")


@pytest.fixture
def adopted(tmp_path: Path) -> tuple[Path, Tools]:
    """An SDK another generator wrote, adopted with --spec, then given custom code on main."""
    repo = tmp_path / "sdk"
    sh(tmp_path, "init", "-q", "-b", "main", str(repo))
    sh(repo, "config", "user.name", "Dev")
    sh(repo, "config", "user.email", "dev@example.com")
    write(repo / "stainless.yml", "targets:\n  python:\n    keep_files:\n      - LICENSE\n")
    write(repo / "LICENSE", "MIT\n")
    write(repo / "src/acme/__init__.py", "old\n")
    write(repo / "src/acme/_rpc.py", "custom\n")
    sh(repo, "add", "-A")
    sh(repo, "commit", "-qm", "init")
    spec = write(tmp_path / "openapi.json", "{}\n")
    tools = Tools(gen=write(tmp_path / "tools/ferrule-gen"), ruff=sys.executable)
    request = AdoptRequest(repo, tools, config=repo / "stainless.yml", spec=spec)
    adopt(request, runner=runner("v1\n"), renderer=render, env=ENV)
    sh(repo, "checkout", "-q", "ferrule/adopt")
    # Custom code after adoption: a method added to the generated client.
    write(repo / "src/acme/_client.py", CLIENT + "\n    def events(self) -> None: ...\n")
    sh(repo, "commit", "-qam", "Add Acme.events")
    return repo, tools


def test_regenerate_keeps_custom_code_and_kept_files(adopted: tuple[Path, Tools]) -> None:
    repo, tools = adopted
    request = RegenerateRequest(repo, repo / "openapi.json", repo / "stainless.yml", tools)
    result = regenerate(request, runner=runner("v2\n"), renderer=render, env=ENV)
    assert result.conflict_branch is None
    assert (repo / "src/acme/resources/foo.py").read_text() == "v2\n"
    assert "def events" in (repo / "src/acme/_client.py").read_text()
    assert (repo / "LICENSE").read_text() == "MIT\n"
    assert (repo / "src/acme/_rpc.py").read_text() == "custom\n"
    # Ferrule owns this runtime (the lock lists it), so it is rendered, not copied back.
    assert (repo / "src/acme/_utils/__init__.py").read_text() == "# acme runtime\n"
    lock = json.loads((repo / "ferrule.lock").read_text())
    assert "src/acme/resources/foo.py" in lock["files"]
    assert f"Ferrule-Generated-From: {sh(repo, 'rev-parse', 'generated')}" in sh(
        repo, "log", "-1", "--format=%B"
    )
    assert sh(repo, "status", "--porcelain") == ""
    # Nothing changed since: a second run commits nothing.
    again = regenerate(request, runner=runner("v2\n"), renderer=render, env=ENV)
    assert again.commit is None


def test_regenerate_conflict_and_cli(
    adopted: tuple[Path, Tools], monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    repo, tools = adopted
    write(repo / "src/acme/resources/foo.py", "mine\n")
    sh(repo, "commit", "-qam", "Edit foo")

    def resolve(*_a: object, **_k: object) -> Tools:
        return tools

    monkeypatch.setattr("ferrule.cli.resolve_tools", resolve)
    monkeypatch.setattr("ferrule.cli.load_renderer", lambda: render)
    monkeypatch.setattr("ferrule.regenerate.run_subprocess", runner("theirs\n"))
    for key, value in ENV.items():
        monkeypatch.setenv(key, value)
    argv = ["regenerate", "--repo", str(repo), "--spec", str(repo / "openapi.json")]
    code = cli.main([*argv, "--config", str(repo / "stainless.yml")])
    assert code == 1
    out = capsys.readouterr().out
    assert "conflict" in out
    assert "ferrule/conflict-" in out
    assert (repo / "src/acme/resources/foo.py").read_text() == "mine\n"


def test_regenerate_needs_the_spec_and_config(adopted: tuple[Path, Tools]) -> None:
    repo, tools = adopted
    request = RegenerateRequest(repo, repo / "nope.json", repo / "stainless.yml", tools)
    with pytest.raises(FerruleError, match="FER4001"):
        regenerate(request, runner=runner("v2\n"), renderer=render, env=ENV)
