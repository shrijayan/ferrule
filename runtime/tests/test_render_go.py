# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the Go runtime: options, slots and gofmt import order."""

from __future__ import annotations

import shutil
import subprocess
from typing import Any

import pytest

from ferrule_runtime import GoOptions, RenderError, render_go_files, template_root

GO = GoOptions(module_path="github.com/acme/acme-go", package_name="acme", client_name="Acme")


def test_template_root_exists() -> None:
    assert template_root("go").joinpath("internal", "apijson", "decoder.go").is_file()


def test_go_defaults() -> None:
    files = render_go_files(GO)
    assert files["field.go"].startswith("package acme\n")
    assert '"Acme/Go %s"' in files["internal/requestconfig/requestconfig.go"]
    assert "\t\tMaxRetries: 2,\n" in files["internal/requestconfig/requestconfig.go"]
    assert "made by the acme API Client" in files["option/requestoption.go"]
    assert files["option/requestoption.go"].endswith("\t})\n}\n")
    assert (
        '[]string{"authorization", "api-key", "x-api-key", "cookie", "set-cookie"}'
        in (files["option/middleware.go"])
    )
    assert not any("ferrule" in text.lower() for text in files.values())


def test_go_slots_and_options() -> None:
    files = render_go_files(
        GoOptions(
            module_path="github.com/acme/acme-go",
            package_name="acme",
            client_name="Acme",
            org_name="acme-inc",
            max_retries=4,
            redacted_headers=(),
            config_fields="\tToken          *string\n",
            config_clone="\t\tToken:          cfg.Token,\n",
            client_options="\nfunc WithToken() {}\n",
        )
    )
    config = files["internal/requestconfig/requestconfig.go"]
    assert (
        "\tMiddlewares    []middleware\n\tToken          *string\n\t// If ResponseBodyInto"
        in config
    )
    assert "\t\tMiddlewares:    cfg.Middlewares,\n\t\tToken:          cfg.Token,\n\t}\n" in config
    assert "\t\tMaxRetries: 4,\n" in config
    assert files["option/requestoption.go"].endswith(
        "\treturn nil\n\t})\n}\n\nfunc WithToken() {}\n"
    )
    assert "made by the acme-inc API Client" in files["option/requestoption.go"]
    assert '[]string{"authorization", "cookie", "set-cookie"}' in files["option/middleware.go"]


def test_go_long_client_settings_realign_their_runtime_neighbours() -> None:
    # A setting longer than the runtime's longest field widens the whole gofmt alignment run.
    files = render_go_files(
        GoOptions(
            module_path="github.com/acme/acme-go",
            package_name="acme",
            client_name="Acme",
            config_fields="\tAPIKey         *string\n\tWebhookSigningSecret *string\n",
            config_clone=(
                "\t\tAPIKey:         cfg.APIKey,\n"
                "\t\tWebhookSigningSecret: cfg.WebhookSigningSecret,\n"
            ),
        )
    )
    config = files["internal/requestconfig/requestconfig.go"]
    assert (
        "\t// WithBaseURL.\n"
        "\tDefaultBaseURL       *url.URL\n"
        "\tCustomHTTPDoer       HTTPDoer\n"
        "\tHTTPClient           *http.Client\n"
        "\tMiddlewares          []middleware\n"
        "\tAPIKey               *string\n"
        "\tWebhookSigningSecret *string\n"
        "\t// If ResponseBodyInto"
    ) in config
    # The fields before the comment are another run and keep their width.
    assert "\tMaxRetries     int\n\tRequestTimeout time.Duration\n" in config
    assert (
        "\t\tMaxRetries:           cfg.MaxRetries,\n"
        "\t\tRequestTimeout:       cfg.RequestTimeout,\n"
        "\t\tContext:              ctx,\n"
        "\t\tRequest:              req,\n"
        "\t\tBaseURL:              cfg.BaseURL,\n"
        "\t\tHTTPClient:           cfg.HTTPClient,\n"
        "\t\tMiddlewares:          cfg.Middlewares,\n"
        "\t\tAPIKey:               cfg.APIKey,\n"
        "\t\tWebhookSigningSecret: cfg.WebhookSigningSecret,\n"
        "\t}\n"
    ) in config
    gofmt = shutil.which("gofmt")
    if gofmt is not None:
        out = subprocess.run(  # noqa: S603 - gofmt from PATH on generated text
            [gofmt], input=config, capture_output=True, text=True, check=True
        )
        assert out.stdout == config


@pytest.mark.parametrize(
    ("module", "first", "second"),
    [
        # gofmt orders each import group by path.
        ("github.com/acme/acme-go", '\t"github.com/acme/acme-go/internal/param"\n', '\t"io"\n'),
        ("acme.dev/sdk", '\t"acme.dev/sdk/internal/param"\n', '\t"io"\n'),
        ("zeta.dev/sdk", '\t"io"\n', '\t"zeta.dev/sdk/internal/param"\n'),
    ],
)
def test_go_field_imports_follow_gofmt_order(module: str, first: str, second: str) -> None:
    field = render_go_files(GoOptions(module_path=module, package_name="acme", client_name="A"))
    assert f"import (\n{first}{second})\n" in field["field.go"]


def test_go_requestoption_imports_follow_gofmt_order() -> None:
    files = render_go_files(
        GoOptions(module_path="gopkg.in/acme", package_name="acme", client_name="A")
    )
    assert (
        '\t"github.com/tidwall/sjson"\n\t"gopkg.in/acme/internal/requestconfig"\n)'
        in (files["option/requestoption.go"])
    )


V2 = GoOptions(
    module_path="github.com/acme/acme-go/v3", package_name="acme", client_name="Acme", layout="v2"
)


def test_go_v2_template_root_exists() -> None:
    assert template_root("go-v2").joinpath("packages", "param", "option.go").is_file()


def test_go_v2_defaults() -> None:
    files = render_go_files(V2)
    assert "packages/respjson/respjson.go" in files
    assert "internal/param/field.go" not in files
    assert files["field.go"].startswith("package acme\n")
    assert "\nfunc File(rdr io.Reader" in files["field.go"]
    option = files["option/requestoption.go"]
    assert '"encoding/base64"' not in option
    assert option.endswith("\t})\n}\n")
    # pkg.go.dev's README link names the module without its major-version suffix.
    assert "https://pkg.go.dev/github.com/acme/acme-go#readme-requestoptions" in option
    assert '\t"github.com/acme/acme-go/v3/internal/requestconfig"\n' in option
    assert not any("ferrule" in text.lower() for text in files.values())


def test_go_v2_options() -> None:
    files = render_go_files(
        GoOptions(
            module_path="zeta.dev/sdk",
            package_name="acme",
            client_name="Acme",
            layout="v2",
            new_file=True,
            config_fields="\tToken          string\n",
            config_clone="\t\tToken:          cfg.Token,\n",
            client_options="\nvar token = base64.StdEncoding.EncodeToString(nil)\n",
        )
    )
    assert "\nfunc NewFile(rdr io.Reader" in files["field.go"]
    assert 'import (\n\t"io"\n\t"time"\n\t"zeta.dev/sdk/packages/param"\n)' in files["field.go"]
    option = files["option/requestoption.go"]
    assert '\t"bytes"\n\t"encoding/base64"\n\t"fmt"\n' in option
    assert (
        "\tMiddlewares    []middleware\n\tToken          string\n"
        in (files["internal/requestconfig/requestconfig.go"])
    )


def test_go_rejects_new_file_in_v1() -> None:
    with pytest.raises(RenderError, match="new_file"):
        render_go_files(
            GoOptions(module_path="a.dev/b", package_name="b", client_name="B", new_file=True)
        )


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"module_path": "github.com/acme/"}, "module_path"),
        ({"module_path": "github.com/a cme"}, "module_path"),
        ({"package_name": "acme-go"}, "package_name must be a Go identifier"),
        ({"package_name": "func"}, "package_name"),
        ({"client_name": "A/B"}, "client_name"),
        ({"max_retries": -1}, "max_retries"),
        ({"config_fields": "\tToken *string"}, "config_fields must be empty or end with"),
        ({"redacted_headers": ("x key",)}, "lower-case header names"),
        ({"layout": "v3"}, "layout"),
    ],
)
def test_go_rejects_bad_options(kwargs: dict[str, Any], match: str) -> None:
    options: dict[str, Any] = {
        "module_path": GO.module_path,
        "package_name": GO.package_name,
        "client_name": GO.client_name,
        **kwargs,
    }
    with pytest.raises(RenderError, match=match):
        render_go_files(GoOptions(**options))
