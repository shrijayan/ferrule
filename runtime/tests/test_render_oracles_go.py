# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The rendered Go runtime is byte-identical to the pinned oracle SDKs' files, in both layouts.

Oracles are checkouts at ``oracles/cache/<name>`` (pins in ``runtime/README.md``); a test skips
when its checkout is absent. Only OneBusAway/go-sdk (v1) and imagekit-go (v2), both Apache-2.0,
feed the templates; opencode-sdk-go and ogx-client-go (MIT) are compared, never copied.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

from ferrule_runtime import GoOptions, render_go_files

CACHE = Path(__file__).resolve().parents[2] / "oracles" / "cache"

# What the plugin generates into the slots for OneBusAway: the `api_key` client setting (sent
# as the `key` query parameter) and the production environment.
OBA_CLIENT_OPTIONS = """
// WithEnvironmentProduction returns a RequestOption that sets the current
// environment to be the "production" environment. An environment specifies which base URL
// to use by default.
func WithEnvironmentProduction() RequestOption {
\treturn requestconfig.WithDefaultBaseURL("https://api.pugetsound.onebusaway.org/")
}

// WithAPIKey returns a RequestOption that sets the client setting "api_key".
func WithAPIKey(value string) RequestOption {
\treturn requestconfig.PreRequestOptionFunc(func(r *requestconfig.RequestConfig) error {
\t\tif value == "" {
\t\t\treturn fmt.Errorf("default param cannot be empty string")
\t\t}
\t\tr.APIKey = &value
\t\treturn r.Apply(WithQuery("key", *r.APIKey))
\t})
}
"""

# What the plugin generates into the v2 slots: imagekit's HTTP Basic pair and webhook secret
# (`encoding/base64` is then imported), knock-mgmt's bearer token, ogx's API key.
_BASIC = (
    'fmt.Sprintf("Basic %s", '
    'base64.StdEncoding.EncodeToString([]byte(r.PrivateKey+":"+r.Password)))'
)
IMAGEKIT_CLIENT_OPTIONS = """
// WithEnvironmentProduction returns a RequestOption that sets the current
// environment to be the "production" environment. An environment specifies which base URL
// to use by default.
func WithEnvironmentProduction() RequestOption {
\treturn requestconfig.WithDefaultBaseURL("https://api.imagekit.io/")
}

// WithPrivateKey returns a RequestOption that sets the client setting "private_key".
func WithPrivateKey(value string) RequestOption {
\treturn requestconfig.RequestOptionFunc(func(r *requestconfig.RequestConfig) error {
\t\tr.PrivateKey = value
\t\treturn r.Apply(WithHeader("authorization", {BASIC}))
\t})
}

// WithPassword returns a RequestOption that sets the client setting "password".
func WithPassword(value string) RequestOption {
\treturn requestconfig.PreRequestOptionFunc(func(r *requestconfig.RequestConfig) error {
\t\tr.Password = value
\t\treturn r.Apply(WithHeader("authorization", {BASIC}))
\t})
}

// WithWebhookSecret returns a RequestOption that sets the client setting "webhook_secret".
func WithWebhookSecret(value string) RequestOption {
\treturn requestconfig.PreRequestOptionFunc(func(r *requestconfig.RequestConfig) error {
\t\tr.WebhookSecret = value
\t\treturn nil
\t})
}
""".replace("{BASIC}", _BASIC)


def _bearer_options(url: str, field: str, setting: str) -> str:
    return f"""
// WithEnvironmentProduction returns a RequestOption that sets the current
// environment to be the "production" environment. An environment specifies which base URL
// to use by default.
func WithEnvironmentProduction() RequestOption {{
\treturn requestconfig.WithDefaultBaseURL("{url}")
}}

// With{field} returns a RequestOption that sets the client setting "{setting}".
func With{field}(value string) RequestOption {{
\treturn requestconfig.RequestOptionFunc(func(r *requestconfig.RequestConfig) error {{
\t\tr.{field} = value
\t\treturn r.Apply(WithHeader("authorization", fmt.Sprintf("Bearer %s", r.{field})))
\t}})
}}
"""


# ogx-client-go prepends its `settings.file_header` to the files that carry Stainless's header.
OGX_HEADER = """// Copyright (c) The OGX Contributors.
// All rights reserved.
//
// This source code is licensed under the terms described in the LICENSE file in
// the root directory of this source tree.
//
"""

# Older Stainless Go v1 generations (2025): same files, older text. Slots are left empty for
# them because the files holding slots differ anyway.
_GO_2025 = frozenset(
    {
        "default_http_client.go",  # absent: added by later generations
        "internal/apiform/encoder.go",
        "internal/apiform/form_test.go",
        "internal/apiform/tag.go",
        "internal/apijson/decoder.go",
        "internal/apijson/encoder.go",
        "internal/apijson/json_test.go",
        "internal/apijson/tag.go",
        "internal/apiquery/encoder.go",
        "internal/apiquery/tag.go",
        "internal/requestconfig/requestconfig.go",
        "internal/testutil/testutil.go",
        "option/middleware.go",  # knock-go: absent; opencode-sdk-go: no header redaction yet
        "option/requestoption.go",
    }
)


@dataclass(frozen=True)
class Oracle:
    name: str
    options: GoOptions
    known_drift: frozenset[str] = frozenset()
    """Files whose text comes from another Stainless generation (runtime/README.md)."""
    generated: frozenset[str] = frozenset()
    """Runtime-adjacent files the plugin generates (pagination, streaming, constants)."""
    header: str = ""
    """A file header the oracle's config adds in front of Stainless's (compared without it)."""


ORACLES = [
    Oracle(
        "onebusaway-go",
        GoOptions(
            module_path="github.com/OneBusAway/go-sdk",
            package_name="onebusaway",
            client_name="OnebusawaySDK",
            org_name="onebusaway-sdk",
            config_fields="\tAPIKey         *string\n",
            config_clone="\t\tAPIKey:         cfg.APIKey,\n",
            client_options=OBA_CLIENT_OPTIONS,
        ),
    ),
    Oracle(
        "knock-go",
        GoOptions(
            module_path="github.com/knocklabs/knock-go", package_name="knock", client_name="Knock"
        ),
        _GO_2025,
        frozenset({"packages/pagination/pagination.go"}),
    ),
    Oracle(
        "opencode-go",
        GoOptions(
            module_path="github.com/sst/opencode-sdk-go",
            package_name="opencode",
            client_name="Opencode",
        ),
        _GO_2025,
        # SSE streaming: no Apache-2.0 Go v1 oracle ships it, so it is not templated.
        frozenset({"packages/ssestream/ssestream.go"}),
    ),
    Oracle(
        "imagekit-go",
        GoOptions(
            module_path="github.com/imagekit-developer/imagekit-go/v2",
            package_name="imagekit",
            client_name="ImageKit",
            org_name="ImageKit",
            layout="v2",
            new_file=True,
            config_fields=(
                "\tPrivateKey     string\n\tPassword       string\n\tWebhookSecret  string\n"
            ),
            config_clone=(
                "\t\tPrivateKey:     cfg.PrivateKey,\n\t\tPassword:       cfg.Password,\n"
                "\t\tWebhookSecret:  cfg.WebhookSecret,\n"
            ),
            client_options=IMAGEKIT_CLIENT_OPTIONS,
        ),
        generated=frozenset({"shared/constant/constants.go"}),
    ),
    Oracle(
        "knock-mgmt-go",
        GoOptions(
            module_path="github.com/knocklabs/knock-mgmt-go",
            package_name="knockmapi",
            client_name="KnockMgmt",
            org_name="knock mgmt",
            layout="v2",
            config_fields="\tServiceToken   string\n",
            config_clone="\t\tServiceToken:   cfg.ServiceToken,\n",
            client_options=_bearer_options(
                "https://control.knock.app/", "ServiceToken", "service_token"
            ),
        ),
        generated=frozenset({"packages/pagination/pagination.go", "shared/constant/constants.go"}),
    ),
    Oracle(
        "ogx-go",
        GoOptions(
            module_path="github.com/ogx-ai/ogx-client-go",
            package_name="ogxclient",
            client_name="OgxClient",
            org_name="ogx-client",
            layout="v2",
            new_file=True,
            config_fields="\tAPIKey         string\n",
            config_clone="\t\tAPIKey:         cfg.APIKey,\n",
            client_options=_bearer_options("http://any-hosted-ogx.com/", "APIKey", "api_key"),
        ),
        generated=frozenset(
            {
                "packages/pagination/pagination.go",
                "packages/ssestream/ssestream.go",
                "shared/constant/constants.go",
            }
        ),
        header=OGX_HEADER,
    ),
]
RUNTIME_DIRS = ("internal", "option", "packages", "shared/constant")
RUNTIME_ROOT_FILES = ("field.go", "default_http_client.go")
GENERATED = frozenset({"internal/version.go"})


def _root(oracle: Oracle) -> Path:
    root = CACHE / oracle.name
    if not (root / "go.mod").is_file():
        pytest.skip(f"oracle checkout absent: oracles/cache/{oracle.name}")
    return root


def _matches(root: Path, rel: str, text: str, header: str = "") -> bool:
    path = root / rel
    if not path.is_file():
        return False
    data = path.read_bytes()
    return data.removeprefix(header.encode()) == text.encode()


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_matches_oracle_bytes(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_go_files(oracle.options)
    mismatched = sorted(
        rel
        for rel, text in files.items()
        if rel not in oracle.known_drift and not _matches(root, rel, text, oracle.header)
    )
    assert mismatched == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_known_drift_is_still_drift(oracle: Oracle) -> None:
    root = _root(oracle)
    files = render_go_files(oracle.options)
    stale = sorted(
        rel for rel in oracle.known_drift if _matches(root, rel, files[rel], oracle.header)
    )
    assert stale == []


@pytest.mark.parametrize("oracle", ORACLES, ids=lambda o: o.name)
def test_render_covers_every_runtime_file(oracle: Oracle) -> None:
    root = _root(oracle)
    on_disk = {
        p.relative_to(root).as_posix()
        for top in RUNTIME_DIRS
        for p in (root / top).rglob("*")
        if p.is_file()
    } | {rel for rel in RUNTIME_ROOT_FILES if (root / rel).is_file()}
    rendered = set(render_go_files(oracle.options))
    expected = on_disk - GENERATED - oracle.generated
    assert expected - rendered == set()
    assert rendered - expected <= oracle.known_drift
