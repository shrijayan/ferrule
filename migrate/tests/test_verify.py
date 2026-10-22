# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
from pathlib import Path

import pytest

from ferrule.cli import main
from ferrule.errors import FerruleError
from ferrule.verify import closeness, detect_package, export_ref, normalize, verify

OLD = '''\
"""Acme."""
from acme._client import Client

__all__ = ["Client", "helper"]


def helper(x: int) -> int:
    return x
'''
OLD_CLIENT = """\
class Client:
    def get(self, a: int) -> int:
        return a

    def remove_me(self) -> None:
        pass

    def _private(self) -> None:
        pass
"""
NEW = '''\
"""Acme."""
from acme._client import Client

__all__ = ["Client", "helper", "added"]


def helper(x: int, y: int) -> int:
    return x + y


def added(*args: int) -> None:
    pass
'''
NEW_CLIENT = """\
class Client:
    def get(self, a: int, b: int = 1, **kw: int) -> int:
        return a

    def _private(self, z: int) -> None:
        pass

    class Nested:
        pass
"""


def make(root: Path, init: str, client: str, *, src: bool = True) -> Path:
    pkg = root / "src" / "acme" if src else root / "acme"
    pkg.mkdir(parents=True)
    (pkg / "__init__.py").write_text(init)
    (pkg / "_client.py").write_text(client)
    (root / "README.md").write_text("# Acme\n\nhello\n")
    return root


@pytest.fixture
def trees(tmp_path: Path) -> tuple[Path, Path]:
    return make(tmp_path / "old", OLD, OLD_CLIENT), make(
        tmp_path / "new", NEW, NEW_CLIENT, src=False
    )


def test_breaking_and_additive(trees: tuple[Path, Path]) -> None:
    report = verify(*trees)
    assert report.package == "acme"
    breaking = {(c.kind, c.path) for c in report.breaking}
    assert ("OBJECT_REMOVED", "acme._client.Client.remove_me") in breaking
    assert ("PARAMETER_ADDED_REQUIRED", "acme.helper") in breaking
    additive = {(c.kind, c.detail) for c in report.additive}
    assert additive == {
        ("OBJECT_ADDED", "acme.added was added"),
        ("PARAMETER_ADDED_OPTIONAL", "acme.Client.get(b) was added"),
        ("PARAMETER_ADDED_OPTIONAL", "acme.Client.get(kw) was added"),
        ("OBJECT_ADDED", "acme.Client.Nested was added"),
    }
    assert report.summary().startswith(f"public API: {len(report.breaking)} breaking · 4 additive")


def test_identical_trees(trees: tuple[Path, Path]) -> None:
    old, _ = trees
    report = verify(old, old)
    assert (report.breaking, report.additive, report.closeness) == ([], [], 100.0)


def test_closeness(tmp_path: Path) -> None:
    (tmp_path / "a").mkdir()
    (tmp_path / "b").mkdir()
    (tmp_path / "a/same.py").write_text("x = 1\n\ny = 2   \n")
    (tmp_path / "b/same.py").write_text("x = 1\ny = 2\n")
    (tmp_path / "a/gone.py").write_text("z = 3\nw = 4\n")
    (tmp_path / "b/empty.py").write_text("\n")
    (tmp_path / "b/image.png").write_bytes(b"\x89PNG")
    pct, scores = closeness(tmp_path / "a", tmp_path / "b")
    assert pct == 50.0
    assert [(s.path, s.ratio, s.weight) for s in scores] == [
        ("gone.py", 0.0, 2),
        ("same.py", 1.0, 2),
    ]
    assert closeness(tmp_path / "nope", tmp_path / "nope2") == (100.0, [])
    assert normalize("a  \n\n b\n") == ["a", " b"]


def test_detect_package_errors(tmp_path: Path) -> None:
    with pytest.raises(FerruleError, match="FER4010"):
        detect_package(tmp_path)
    with pytest.raises(FerruleError, match="FER4010"):
        verify(tmp_path, tmp_path, "acme")


def test_cli_verify_json(
    trees: tuple[Path, Path], tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    old, new = trees
    out = tmp_path / "report.json"
    assert main(["verify", "--against", str(old), "--new", str(new), "--json", str(out)]) == 1
    printed = capsys.readouterr().out
    assert "breaking · 4 additive" in printed
    assert "  breaking " in printed
    data = json.loads(out.read_text())
    assert data["additive_count"] == 4
    assert data["breaking_count"] == len(data["breaking"]) > 0
    assert main(["verify", "--against", str(old), "--new", str(old)]) == 0


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)  # noqa: S603, S607


def test_cli_verify_git_ref(trees: tuple[Path, Path], capsys: pytest.CaptureFixture[str]) -> None:
    old, _ = trees
    git(old, "init", "-q")
    git(old, "add", ".")
    git(old, "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-qm", "init")
    assert main(["verify", "--against", "HEAD", "--repo", str(old), "--new", str(old)]) == 0
    assert "0 breaking · 0 additive" in capsys.readouterr().out
    assert main(["verify", "--against", "no-such-ref", "--repo", str(old), "--new", str(old)]) == 1
    assert "FER4011" in capsys.readouterr().err


def test_export_ref(trees: tuple[Path, Path], tmp_path: Path) -> None:
    old, _ = trees
    git(old, "init", "-q")
    git(old, "add", ".")
    git(old, "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-qm", "init")
    dest = export_ref(old, "HEAD", tmp_path / "x")
    assert (dest / "src/acme/_client.py").read_text() == OLD_CLIENT
