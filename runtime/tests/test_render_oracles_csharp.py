# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered C# runtime is byte-identical to the pinned oracle SDKs' files.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. The template comes from imagekit-dotnet (Apache-2.0).
stagehand-net (MIT) is only compared with: nothing of it is in the template.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

from ferrule_runtime import CSharpOptions, render_csharp_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"

# What the plugin generates into the slot, after csharpier: imagekit's basic auth on the
# private key (`password` defaults to a dummy) and stagehand's two API-key headers.
IMAGEKIT_AUTH = """\
        if (options.PrivateKey != null && options.Password != null)
        {
            request.Headers.Add(
                "Authorization",
                string.Format(
                    "Basic {0}",
                    Convert.ToBase64String(
                        Encoding
                            .GetEncoding("ISO-8859-1")
                            .GetBytes(
                                string.Format("{0}:{1}", options.PrivateKey, options.Password)
                            )
                    )
                )
            );
        }
"""
STAGEHAND_AUTH = """\
        if (options.BrowserbaseApiKey != null)
        {
            request.Headers.Add("x-bb-api-key", options.BrowserbaseApiKey);
        }
        if (options.ModelApiKey != null)
        {
            request.Headers.Add("x-model-api-key", options.ModelApiKey);
        }
"""


@dataclass(frozen=True)
class Oracle:
    name: str
    options: CSharpOptions
    not_in_template: frozenset[str] = frozenset()
    """Runtime files of the oracle the template does not have, with the namespace written NS."""

    def path(self, rel: str) -> str:
        return rel.replace("NS", self.options.namespace)


ORACLES = [
    Oracle(
        "imagekit-csharp",
        CSharpOptions(
            namespace="Imagekit",
            client_name="ImageKit",
            env_prefix="IMAGE_KIT",
            auth_headers=IMAGEKIT_AUTH,
        ),
    ),
    Oracle(
        "stagehand-csharp",
        CSharpOptions(
            namespace="Stagehand",
            client_name="Stagehand",
            env_prefix="STAGEHAND",
            auth_headers=STAGEHAND_AUTH,
        ),
        # Server-sent events: stagehand streams, imagekit does not, and stagehand-net is MIT,
        # so its streaming runtime is not copied (streaming is not supported yet).
        frozenset(
            {
                "src/NS/Core/Sse.cs",
                "src/NS/Exceptions/StagehandSseException.cs",
                "src/NS.Tests/SseTest.cs",
            }
        ),
    ),
]

# Generated per SDK: the client options (client_settings.opts, environments), the environment
# URLs, the JSON options with every enum converter, and the test base (the client's options).
GENERATED = frozenset(
    {
        "src/NS/Core/ClientOptions.cs",
        "src/NS/Core/EnvironmentUrl.cs",
        "src/NS/Core/ModelBase.cs",
        "src/NS.Tests/TestBase.cs",
    }
)
# Hand-written in the oracles: imagekit's upload and URL helpers, stagehand's base-URL test.
CUSTOM_DIRS = ("src/NS/Helper/", "src/NS.Tests/Helper/")
CUSTOM = frozenset({"src/NS.Tests/ClientOptionsTest.cs"})


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / ".stats.yml").is_file():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str) -> bool:
    path = root / rel
    return path.is_file() and path.read_bytes() == text.encode()


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_csharp_files(oracle.options)
    mismatched = sorted(rel for rel, text in files.items() if not _matches(root, rel, text))
    assert mismatched == []


def _runtime_on_disk(root: Path, ns: str) -> set[str]:
    """Every `.cs` file the runtime could hold: those of the SDK and its tests outside
    `Models/` and `Services/`, but the client and its interface (`src/<Ns>/*Client.cs`)."""
    found: set[str] = set()
    for project in (ns, f"{ns}.Tests"):
        base = root / "src" / project
        for p in base.rglob("*.cs"):
            rel = p.relative_to(base).as_posix()
            client = project == ns and "/" not in rel and rel.endswith("Client.cs")
            if rel.split("/")[0] not in {"Models", "Services"} and not client:
                found.add(p.relative_to(root).as_posix())
    return found


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    ns = oracle.options.namespace
    rendered = set(render_csharp_files(oracle.options))
    skipped = {oracle.path(rel) for rel in GENERATED | CUSTOM | oracle.not_in_template}
    custom_dirs = tuple(oracle.path(d) for d in CUSTOM_DIRS)
    expected = {
        rel for rel in _runtime_on_disk(root, ns) - skipped if not rel.startswith(custom_dirs)
    }
    assert expected - rendered == set()
    assert rendered - expected == set()
