# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import json
import subprocess
from pathlib import Path

import pytest

from ferrule.cli import main
from ferrule.errors import FerruleError
from ferrule.merge import KeepRules
from ferrule.merge.gitio import read_dir, read_ref, read_tree, regenerate_branch, write_tree

GEN1 = {"src/acme/_client.py": b"A = 1\n", "README.md": b"r\n"}


def git(repo: Path, *args: str) -> str:
    return subprocess.run(  # noqa: S603
        ["git", "-C", str(repo), *args],  # noqa: S607
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    path = tmp_path / "repo"
    path.mkdir()
    git(path, "init", "-q", "-b", "main")
    git(path, "config", "user.name", "T")
    git(path, "config", "user.email", "t@example.com")
    for rel, data in GEN1.items():
        (path / rel).parent.mkdir(parents=True, exist_ok=True)
        (path / rel).write_bytes(data)
    git(path, "add", "-A")
    git(path, "commit", "-qm", "init")
    return path


def test_regenerate_keeps_custom_code_and_fast_forwards(repo: Path) -> None:
    first = regenerate_branch(repo, GEN1, keep=KeepRules())
    assert first.commit is None
    (repo / "src/acme/lib").mkdir()
    (repo / "src/acme/lib/x.py").write_bytes(b"custom\n")
    git(repo, "add", "-A")
    git(repo, "commit", "-qm", "custom")
    gen2 = {**GEN1, "README.md": b"r2\n", "src/acme/lib/x.py": b"gen\n"}
    second = regenerate_branch(repo, gen2, keep=KeepRules())
    assert second.conflict_branch is None
    assert (repo / "README.md").read_bytes() == b"r2\n"
    assert (repo / "src/acme/lib/x.py").read_bytes() == b"custom\n"
    assert "Ferrule-Generated-From: " + second.generated in git(repo, "log", "-1", "--format=%B")
    assert read_ref(repo, "generated") == gen2
    assert git(repo, "status", "--porcelain") == ""


def test_regenerate_conflict_goes_to_its_own_branch(repo: Path) -> None:
    regenerate_branch(repo, GEN1, keep=KeepRules())
    (repo / "README.md").write_bytes(b"mine\n")
    git(repo, "commit", "-qam", "edit")
    head = git(repo, "rev-parse", "HEAD")
    regen = regenerate_branch(repo, {**GEN1, "README.md": b"theirs\n"}, keep=KeepRules())
    assert regen.conflict_branch is not None
    assert git(repo, "rev-parse", "HEAD") == head
    assert b"<<<<<<<" in read_ref(repo, regen.conflict_branch)["README.md"]


def test_regenerate_into_other_branch(repo: Path) -> None:
    git(repo, "branch", "next")
    regenerate_branch(repo, GEN1, keep=KeepRules(), into="next")
    regen = regenerate_branch(repo, {**GEN1, "B.md": b"b\n"}, keep=KeepRules(), into="next")
    assert read_ref(repo, "next")["B.md"] == b"b\n"
    assert regen.branch == "next"
    with pytest.raises(FerruleError):
        regenerate_branch(repo, GEN1, keep=KeepRules(), into="nope")
    git(repo, "checkout", "-q", "--detach")
    with pytest.raises(FerruleError):
        regenerate_branch(repo, GEN1, keep=KeepRules())


def test_readers_and_writer(repo: Path) -> None:
    assert read_tree("HEAD", repo) == GEN1
    assert read_tree(str(repo), repo) == GEN1
    (repo / "link").symlink_to("README.md")
    with pytest.raises(FerruleError):
        read_dir(repo)
    git(repo, "add", "link")
    git(repo, "commit", "-qm", "link")
    with pytest.raises(FerruleError):
        read_ref(repo, "HEAD")
    with pytest.raises(FerruleError):
        read_ref(repo, "missing-ref")
    with pytest.raises(FerruleError):
        write_tree(repo, {})


def test_cli_merge(repo: Path, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    dirs: dict[str, Path] = {}
    for name, readme in (("b", b"a\nb\nc\n"), ("o", b"A\nb\nc\n"), ("t", b"a\nb\nC\n")):
        dirs[name] = tmp_path / name
        write_tree(dirs[name], {"README.md": readme})
    out, report = tmp_path / "out", tmp_path / "merge.json"
    argv = ["merge", "--base", str(dirs["b"]), "--ours", str(dirs["o"])]
    code = main([*argv, "--theirs", str(dirs["t"]), "--out", str(out), "--report", str(report)])
    assert code == 0
    assert (out / "README.md").read_bytes() == b"A\nb\nC\n"
    assert json.loads(report.read_text())["clean"] is True
    write_tree(dirs["t"] / "x", {})
    (dirs["t"] / "README.md").write_bytes(b"Z\nb\nc\n")
    assert main([*argv, "--theirs", str(dirs["t"]), "--keep", "none"]) == 1
    assert "FER4301" in capsys.readouterr().out
    assert main([*argv, "--theirs", "nope", "--repo", str(repo)]) == 1


@pytest.mark.parametrize(
    "rel",
    [
        "../escape.py",
        "/abs/path.py",
        "a/../../b.py",
        "a/./b.py",
        "a//b.py",
        "a\\b.py",
        ".git/config",
        "a/.git/hooks/x",
        "a\0b",
        # Variants that macOS/Windows file systems fold onto .git (security review).
        ".GIT/config",
        "a/.Git/x",
        ".git./x",
        ".git /x",
        "‌.git/x",
        "GIT~1/config",
    ],
)
def test_write_tree_refuses_paths_outside_out(tmp_path: Path, rel: str) -> None:
    # A crafted ref or merge result must never write outside --out (security review).
    with pytest.raises(FerruleError, match="FER4008"):
        write_tree(tmp_path / "out", {rel: b"x"})
    assert not (tmp_path / "escape.py").exists()
