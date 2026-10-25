# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Run the action's shell scripts against a stub ``gh`` and a throwaway git repo."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parent.parent
GIT = shutil.which("git") or "git"
BASH = shutil.which("bash") or "/bin/bash"

STUB_GH = """#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$GH_LOG"
if [[ "$1 $2" == "pr list" ]]; then
  expr=""
  while [[ $# -gt 0 ]]; do
    if [[ "$1" == "--jq" ]]; then expr="$2"; fi
    shift
  done
  jq -r "$expr" <<< "${GH_PR_LIST_JSON:-[]}"
fi
"""

pytestmark = pytest.mark.skipif(shutil.which("jq") is None, reason="needs jq")


def _env(tmp: Path, **extra: str) -> dict[str, str]:
    bin_dir = tmp / "bin"
    bin_dir.mkdir(exist_ok=True)
    gh = bin_dir / "gh"
    gh.write_text(STUB_GH)
    gh.chmod(0o755)
    env = {k: v for k, v in os.environ.items() if not k.startswith(("GITHUB_", "GH_"))}
    env |= {
        "PATH": f"{bin_dir}{os.pathsep}{env['PATH']}",
        "GH_LOG": str(tmp / "gh.log"),
        "GH_TOKEN": "t",
        "GITHUB_REPOSITORY_OWNER": "me",
        "GIT_CONFIG_GLOBAL": str(tmp / "gitconfig"),
        "GIT_CONFIG_NOSYSTEM": "1",
    }
    return env | extra


def _git(repo: Path, *args: str) -> str:
    done = subprocess.run(  # noqa: S603
        [GIT, "-C", str(repo), *args], check=True, capture_output=True, text=True
    )
    return done.stdout


def _repo(tmp: Path) -> Path:
    remote = tmp / "remote.git"
    _git(tmp, "init", "-q", "--bare", str(remote))
    repo = tmp / "repo"
    _git(tmp, "init", "-q", "-b", "main", str(repo))
    _git(repo, "config", "user.email", "t@example.com")
    _git(repo, "config", "user.name", "t")
    (repo / "gen.py").write_text("old\n")
    (repo / "hand.py").write_text("mine\n")
    _git(repo, "add", "-A")
    _git(repo, "commit", "-q", "-m", "init")
    _git(repo, "remote", "add", "origin", str(remote))
    return repo


def _open_pr(
    tmp: Path, repo: Path, manifest: list[str], **extra: str
) -> subprocess.CompletedProcess[str]:
    listing = tmp / "manifest.txt"
    listing.write_text("".join(f"{p}\n" for p in manifest))
    env = _env(
        tmp,
        LABEL="minor",
        BREAKING="0",
        ADDITIVE="1",
        WORKSPACE=str(repo),
        MANIFEST=str(listing),
        **extra,
    )
    return subprocess.run(  # noqa: S603
        [BASH, str(SCRIPTS / "open_pr.sh")], env=env, capture_output=True, text=True, check=False
    )


def _log(tmp: Path) -> str:
    log = tmp / "gh.log"
    return log.read_text() if log.exists() else ""


PRS = [
    {"number": 7, "isCrossRepository": True, "headRepositoryOwner": {"login": "evil"}},
    {"number": 9, "isCrossRepository": False, "headRepositoryOwner": {"login": "me"}},
]


def test_open_pr_ignores_fork_prs(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    (repo / "gen.py").write_text("new\n")
    done = _open_pr(tmp_path, repo, ["gen.py"], GH_PR_LIST_JSON=json.dumps(PRS))
    assert done.returncode == 0, done.stderr
    assert "pr edit 9" in _log(tmp_path)
    assert "pr edit 7" not in _log(tmp_path)


def test_open_pr_creates_when_only_fork_pr(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    (repo / "gen.py").write_text("new\n")
    done = _open_pr(tmp_path, repo, ["gen.py"], GH_PR_LIST_JSON=json.dumps(PRS[:1]))
    assert done.returncode == 0, done.stderr
    assert "pr create" in _log(tmp_path)
    assert "pr edit" not in _log(tmp_path)


def test_open_pr_refuses_unrelated_changes(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    (repo / "gen.py").write_text("new\n")
    (repo / "hand.py").write_text("tampered\n")
    (repo / "stray.txt").write_text("x\n")
    done = _open_pr(tmp_path, repo, ["gen.py"])
    assert done.returncode != 0
    assert "push" not in _git(repo, "reflog")
    assert not _git(tmp_path / "remote.git", "branch", "--list")


def test_open_pr_stages_only_generated_paths(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    (repo / "gen.py").write_text("new\n")
    (repo / "new_gen.py").write_text("added\n")
    done = _open_pr(tmp_path, repo, ["gen.py", "new_gen.py"])
    assert done.returncode == 0, done.stderr
    files = _git(repo, "show", "--name-only", "--format=", "HEAD").split()
    assert sorted(files) == ["gen.py", "new_gen.py"]
    assert _git(tmp_path / "remote.git", "branch", "--list").strip() == "ferrule/regenerate"


def test_open_pr_drops_allow_breaking_on_push(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    (repo / "gen.py").write_text("new\n")
    done = _open_pr(tmp_path, repo, ["gen.py"], GH_PR_LIST_JSON=json.dumps(PRS))
    assert done.returncode == 0, done.stderr
    edit = _log(tmp_path).split("pr edit 9", 1)[1]
    assert "--remove-label major,minor,patch,ferrule:allow-breaking" in edit


def _labels(tmp: Path, event: dict[str, object]) -> str:
    path = tmp / "event.json"
    path.write_text(json.dumps(event))
    env = _env(tmp, GITHUB_EVENT_PATH=str(path), GH_PR_LIST_JSON=json.dumps(PRS))
    done = subprocess.run(  # noqa: S603
        [BASH, str(SCRIPTS / "pr_labels.sh")], env=env, capture_output=True, text=True, check=True
    )
    return done.stdout.strip()


def _pr_event(head: str) -> dict[str, object]:
    return {
        "pull_request": {
            "number": 9,
            "labels": [{"name": "ferrule:allow-breaking"}],
            "head": {"repo": {"full_name": head}},
            "base": {"repo": {"full_name": "me/sdk"}},
        }
    }


def test_labels_from_same_repo_triggering_pr(tmp_path: Path) -> None:
    assert _labels(tmp_path, _pr_event("me/sdk")) == "ferrule:allow-breaking"


def test_labels_ignore_fork_triggering_pr(tmp_path: Path) -> None:
    assert _labels(tmp_path, _pr_event("evil/sdk")) == ""


def test_labels_never_read_other_prs(tmp_path: Path) -> None:
    assert _labels(tmp_path, {"ref": "refs/heads/main"}) == ""
    assert "pr list" not in _log(tmp_path)


STUB_CARGO = """#!/usr/bin/env bash
printf '%s %s\\n' "$PWD" "$*" >> "$CARGO_LOG"
"""

STUB_FERRULE = """#!/usr/bin/env bash
printf '%s %s\\n' "$1" "${FERRULE_GEN:-}" >> "$FERRULE_LOG"
case "$1" in
  generate) mkdir -p "$OUT" && echo gen > "$OUT/gen.py" ;;
  verify) echo '{}' > "$REPORT" ;;
esac
"""


def _regenerate(tmp: Path, **extra: str) -> tuple[Path, str, str]:
    src = tmp / "ferrule"
    (src / "action").mkdir(parents=True)
    env = _env(
        tmp,
        GITHUB_ACTION_PATH=str(src / "action"),
        INPUT_SPEC="openapi.json",
        INPUT_CONFIG="stainless.yml",
        INPUT_AGAINST="HEAD",
        WORKSPACE=str(_repo(tmp)),
        OUT=str(tmp / "out"),
        REPORT=str(tmp / "report.json"),
        MANIFEST=str(tmp / "manifest.txt"),
        CARGO_LOG=str(tmp / "cargo.log"),
        FERRULE_LOG=str(tmp / "ferrule.log"),
        **extra,
    )
    for name, text in (("cargo", STUB_CARGO), ("ferrule", STUB_FERRULE)):
        stub = tmp / "bin" / name
        stub.write_text(text)
        stub.chmod(0o755)
    subprocess.run(  # noqa: S603
        [BASH, str(SCRIPTS / "regenerate.sh")], env=env, capture_output=True, text=True, check=True
    )
    cargo = tmp / "cargo.log"
    return (
        src.resolve(),
        cargo.read_text() if cargo.exists() else "",
        (tmp / "ferrule.log").read_text(),
    )


def test_regenerate_builds_ferrule_gen_from_the_action_checkout(tmp_path: Path) -> None:
    src, cargo, ferrule = _regenerate(tmp_path)
    assert cargo == f"{src} build --release --locked --quiet -p ferrule-gen\n"
    gen = src / "target/release/ferrule-gen"
    assert ferrule.splitlines() == [f"generate {gen}", f"verify {gen}"]


def test_regenerate_uses_a_given_ferrule_gen(tmp_path: Path) -> None:
    _, cargo, ferrule = _regenerate(tmp_path, FERRULE_GEN="/opt/ferrule-gen")
    assert cargo == ""
    assert ferrule.splitlines()[0] == "generate /opt/ferrule-gen"
