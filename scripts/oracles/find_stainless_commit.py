"""Print the last stainless-app[bot] commit and its .stats.yml for GitHub repos.

Usage: python3 scripts/oracles/find_stainless_commit.py owner/repo [owner/repo ...]

Requires an authenticated `gh` CLI. The method is described in oracles/README.md.
"""

from __future__ import annotations

import base64
import json
import subprocess
import sys
from typing import cast

BOT = "stainless-app[bot]"
MAX_PAGES = 10


def gh(path: str) -> object:
    """Call `gh api` and decode its JSON."""
    out = subprocess.run(  # noqa: S603 - fixed argv, no shell
        ["gh", "api", path],  # noqa: S607 - gh from PATH is intended
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return cast("object", json.loads(out))


def last_stainless_commit(repo: str) -> dict[str, object] | None:
    """Return the newest default-branch commit whose GitHub author is the Stainless bot."""
    for page in range(1, MAX_PAGES + 1):
        commits = cast(
            "list[dict[str, object]]", gh(f"repos/{repo}/commits?per_page=100&page={page}")
        )
        for commit in commits:
            author = cast("dict[str, object] | None", commit.get("author"))
            if author is not None and author.get("login") == BOT:
                return commit
        if len(commits) < 100:  # noqa: PLR2004 - page size
            break
    return None


def main(argv: list[str]) -> int:
    """Entry point."""
    for repo in argv:
        commit = last_stainless_commit(repo)
        if commit is None:
            sys.stdout.write(f"{repo}: no {BOT} commit in the last {MAX_PAGES * 100}\n")
            continue
        sha = cast("str", commit["sha"])
        info = cast("dict[str, object]", commit["commit"])
        message = cast("str", info["message"]).splitlines()[0]
        date = cast("dict[str, str]", info["committer"])["date"]
        content = cast("dict[str, str]", gh(f"repos/{repo}/contents/.stats.yml?ref={sha}"))
        stats = base64.b64decode(content["content"]).decode()
        sys.stdout.write(f"{repo} {sha} {date} {message}\n{stats}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
