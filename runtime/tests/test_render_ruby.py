# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Rendering the Ruby runtime: names, auth placement, array format, slots and line wrapping."""

from __future__ import annotations

from typing import Any

import pytest

from ferrule_runtime import RenderError, RubyOptions, render_ruby_files, template_root

ACME = RubyOptions(gem_name="acme", module_name="Acme")
BASE_CLIENT = "lib/acme/internal/transport/base_client.rb"


def test_template_root_exists() -> None:
    assert template_root("ruby").joinpath("lib", "__ferrule_gem__", "internal.rb").is_file()


def test_ruby_defaults() -> None:
    files = render_ruby_files(ACME)
    assert {"lib/acme/errors.rb", "rbi/acme/errors.rbi", "sig/acme/errors.rbs"} <= set(files)
    assert files["lib/acme/internal.rb"].startswith(
        "# frozen_string_literal: true\n\nmodule Acme\n"
    )
    client = files[BASE_CLIENT]
    assert "private def auth_headers = {}" in client
    assert "auth_query" not in client
    assert "private def auth_headers: -> " in files["sig/acme/internal/transport/base_client.rbs"]
    assert 'element.map(&:to_s).join(",")' in files["lib/acme/internal/util.rb"]
    assert "# @example" not in files["lib/acme/internal/type/enum.rb"]
    assert not any("ferrule" in rel.lower() or "ferrule" in t.lower() for rel, t in files.items())


def test_query_auth_and_repeated_arrays() -> None:
    options = RubyOptions(
        gem_name="acme", module_name="Acme", auth_in="query", array_format="repeat"
    )
    files = render_ruby_files(options)
    client = files[BASE_CLIENT]
    assert "private def auth_query = {}" in client
    assert (
        "query = Acme::Internal::Util.deep_merge(auth_query, req[:query].to_h, "
        "opts[:extra_query].to_h)\n" in client
    )
    assert "            auth_headers,\n" not in client
    assert "collection[key] = element.map(&:to_s)\n" in files["lib/acme/internal/util.rb"]


def test_slots() -> None:
    example = "      #\n      # @example\n      #   Acme::Color::RED\n"
    files = render_ruby_files(
        RubyOptions(gem_name="acme", module_name="Acme", enum_example=example)
    )
    assert (
        f"values safely.\n{example}      module Enum\n" in files["lib/acme/internal/type/enum.rb"]
    )


def test_wrapping_follows_the_module_name_length() -> None:
    short = render_ruby_files(ACME)["sig/acme/internal.rbs"]
    assert "    type file_input = Pathname | StringIO | IO | String | Acme::FilePart\n" in short
    name = "AcmeLongerModule"
    long = render_ruby_files(RubyOptions(gem_name="acme", module_name=name, auth_in="query"))
    assert (
        f"    type file_input =\n      Pathname | StringIO | IO | String | {name}::FilePart\n"
        in long["sig/acme/internal.rbs"]
    )
    assert f"query = {name}::Internal::Util.deep_merge(\n" in long[BASE_CLIENT]


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"gem_name": "Acme"}, "gem_name"),
        ({"gem_name": "acme-sdk"}, "gem_name"),
        ({"module_name": "acme"}, "module_name"),
        ({"module_name": "Acme::SDK"}, "module_name"),
        ({"enum_example": "# @example"}, "enum_example must be empty or end with"),
    ],
)
def test_rejects_bad_options(kwargs: dict[str, Any], match: str) -> None:
    options: dict[str, Any] = {"gem_name": "acme", "module_name": "Acme", **kwargs}
    with pytest.raises(RenderError, match=match):
        render_ruby_files(RubyOptions(**options))
