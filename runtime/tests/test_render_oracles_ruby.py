# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered Ruby runtime is byte-identical to the pinned oracle gems' files.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. All three are Apache-2.0; the template comes from OneBusAway's
ruby-sdk.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

from ferrule_runtime import RubyOptions, render_ruby_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"

# What the plugin generates into the example slots: the first model, enum and union.
OBA_BASE_MODEL_EXAMPLE = """\
        #
        # @example
        #   # `references` is a `OnebusawaySDK::References`
        #   references => {
        #     agencies: agencies,
        #     routes: routes,
        #     situations: situations
        #   }
"""
IMAGEKIT_BASE_MODEL_EXAMPLE = """\
        #
        # @example
        #   # `ai_tag` is a `Imagekitio::AITag`
        #   ai_tag => {
        #     confidence: confidence,
        #     name: name,
        #     source: source
        #   }
"""
IMAGEKIT_ENUM_EXAMPLE = """\
      #
      # @example
      #   # `streaming_resolution` is a `Imagekitio::StreamingResolution`
      #   case streaming_resolution
      #   when Imagekitio::StreamingResolution::STREAMING_RESOLUTION_240
      #     # ...
      #   when Imagekitio::StreamingResolution::STREAMING_RESOLUTION_360
      #     # ...
      #   when Imagekitio::StreamingResolution::STREAMING_RESOLUTION_480
      #     # ...
      #   else
      #     puts(streaming_resolution)
      #   end
      #
      # @example
      #   case streaming_resolution
      #   in :"240"
      #     # ...
      #   in :"360"
      #     # ...
      #   in :"480"
      #     # ...
      #   else
      #     puts(streaming_resolution)
      #   end
"""
IMAGEKIT_UNION_EXAMPLE = """\
      #
      # @example
      #   # `extension_config` is a `Imagekitio::ExtensionConfig`
      #   case extension_config
      #   when Imagekitio::ExtensionConfig::RemoveBg
      #     puts(extension_config.name)
      #   when Imagekitio::ExtensionConfig::AIAutoDescription
      #     puts(extension_config.name)
      #   when Imagekitio::ExtensionConfig::AITasks
      #     puts(extension_config.tasks)
      #   else
      #     puts(extension_config)
      #   end
      #
      # @example
      #   case extension_config
      #   in {name: :"remove-bg", options: options}
      #     puts(options)
      #   in {name: :"ai-auto-description"}
      #     # ...
      #   in {name: :"ai-tasks", tasks: tasks}
      #     puts(tasks)
      #   else
      #     puts(extension_config)
      #   end
"""


@dataclass(frozen=True)
class Oracle:
    name: str
    options: RubyOptions
    known_drift: frozenset[str] = frozenset()
    """Files whose text comes from another Stainless generation (runtime/README.md), with the
    gem directory written ``GEM``."""
    generated: frozenset[str] = frozenset()
    """Runtime-adjacent files the plugin generates (pagination), written like known_drift."""

    def path(self, rel: str) -> str:
        return rel.replace("GEM", self.options.gem_name)


# knock-ruby's pin is a 2025 generation (older coercion state, no `write_query_param_element!`,
# older errors and file parts); slots are left empty because these files differ anyway.
_KNOCK_2025 = frozenset(
    {
        "lib/GEM/errors.rb",
        "lib/GEM/file_part.rb",
        "lib/GEM/internal/transport/base_client.rb",
        "lib/GEM/internal/transport/pooled_net_requester.rb",
        "lib/GEM/internal/type/array_of.rb",
        "lib/GEM/internal/type/base_model.rb",
        "lib/GEM/internal/type/base_page.rb",
        "lib/GEM/internal/type/boolean.rb",
        "lib/GEM/internal/type/converter.rb",
        "lib/GEM/internal/type/enum.rb",
        "lib/GEM/internal/type/file_input.rb",
        "lib/GEM/internal/type/hash_of.rb",
        "lib/GEM/internal/type/union.rb",
        "lib/GEM/internal/type/unknown.rb",
        "lib/GEM/internal/util.rb",
        "rbi/GEM/errors.rbi",
        "rbi/GEM/file_part.rbi",
        "rbi/GEM/internal.rbi",
        "rbi/GEM/internal/transport/base_client.rbi",
        "rbi/GEM/internal/transport/pooled_net_requester.rbi",
        "rbi/GEM/internal/type/base_model.rbi",
        "rbi/GEM/internal/type/base_page.rbi",
        "rbi/GEM/internal/type/boolean.rbi",
        "rbi/GEM/internal/type/converter.rbi",
        "rbi/GEM/internal/type/union.rbi",
        "rbi/GEM/internal/type/unknown.rbi",
        "rbi/GEM/internal/util.rbi",
        "sig/GEM/errors.rbs",
        "sig/GEM/file_part.rbs",
        "sig/GEM/internal/transport/base_client.rbs",
        "sig/GEM/internal/transport/pooled_net_requester.rbs",
        "sig/GEM/internal/type/converter.rbs",
        "sig/GEM/internal/type/request_parameters.rbs",
        "sig/GEM/internal/type/union.rbs",
        "sig/GEM/internal/util.rbs",
        "sig/GEM/request_options.rbs",
        "test/GEM/internal/sorbet_runtime_support_test.rb",
        "test/GEM/internal/type/base_model_test.rb",
        "test/GEM/internal/util_test.rb",
    }
)

ORACLES = [
    Oracle(
        "onebusaway-ruby",
        RubyOptions(
            gem_name="onebusaway_sdk",
            module_name="OnebusawaySDK",
            array_format="repeat",
            auth_in="query",
            base_model_example=OBA_BASE_MODEL_EXAMPLE,
        ),
    ),
    Oracle(
        "imagekit-ruby",
        RubyOptions(
            gem_name="imagekitio",
            module_name="Imagekitio",
            base_model_example=IMAGEKIT_BASE_MODEL_EXAMPLE,
            enum_example=IMAGEKIT_ENUM_EXAMPLE,
            union_example=IMAGEKIT_UNION_EXAMPLE,
        ),
        # 2026-05: elides the content-type header whenever the body is nil; the 2026-07
        # generation keeps it for operations that define a body.
        frozenset({"lib/GEM/internal/transport/base_client.rb"}),
    ),
    Oracle(
        "knock-ruby",
        RubyOptions(gem_name="knockapi", module_name="Knockapi"),
        _KNOCK_2025,
        frozenset(
            f"{view}/GEM/internal/{name}.{ext}"
            for view, ext in (("lib", "rb"), ("rbi", "rbi"), ("sig", "rbs"))
            for name in (
                "entries_cursor",
                "items_cursor",
                "ms_teams_pagination",
                "slack_channels_cursor",
            )
        ),
    ),
]
# Generated per SDK, like Python's _client.py and _version.py: the client, the version, the
# require list and model aliases, and the test helper (client settings).
GENERATED = frozenset(
    {
        *(
            f"{view}/GEM/{name}.{ext}"
            for view, ext in (("lib", "rb"), ("rbi", "rbi"), ("sig", "rbs"))
            for name in ("client", "models", "version")
        ),
        "test/GEM/test_helper.rb",
    }
)


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / "Gemfile").is_file():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str) -> bool:
    path = root / rel
    return path.is_file() and path.read_bytes() == text.encode()


def _drift(oracle: Oracle) -> set[str]:
    return {oracle.path(rel) for rel in oracle.known_drift}


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_ruby_files(oracle.options)
    drift = _drift(oracle)
    mismatched = sorted(
        rel for rel, text in files.items() if rel not in drift and not _matches(root, rel, text)
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_ruby_files(oracle.options)
    stale = sorted(rel for rel in _drift(oracle) if _matches(root, rel, files[rel]))
    assert stale == []


def _on_disk(root: Path, gem: str) -> set[str]:
    found: set[str] = set()
    for view, ext in (("lib", "rb"), ("rbi", "rbi"), ("sig", "rbs")):
        base = root / view / gem
        found |= {p.relative_to(root).as_posix() for p in base.glob(f"*.{ext}")}
        found |= {p.relative_to(root).as_posix() for p in (base / "internal").rglob(f"*.{ext}")}
    tests = root / "test" / gem
    found |= {p.relative_to(root).as_posix() for p in tests.glob("*_test.rb")}
    found |= {p.relative_to(root).as_posix() for p in (tests / "internal").rglob("*.rb")}
    found.add(f"test/{gem}/test_helper.rb")
    return found


# Not runtime: the client test exercises generated resources; resource_namespaces lists them.
NOT_RUNTIME = frozenset({"test/GEM/client_test.rb"})


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    rendered = set(render_ruby_files(oracle.options))
    skipped = {oracle.path(rel) for rel in GENERATED | NOT_RUNTIME | oracle.generated}
    expected = _on_disk(root, oracle.options.gem_name) - skipped
    assert expected - rendered == set()
    assert rendered - expected <= _drift(oracle)
