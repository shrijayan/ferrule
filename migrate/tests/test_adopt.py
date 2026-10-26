# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import functools
import hashlib
import json
import re
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path, PurePosixPath

import pytest

from ferrule import cli
from ferrule.adopt import (
    ACTION_SHA,
    BRANCH,
    CHECKOUT_SHA,
    AdoptRequest,
    Git,
    adopt,
    forge_commit,
    keep_files,
    patch_name,
    slug,
    workflow_text,
)
from ferrule.errors import FerruleError
from ferrule.generate import GenVersion, Tools
from ferrule.lock import Lock, parse
from ferrule.verify import Change, Report

SPEC = b'{"openapi": "3.1.0"}\n'
MD5 = hashlib.md5(SPEC, usedforsecurity=False).hexdigest()
BASE_FOO = "".join(f"line{i}\n" for i in range(1, 9))
ENV = {"FERRULE_FORGE_COMMIT": "f" * 40, "PATH": "/usr/bin:/bin", "HOME": "/nonexistent"}
REV = "91f52a40c746e8a2ac0271d8be0b7c82cdf28798"


def write(path: Path, text: str = "") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    return path


def sh(repo: Path, *args: str, env: dict[str, str] | None = None) -> str:
    return subprocess.run(  # noqa: S603
        ["git", "-C", str(repo), "-c", "core.hooksPath=/dev/null", *args],  # noqa: S607
        check=True,
        capture_output=True,
        text=True,
        env={**ENV, **(env or {})},
    ).stdout.strip()


COMMITTER = {"GIT_COMMITTER_NAME": "Ferrule Tests", "GIT_COMMITTER_EMAIL": "tests@example.com"}


def commit(repo: Path, message: str, author: str) -> None:
    sh(repo, "add", "-A")
    # HOME is /nonexistent, so repos without user.* config would have no committer identity.
    sh(repo, "commit", "-q", "-m", message, f"--author={author}", env=COMMITTER)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    root = tmp_path / "sdk"
    sh(tmp_path, "init", "-q", "-b", "main", str(root))
    sh(root, "config", "user.name", "Dev")
    sh(root, "config", "user.email", "dev@example.com")
    write(
        root / ".stats.yml", f"openapi_spec_url: https://x.dev/s.json\nopenapi_spec_hash: {MD5}\n"
    )
    write(root / "stainless.yml", "targets:\n  python:\n    keep_files:\n      - README.md\n")
    write(root / "README.md", "readme\n")
    write(root / "src/acme/__init__.py", "from acme._client import Acme\n")
    write(root / "src/acme/_client.py", "class Acme: ...\n")
    write(root / "src/acme/_types.py", "RUNTIME = 1\n")
    write(root / "src/acme/_utils/__init__.py", "")
    write(root / "src/acme/resources/foo.py", BASE_FOO)
    commit(root, "codegen", "stainless-app[bot] <bot@stainless.example>")
    write(root / "src/acme/resources/foo.py", BASE_FOO.replace("line8", "custom8"))
    write(root / "src/acme/lib/custom.py", "def helper() -> int:\n    return 1\n")
    commit(root, "hand edit", "Dev <dev@example.com>")
    return root


def fake_runner(foo: str = BASE_FOO.replace("line1", "gen1")):  # noqa: ANN201
    def run(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        if tuple(argv[1:]) == ("--version",):
            return subprocess.CompletedProcess(argv, 0, f"ferrule-gen 0.0.0 forge {REV}\n", "")
        if argv[1] == "generate":
            out = Path(argv[argv.index("--out") + 1])
            write(out / "src/acme/__init__.py", "from acme._client import Acme\n")
            write(out / "src/acme/_client.py", "class Acme:\n    def ping(self) -> None: ...\n")
            write(out / "src/acme/resources/foo.py", foo)
            write(out / "src/acme/lib/custom.py", "# generator must not win here\n")
            write(out / "README.md", "generated readme\n")
            write(out / "api.md", "# api\n")
        return subprocess.CompletedProcess(argv, 0, "", "")

    return run


def fixed_tools(tools: Tools):  # noqa: ANN201
    def resolve(*_a: object, **_k: object) -> Tools:
        return tools

    return resolve


def fetch(url: str, limit: int, timeout: float) -> bytes:
    del url, limit, timeout
    return SPEC


@pytest.fixture
def tools(tmp_path: Path) -> Tools:
    return Tools(gen=write(tmp_path / "tools/ferrule-gen"), ruff=sys.executable)


def render(package: str, package_dir: Path, /) -> None:
    write(package_dir / "_utils/__init__.py", f"# {package} runtime\n")


def run_adopt(repo: Path, tools: Tools, **kwargs: object):  # noqa: ANN201
    lines: list[str] = []
    request = AdoptRequest(repo, tools, **kwargs)  # type: ignore[arg-type]
    result = adopt(
        request, runner=fake_runner(), renderer=render, fetch=fetch, env=ENV, say=lines.append
    )
    return result, lines


def show(repo: Path, path: str) -> str:
    return sh(repo, "show", f"{BRANCH}:{path}")


def test_adopt_end_to_end(repo: Path, tools: Tools) -> None:
    head = sh(repo, "rev-parse", "HEAD")
    result, lines = run_adopt(repo, tools)
    assert result.branch == BRANCH
    assert result.patches == ["0001-src-acme-resources-foo-py.patch"]
    assert lines[0].startswith("✓ read .stats.yml")
    assert "✓ found 1 hand edits → .ferrule/patches/" in lines
    assert "✓ generated via ferrule-gen (forge + ferrule-python)" in lines
    # the user's checkout is untouched
    assert sh(repo, "rev-parse", "HEAD") == head
    assert sh(repo, "status", "--porcelain") == ""
    assert sh(repo, "worktree", "list").count("\n") == 0
    # three-way: regenerated line1 and the hand-edited line8 both survive
    foo = show(repo, "src/acme/resources/foo.py")
    assert "gen1" in foo
    assert "custom8" in foo
    # lib/ and keep_files are never written; the generated client is
    assert show(repo, "src/acme/lib/custom.py").startswith("def helper")
    assert show(repo, "README.md") == "readme"
    assert "ping" in show(repo, "src/acme/_client.py")
    assert show(repo, "src/acme/_types.py") == "RUNTIME = 1"
    assert show(repo, "openapi.json") == SPEC.decode().strip()
    assert "custom8" in show(repo, ".ferrule/patches/0001-src-acme-resources-foo-py.patch")
    lock_text = show(repo, "ferrule.lock") + "\n"
    lock = parse(lock_text)
    assert lock.forge_commit == "f" * 40
    assert lock.generator == "ferrule-gen 0.0.0"
    assert json.loads(lock_text)["lock_version"] == 2
    assert "plugin_sha256" not in lock_text
    assert "src/acme/lib/custom.py" not in lock.files
    assert "README.md" not in lock.files
    assert set(lock.files) == {
        "api.md",
        "src/acme/__init__.py",
        "src/acme/_client.py",
        "src/acme/resources/foo.py",
    }
    assert "ferrule-dev/action@" in show(repo, ".github/workflows/sdk.yml")
    message = sh(repo, "log", "-1", "--format=%an <%ae>%n%B", BRANCH)
    assert message.startswith("Dev <dev@example.com>\nSwitch SDK generation to Ferrule")
    assert "Signed-off-by: Dev <dev@example.com>" in message
    assert sh(repo, "rev-parse", f"{BRANCH}~1") == head
    assert result.against == sh(repo, "rev-parse", "HEAD~1")


def test_adopt_keeps_a_hand_edit_a_later_stainless_commit_carried(repo: Path, tools: Tools) -> None:
    # Stainless merges custom code into every later generation, so the last bot commit already
    # holds the edit and diffing from it finds nothing. Reduced from isaacus-python 0.22.0:
    # Span.decode, added by a maintainer in bc41468, carried by the bot from 75d28d4 on.
    carried = BASE_FOO.replace("line2", "bot2").replace("line8", "custom8")
    write(repo / "src/acme/resources/foo.py", carried)
    commit(repo, "codegen", "stainless-app[bot] <bot@stainless.example>")
    runner = fake_runner(BASE_FOO.replace("line1", "gen1").replace("line2", "bot2"))
    result = adopt(AdoptRequest(repo, tools), runner=runner, fetch=fetch, env=ENV)
    assert result.patches == ["0001-src-acme-resources-foo-py.patch"]
    foo = show(repo, "src/acme/resources/foo.py")
    assert foo == BASE_FOO.replace("line1", "gen1").replace("line2", "bot2").replace(
        "line8", "custom8"
    ).rstrip("\n")


BOT = "stainless-app[bot] <bot@stainless.example>"
SPAN = 'class Span:\n    start: int\n    """Start."""\n\n    end: int\n    """End."""\n'
DECODE = "\n    def decode(self, text: str) -> str:\n        return text[self.start : self.end]\n"


def test_adopt_merges_a_method_into_a_class_the_generator_reorders(
    repo: Path, tools: Tools
) -> None:
    # isaacus-python: Ferrule emits Span's fields in another order, so the line merge
    # conflicts and the libcst merge places decode.
    write(repo / "src/acme/resources/foo.py", SPAN)
    commit(repo, "codegen", BOT)
    write(repo / "src/acme/resources/foo.py", SPAN + DECODE)
    commit(repo, "feat: add decode", "Dev <dev@example.com>")
    write(repo / "src/acme/resources/foo.py", SPAN.replace("Start.", "Start index.") + DECODE)
    commit(repo, "codegen", BOT)
    reordered = (
        'class Span:\n    end: int\n    """End."""\n\n    start: int\n    """Start index."""\n'
    )
    result = adopt(AdoptRequest(repo, tools), runner=fake_runner(reordered), fetch=fetch, env=ENV)
    assert result.patches == ["0001-src-acme-resources-foo-py.patch"]
    foo = show(repo, "src/acme/resources/foo.py")
    assert "def decode(self, text: str) -> str:" in foo
    assert foo.index("end: int") < foo.index("start: int")
    assert "Start index." in foo


def test_adopt_drops_a_hand_edit_a_later_generation_overwrote(repo: Path, tools: Tools) -> None:
    write(repo / "src/acme/resources/foo.py", BASE_FOO.replace("line2", "bot2"))
    commit(repo, "codegen", BOT)
    result, _ = run_adopt(repo, tools, dry_run=True)
    assert result.patches == []


def test_adopt_drops_a_hand_edit_the_generator_now_writes(repo: Path, tools: Tools) -> None:
    runner = fake_runner(BASE_FOO.replace("line8", "custom8"))
    result = adopt(AdoptRequest(repo, tools, dry_run=True), runner=runner, fetch=fetch, env=ENV)
    assert result.patches == []


def test_adopt_ignores_an_edit_to_a_file_head_no_longer_has(repo: Path, tools: Tools) -> None:
    write(repo / "api.md", "# hand-written api\n")
    commit(repo, "docs: api", "Dev <dev@example.com>")
    (repo / "api.md").unlink()
    commit(repo, "docs: drop api", "Dev <dev@example.com>")
    result, _ = run_adopt(repo, tools, dry_run=True)
    assert result.patches == ["0001-src-acme-resources-foo-py.patch"]


def test_adopt_dry_run(repo: Path, tools: Tools) -> None:
    result, _ = run_adopt(repo, tools, dry_run=True)
    assert result.branch is None
    assert result.patches == ["0001-src-acme-resources-foo-py.patch"]
    assert sh(repo, "branch", "--list", BRANCH) == ""


def test_adopt_with_spec_and_release_tag(repo: Path, tools: Tools, tmp_path: Path) -> None:
    sh(repo, "tag", "v1.0.0")
    spec = write(tmp_path / "given.yaml", "openapi: 3.1.0\n")
    result, lines = run_adopt(repo, tools, spec=spec)
    assert result.against == "v1.0.0"
    assert not any(".stats.yml" in line for line in lines)
    assert show(repo, "openapi.yaml") == "openapi: 3.1.0"


def test_adopt_conflict(repo: Path, tools: Tools) -> None:
    runner = fake_runner(BASE_FOO.replace("line8", "generated8"))
    with pytest.raises(FerruleError) as exc:
        adopt(AdoptRequest(repo, tools), runner=runner, fetch=fetch, env=ENV)
    assert exc.value.diagnostic.code == "FER4102"
    assert "0001-src-acme-resources-foo-py.patch" in exc.value.diagnostic.message
    assert sh(repo, "branch", "--list", BRANCH) == ""
    assert sh(repo, "worktree", "list").count("\n") == 0


def test_adopt_falls_back_to_stats_commit(tmp_path: Path, tools: Tools) -> None:
    root = tmp_path / "plain"
    sh(tmp_path, "init", "-q", "-b", "main", str(root))
    sh(root, "config", "user.name", "Dev")
    sh(root, "config", "user.email", "dev@example.com")
    write(root / "stainless.yml", "{}\n")
    write(root / "src/acme/__init__.py", "")
    write(root / "src/acme/_utils/__init__.py", "")
    commit(root, "init", "Dev <dev@example.com>")
    write(
        root / ".stats.yml", f"openapi_spec_url: https://x.dev/s.json\nopenapi_spec_hash: {MD5}\n"
    )
    commit(root, "stats", "Dev <dev@example.com>")
    write(root / "notes.txt", "x\n")
    commit(root, "later", "Dev <dev@example.com>")
    result, _ = run_adopt(root, tools, dry_run=True)
    assert result.base == sh(root, "rev-parse", "HEAD~1")
    assert result.patches == []


def test_adopt_no_stainless_history(tmp_path: Path, tools: Tools) -> None:
    # An SDK another generator wrote (opencode-python-sdk): given its spec, it is adopted from
    # HEAD. Nothing it has was generated by Stainless, so no hand edit is recorded; the
    # generator's files replace the other generator's and everything else is kept.
    root = tmp_path / "none"
    sh(tmp_path, "init", "-q", "-b", "main", str(root))
    sh(root, "config", "user.name", "Dev")
    sh(root, "config", "user.email", "dev@example.com")
    write(root / "stainless.yml", "{}\n")
    write(root / "src/acme/__init__.py", "old\n")
    write(root / "src/acme/resources/foo.py", "# written by another generator\n")
    write(root / "src/acme/_rpc.py", "custom\n")
    commit(root, "init", "Dev <dev@example.com>")
    write(root / "src/acme/resources/foo.py", "# edited after it\n")
    commit(root, "edit", "Dev <dev@example.com>")
    spec = write(tmp_path / "openapi.json", "{}\n")
    result, lines = run_adopt(root, tools, spec=spec)
    assert result.base == sh(root, "rev-parse", "HEAD")
    assert result.patches == []
    assert any("no Stainless history" in line for line in lines)
    assert show(root, "src/acme/resources/foo.py") == BASE_FOO.replace("line1", "gen1").rstrip("\n")
    assert show(root, "src/acme/_rpc.py") == "custom"


def test_adopt_no_stainless_history_without_spec(tmp_path: Path, tools: Tools) -> None:
    root = tmp_path / "none"
    sh(tmp_path, "init", "-q", "-b", "main", str(root))
    write(root / "stainless.yml", "{}\n")
    commit(root, "init", "Dev <dev@example.com>")
    with pytest.raises(FerruleError, match=r"\.stats\.yml"):
        run_adopt(root, tools)


@pytest.mark.parametrize(
    ("setup", "needle"),
    [
        ("dirty", "uncommitted"),
        ("branch", "already exists"),
        ("symlink", "symlinks"),
        ("subdir", "not a git repository root"),
        ("no-config", "does not exist"),
        ("no-spec", "does not exist"),
        ("no-identity", "user.name"),
    ],
)
def test_adopt_refusals(repo: Path, tools: Tools, setup: str, needle: str) -> None:
    kwargs: dict[str, object] = {}
    target = repo
    if setup == "dirty":
        write(repo / "README.md", "changed\n")
    elif setup == "branch":
        sh(repo, "branch", BRANCH)
    elif setup == "symlink":
        (repo / "link").symlink_to("README.md")
        commit(repo, "link", "Dev <dev@example.com>")
    elif setup == "subdir":
        target = repo / "src"
    elif setup == "no-config":
        kwargs["config"] = repo / "missing.yml"
    elif setup == "no-spec":
        kwargs["spec"] = repo / "missing.json"
    else:
        sh(repo, "config", "--unset", "user.name")
    with pytest.raises(FerruleError, match=needle):
        run_adopt(target, tools, **kwargs)
    assert sh(repo, "branch", "--list", BRANCH) == ("" if setup != "branch" else BRANCH)


def test_adopt_config_outside_repo(repo: Path, tools: Tools, tmp_path: Path) -> None:
    config = write(tmp_path / "elsewhere.yml", "{}\n")
    run_adopt(repo, tools, config=config)
    assert show(repo, "stainless.yml") == "{}"
    assert "paths: [openapi.json, stainless.yml]" in show(repo, ".github/workflows/sdk.yml")


def test_git_failure_is_a_diagnostic(tmp_path: Path) -> None:
    with pytest.raises(FerruleError) as exc:
        Git(tmp_path, ENV).run("log")
    assert exc.value.diagnostic.code == "FER4105"


def test_workflow_pins_full_shas() -> None:
    text = workflow_text("openapi.json", "stainless.yml")
    pins = re.findall(r"uses: \S+@(\S+)", text)
    assert pins == [CHECKOUT_SHA, ACTION_SHA]
    assert all(re.fullmatch(r"[0-9a-f]{40}", pin) for pin in pins)
    repo_root = Path(__file__).resolve().parents[2]
    ci = (repo_root / ".github/workflows/ci.yml").read_text()
    assert f"actions/checkout@{CHECKOUT_SHA}" in ci


def test_keep_files() -> None:
    text = (
        "a:\n  keep_files:\n    - README.md # why\n    - 'src/x.py'\n  other: 1\n"
        'b:\n  keep_files: [api.md, "y.py"]\n'
        "c:\n  keep_files:\nd: 1\n"
    )
    assert keep_files(text) == {"README.md", "src/x.py", "api.md", "y.py"}


def test_names() -> None:
    assert slug(PurePosixPath("src/Acme/_client.py")) == "src-acme-client-py"
    assert slug(PurePosixPath("___")) == "file"
    assert patch_name(12, PurePosixPath("a/b.py")) == "0012-a-b-py.patch"


def test_forge_commit() -> None:
    version = GenVersion("0.0.0", REV)
    assert forge_commit(version, {"FERRULE_FORGE_COMMIT": "abc"}) == "abc"
    assert forge_commit(version, {"FERRULE_FORGE_COMMIT": ""}) == REV
    assert forge_commit(version, {}) == REV


def test_adopt_lock_names_the_forge_rev_of_ferrule_gen(repo: Path, tools: Tools) -> None:
    env = {k: v for k, v in ENV.items() if k != "FERRULE_FORGE_COMMIT"}
    adopt(AdoptRequest(repo, tools), runner=fake_runner(), fetch=fetch, env=env)
    lock = parse(show(repo, "ferrule.lock") + "\n")
    assert (lock.forge_commit, lock.generator) == (REV, "ferrule-gen 0.0.0")


def test_adopt_refuses_a_gen_without_a_forge_rev(repo: Path, tools: Tools) -> None:
    generate = fake_runner()

    def old_binary(argv: Sequence[str]) -> subprocess.CompletedProcess[str]:
        if tuple(argv[1:]) == ("--version",):
            return subprocess.CompletedProcess(argv, 0, "ferrule-gen 0.0.0\n", "")
        return generate(argv)

    with pytest.raises(FerruleError, match="FER4002 at FERRULE_GEN"):
        adopt(AdoptRequest(repo, tools), runner=old_binary, fetch=fetch, env=ENV)
    assert sh(repo, "branch", "--list", BRANCH) == ""


def test_lock_is_deterministic() -> None:
    a = Lock("0.0.0", "f", "ferrule-gen 0.0.0", "s", "c", {"b.py": "2", "a.py": "1"})
    b = Lock("0.0.0", "f", "ferrule-gen 0.0.0", "s", "c", {"a.py": "1", "b.py": "2"})
    assert a.render() == b.render()
    assert a.render().endswith("}\n")
    assert list(json.loads(a.render())["files"]) == ["a.py", "b.py"]
    assert parse(a.render()) == a
    assert a.render() == (
        "{\n"
        '  "config_sha256": "c",\n'
        '  "ferrule_version": "0.0.0",\n'
        '  "files": {\n'
        '    "a.py": "1",\n'
        '    "b.py": "2"\n'
        "  },\n"
        '  "forge_commit": "f",\n'
        '  "generator": "ferrule-gen 0.0.0",\n'
        '  "lock_version": 2,\n'
        '  "spec_sha256": "s"\n'
        "}\n"
    )
    with pytest.raises(TypeError):
        parse(json.dumps({**json.loads(a.render()), "files": []}))


def test_lock_reads_version_1() -> None:
    # Repos adopted before ferrule-gen carry plugin.wasm's hash; it is ignored, not an error.
    v1 = {
        "config_sha256": "c",
        "ferrule_version": "0.0.0",
        "files": {"a.py": "1"},
        "forge_commit": REV,
        "lock_version": 1,
        "plugin_sha256": "9" * 64,
        "spec_sha256": "s",
    }
    lock = parse(json.dumps(v1, indent=2, sort_keys=True) + "\n")
    assert lock == Lock("0.0.0", REV, "unknown", "s", "c", {"a.py": "1"})
    assert "plugin_sha256" not in lock.render()


def test_cli_adopt(
    repo: Path, tools: Tools, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(cli, "resolve_tools", fixed_tools(tools))
    monkeypatch.setattr(
        cli, "adopt", functools.partial(adopt, runner=fake_runner(), fetch=fetch, env=ENV)
    )
    assert cli.main(["adopt", str(repo), "--dry-run"]) == 0
    assert cli.main(["adopt", str(repo)]) == 0
    out = capsys.readouterr().out
    assert "ferrule verify --against" in out
    assert "public API: 0 breaking" in out


def test_cli_adopt_breaking(
    repo: Path, tools: Tools, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    def broken(old: Path, new: Path, package: str | None = None) -> Report:
        del old, new, package
        return Report("acme", breaking=[Change("removed", "acme.Acme", "acme.Acme removed")])

    monkeypatch.setattr(cli, "resolve_tools", fixed_tools(tools))
    monkeypatch.setattr(cli, "verify", broken)
    monkeypatch.setattr(
        cli, "adopt", functools.partial(adopt, runner=fake_runner(), fetch=fetch, env=ENV)
    )
    assert cli.main(["adopt", str(repo)]) == 1
    assert "breaking acme.Acme removed" in capsys.readouterr().out
