# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""``ferrule regenerate``: regenerate an adopted SDK and merge it into the checked-out branch.

The new generation (generated files, minus ``keep_files`` and ``lib/``, plus a fresh
``ferrule.lock``) is committed on the ``generated`` branch and merged three-way into the
current branch, from the last generation that branch took (its ``Ferrule-Generated-From``
trailer; ``ferrule adopt`` writes the first). Custom code, in its own files or in generated
ones, survives; a conflict lands on ``ferrule/conflict-<sha>`` instead
(:func:`ferrule.merge.gitio.regenerate_branch`). Nothing is pushed.

The runtime is copied from the repository when it is the SDK's own (adopted from Stainless),
and rendered when Ferrule owns it (``ferrule.lock`` lists it).
"""

from __future__ import annotations

import os
import tempfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

from ferrule import __version__
from ferrule.adopt import forge_commit, is_protected, keep_files, recording
from ferrule.errors import MISSING_INPUT, fail
from ferrule.generate import (
    RUNTIME_DIRS,
    GenerateRequest,
    RuntimeRenderer,
    Tools,
    gen_version,
    generate,
    run_subprocess,
)
from ferrule.lock import LOCK_FILE, Lock, parse, sha256_bytes, sha256_file
from ferrule.merge import KeepRules
from ferrule.merge.gitio import Regeneration, regenerate_branch

if TYPE_CHECKING:
    from collections.abc import Mapping

    from ferrule.generate import Runner

MESSAGE = "chore: regenerate SDK"


@dataclass(frozen=True)
class RegenerateRequest:
    """Inputs of one ``ferrule regenerate`` run; ``spec`` and ``config`` are read as they are."""

    repo: Path
    spec: Path
    config: Path
    tools: Tools


def owns_runtime(repo: Path) -> bool:
    """Whether ``ferrule.lock`` lists runtime files: Ferrule rendered the runtime at adoption."""
    lock_file = repo / LOCK_FILE
    if not lock_file.is_file():
        return False
    files = parse(lock_file.read_text(encoding="utf-8")).files
    return any(set(PurePosixPath(f).parts) & RUNTIME_DIRS for f in files)


def regenerate(
    request: RegenerateRequest,
    *,
    runner: Runner | None = None,
    renderer: RuntimeRenderer | None = None,
    env: Mapping[str, str] | None = None,
) -> Regeneration:
    """Regenerate and merge; returns what the merge did. Raises FerruleError on any problem."""
    run = runner or run_subprocess
    env = dict(os.environ) if env is None else env
    repo = request.repo.resolve()
    for label, path in (("--spec", request.spec), ("--config", request.config)):
        if not path.is_file():
            raise fail(MISSING_INPUT, f"{path} does not exist", label, "pass an existing file")
    config_text = request.config.read_text(encoding="utf-8")
    keep = keep_files(config_text)
    runtime_from = None if owns_runtime(repo) else repo
    with tempfile.TemporaryDirectory(prefix="ferrule-regenerate-") as tmp:
        out = Path(tmp) / "out"
        seen: list[PurePosixPath] = []
        result = generate(
            GenerateRequest(request.spec, request.config, out, request.tools, runtime_from),
            runner=recording(run, request.tools, seen),
            renderer=renderer,
        )
        written = sorted({*seen, *result.rendered})
        generated = [p for p in written if not is_protected(p, keep)]
        version = gen_version(request.tools, run)
        lock = Lock(
            ferrule_version=__version__,
            forge_commit=forge_commit(version, env),
            generator=version.generator,
            spec_sha256=sha256_bytes(request.spec.read_bytes()),
            config_sha256=sha256_bytes(config_text.encode()),
            files={str(p): sha256_file(out / p) for p in generated},
        )
        tree = {str(p): (out / p).read_bytes() for p in generated}
        tree[LOCK_FILE] = lock.render().encode()
    return regenerate_branch(repo, tree, keep=KeepRules(tuple(sorted(keep))), message=MESSAGE)
