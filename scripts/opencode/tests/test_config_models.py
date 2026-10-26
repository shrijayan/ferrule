# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
from __future__ import annotations

import json
from typing import TYPE_CHECKING

import pytest

from scripts.opencode.config_models import (
    Json,
    ModelsError,
    apply,
    casings,
    main,
    model_entries,
    python_name,
)

if TYPE_CHECKING:
    from pathlib import Path

SPEC: Json = {
    "components": {
        "schemas": {
            "Agent.Info": {"type": "object"},
            "Config.LSP.ServerEncoded": {"type": "object"},
            "Money.USD": {"type": "number"},
            "Session.NotFoundErrorEncoded": {"type": "object"},
            "Session.Inbox.MovePayload_1": {"type": "object"},
            "V2Event": {"anyOf": []},
            "V2Event.Server.Connected": {"type": "object"},
            "session.created": {"type": "object"},
            "VoiceWebRTCTransport": {"type": "object"},
        }
    }
}
CONFIG = """\
organization:
  name: opencode

resources:
  agent:
    methods:
      get: get /api/agent/{agentID}

readme:
  example_requests: {}
"""


@pytest.mark.parametrize(
    ("key", "name"),
    [
        ("Agent.Info", "AgentInfo"),
        ("Config.InfoEncoded", "ConfigInfo"),
        ("Session.NotFoundErrorEncoded", "SessionNotFoundErrorBody"),
        ("Session.Inbox.MovePayload_1", "SessionInboxMovePayload1"),
        ("V2Event", "Event"),
        ("V2Event.Server.Connected", "ServerConnectedEvent"),
        ("session.created", "SessionCreatedEvent"),
    ],
)
def test_python_name(key: str, name: str) -> None:
    assert python_name(key) == name


def test_model_entries() -> None:
    assert model_entries(SPEC) == [
        ("agent_info", "Agent.Info"),
        ("config_lsp_server", "Config.LSP.ServerEncoded"),
        ("event", "V2Event"),
        ("money_usd", "Money.USD"),
        ("server_connected_event", "V2Event.Server.Connected"),
        ("session_created_event", "session.created"),
        ("session_inbox_move_payload1", "Session.Inbox.MovePayload_1"),
        ("session_not_found_error_body", "Session.NotFoundErrorEncoded"),
        ("voice_web_rtc_transport", "VoiceWebRTCTransport"),
    ]


def test_model_entries_refuse_one_name_for_two_schemas() -> None:
    spec: Json = {"components": {"schemas": {"A.B": {}, "AB": {}}}}
    with pytest.raises(ModelsError, match="both named ab"):
        model_entries(spec)


def test_casings_spell_the_keys_as_the_sdk() -> None:
    # `id` is upper-case by default; `lsp`, `usd` and `rtc` are not.
    assert casings(SPEC) == {
        "lsp": {"initialism": True},
        "rtc": {"initialism": True},
        "usd": {"initialism": True},
    }
    spec: Json = {"components": {"schemas": {"Api.Key": {}, "ApiKeyId": {}}}}
    assert casings(spec) == {"api": {"initialism": False}, "id": {"initialism": False}}


def test_casings_refuse_two_spellings() -> None:
    spec: Json = {"components": {"schemas": {"Lsp.Client": {}, "LSP.Server": {}}}}
    with pytest.raises(ModelsError, match="spelled both"):
        casings(spec)


def test_apply_inserts_then_replaces_the_blocks() -> None:
    once = apply(CONFIG, SPEC)
    assert once.splitlines()[3:9] == [
        "resources:",
        "  # <ferrule:models> written by scripts/opencode/config_models.py",
        "  $shared:",
        "    models:",
        '      agent_info: "#/components/schemas/Agent.Info"',
        '      config_lsp_server: "#/components/schemas/Config.LSP.ServerEncoded"',
    ]
    assert "custom_casings:\n  lsp: {initialism: true}\n" in once
    assert once.index("# </ferrule:casings>") < once.index("\nreadme:\n")
    assert "  agent:\n    methods:\n" in once
    smaller: Json = {"components": {"schemas": {"Agent.Info": {}}}}
    twice = apply(once, smaller)
    assert apply(twice, smaller) == twice
    assert "money_usd" not in twice
    assert twice.count("<ferrule:models>") == 1


def test_apply_errors() -> None:
    with pytest.raises(ModelsError, match="no 'resources:'"):
        apply("readme: {}\n", SPEC)
    broken = apply(CONFIG, SPEC).replace("  # </ferrule:models>", "")
    with pytest.raises(ModelsError, match="has no"):
        apply(broken, SPEC)


def test_main_rewrites_the_config(tmp_path: Path) -> None:
    spec, config = tmp_path / "openapi.json", tmp_path / "stainless.yml"
    spec.write_text(json.dumps(SPEC))
    config.write_text(CONFIG)
    assert main([str(spec), str(config)]) == 0
    assert config.read_text() == apply(CONFIG, SPEC)


def test_apply_unwraps_the_operations_the_contract_marks() -> None:
    spec: Json = {
        "paths": {
            "/api/agent/{agentID}": {"get": {"x-opencode-unwrap-data": True}},
            "/api/agent": {"get": {}},
        },
        "components": {"schemas": {}},
    }
    config = CONFIG.replace(
        "      get: get /api/agent/{agentID}\n",
        "      get: get /api/agent/{agentID}\n      list: get /api/agent\n",
    )
    once = apply(config, spec)
    assert (
        "      get:\n        endpoint: get /api/agent/{agentID}\n        unwrap_response: data\n"
        in once
    )
    assert "      list: get /api/agent\n" in once
    assert apply(once, spec) == once


def test_model_entries_leave_out_the_envelopes_the_sdk_unwraps() -> None:
    # The SDK returns GenerateTextResponse's `data`, so the envelope is no model of its own (its
    # name would clash with the unwrapped type's); Shared is also used by another schema.
    def returns(schema: str) -> Json:
        content = {"application/json": {"schema": {"$ref": f"#/components/schemas/{schema}"}}}
        return {"x-opencode-unwrap-data": True, "responses": {"200": {"content": content}}}

    spec: Json = {
        "paths": {
            "/api/generate": {"post": returns("GenerateTextResponse")},
            "/api/shared": {"get": returns("Shared")},
        },
        "components": {
            "schemas": {
                "GenerateTextResponse": {"type": "object"},
                "Shared": {"type": "object"},
                "Holder": {"properties": {"s": {"$ref": "#/components/schemas/Shared"}}},
            }
        },
    }
    assert model_entries(spec) == [("holder", "Holder"), ("shared", "Shared")]
