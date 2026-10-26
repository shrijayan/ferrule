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


def _open_pr(tmp: Path, repo: Path, **extra: str) -> subprocess.CompletedProcess[str]:
    env = _env(tmp, LABEL="minor", BREAKING="0", ADDITIVE="1", WORKSPACE=str(repo), **extra)
    return subprocess.run(  # noqa: S603
        [BASH, str(SCRIPTS / "open_pr.sh")], env=env, capture_output=True, text=True, check=False
    )


def _regenerated(repo: Path) -> None:
    """What regenerate.sh leaves: a merge commit on ferrule/regenerate and a generated branch."""
    _git(repo, "switch", "-q", "-c", "ferrule/regenerate")
    (repo / "gen.py").write_text("new\n")
    _git(repo, "commit", "-qam", "chore: regenerate SDK")
    _git(repo, "branch", "generated")


def _log(tmp: Path) -> str:
    log = tmp / "gh.log"
    return log.read_text() if log.exists() else ""


PRS = [
    {"number": 7, "isCrossRepository": True, "headRepositoryOwner": {"login": "evil"}},
    {"number": 9, "isCrossRepository": False, "headRepositoryOwner": {"login": "me"}},
]


def test_open_pr_ignores_fork_prs(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    _regenerated(repo)
    done = _open_pr(tmp_path, repo, CHANGED="true", GH_PR_LIST_JSON=json.dumps(PRS))
    assert done.returncode == 0, done.stderr
    assert "pr edit 9" in _log(tmp_path)
    assert "pr edit 7" not in _log(tmp_path)


def test_open_pr_creates_when_only_fork_pr(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    _regenerated(repo)
    done = _open_pr(tmp_path, repo, CHANGED="true", GH_PR_LIST_JSON=json.dumps(PRS[:1]))
    assert done.returncode == 0, done.stderr
    assert "pr create" in _log(tmp_path)
    assert "pr edit" not in _log(tmp_path)


def test_open_pr_does_nothing_without_a_new_generation(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    done = _open_pr(tmp_path, repo, CHANGED="false")
    assert done.returncode == 0, done.stderr
    assert "nothing to open" in done.stdout
    assert not _git(tmp_path / "remote.git", "branch", "--list")
    assert _log(tmp_path) == ""


def test_open_pr_pushes_the_merge_and_the_generated_branch(tmp_path: Path) -> None:
    # The merge commit carries custom code and keep_files as they were; the generated branch is
    # the base of the next regeneration.
    repo = _repo(tmp_path)
    _regenerated(repo)
    done = _open_pr(tmp_path, repo, CHANGED="true")
    assert done.returncode == 0, done.stderr
    remote = tmp_path / "remote.git"
    assert sorted(_git(remote, "branch", "--list", "--format=%(refname:short)").split()) == [
        "ferrule/regenerate",
        "generated",
    ]
    assert _git(remote, "show", "ferrule/regenerate:hand.py") == "mine\n"
    assert _git(remote, "show", "ferrule/regenerate:gen.py") == "new\n"


def test_open_pr_names_a_merge_conflict(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    _regenerated(repo)
    done = _open_pr(tmp_path, repo, CHANGED="true", CONFLICT="true")
    assert done.returncode == 0, done.stderr
    assert "merge conflict" in _log(tmp_path)


def test_open_pr_drops_allow_breaking_on_push(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    _regenerated(repo)
    done = _open_pr(tmp_path, repo, CHANGED="true", GH_PR_LIST_JSON=json.dumps(PRS))
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
printf '%s %s\\n' "$*" "${FERRULE_GEN:-}" >> "$FERRULE_LOG"
case "$1" in
  regenerate)
    if [[ -n "${FERRULE_CONFLICT:-}" ]]; then
      git branch ferrule/conflict-0123456789ab
      exit 1
    fi
    echo gen > gen.py && git commit -qam "chore: regenerate SDK" && git branch -f generated ;;
  verify) echo '{}' > "$REPORT" ;;
esac
"""


def _regenerate(tmp: Path, **extra: str) -> tuple[Path, str, str, Path]:
    src = tmp / "ferrule"
    (src / "action").mkdir(parents=True)
    repo = _repo(tmp)
    env = _env(
        tmp,
        GITHUB_ACTION_PATH=str(src / "action"),
        INPUT_SPEC="openapi.json",
        INPUT_CONFIG="stainless.yml",
        INPUT_AGAINST="HEAD",
        WORKSPACE=str(repo),
        REPORT=str(tmp / "report.json"),
        GITHUB_OUTPUT=str(tmp / "output"),
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
        repo,
    )


def test_regenerate_builds_ferrule_gen_from_the_action_checkout(tmp_path: Path) -> None:
    src, cargo, ferrule, repo = _regenerate(tmp_path)
    assert cargo == f"{src} build --release --locked --quiet -p ferrule-gen\n"
    gen = src / "target/release/ferrule-gen"
    assert ferrule.splitlines() == [
        f"regenerate --repo {repo} --spec {repo}/openapi.json --config {repo}/stainless.yml {gen}",
        f"verify --repo {repo} --against HEAD --new {repo} --json {tmp_path}/report.json {gen}",
    ]
    assert _git(repo, "branch", "--show-current").strip() == "ferrule/regenerate"
    assert (tmp_path / "output").read_text() == "changed=true\nconflict=false\n"


def test_regenerate_uses_a_given_ferrule_gen(tmp_path: Path) -> None:
    _, cargo, ferrule, _ = _regenerate(tmp_path, FERRULE_GEN="/opt/ferrule-gen")
    assert cargo == ""
    assert ferrule.splitlines()[0].endswith(" /opt/ferrule-gen")


def test_regenerate_hands_a_conflict_to_the_pr(tmp_path: Path) -> None:
    _, _, ferrule, repo = _regenerate(tmp_path, FERRULE_CONFLICT="1", FERRULE_GEN="/g")
    assert [line.split()[0] for line in ferrule.splitlines()] == ["regenerate"]
    assert _git(repo, "branch", "--show-current").strip() == "ferrule/conflict-0123456789ab"
    assert (tmp_path / "output").read_text() == "changed=true\nconflict=true\n"
    report = json.loads((tmp_path / "report.json").read_text())
    assert report["breaking_count"] == 0
