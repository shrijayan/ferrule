# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import functools
import hashlib
from pathlib import Path

import pytest

from ferrule import cli
from ferrule.errors import FerruleError
from ferrule.rescue import (
    MAX_SPEC_BYTES,
    Stats,
    check_url,
    parse_stats,
    rescue,
    write_spec,
)

SPEC = b'{"openapi": "3.1.0"}\n'
MD5 = hashlib.md5(SPEC, usedforsecurity=False).hexdigest()
URL = "https://storage.example.com/acme%2Facme-0123.json"


def stats(url: str = URL, digest: str = MD5) -> str:
    return f"configured_endpoints: 3\nopenapi_spec_url: {url}\nopenapi_spec_hash: '{digest}'\n"


def make_repo(tmp_path: Path, text: str) -> Path:
    (tmp_path / ".stats.yml").write_text(text)
    return tmp_path


def fetcher(data: bytes, calls: list[str] | None = None):  # noqa: ANN201
    def fetch(url: str, limit: int, timeout: float) -> bytes:
        assert limit == MAX_SPEC_BYTES
        assert timeout > 0
        if calls is not None:
            calls.append(url)
        return data

    return fetch


def test_parse_stats() -> None:
    parsed = parse_stats(stats() + "config_hash: abc # trailing\n")
    assert parsed == Stats(URL, MD5)
    assert parsed.suffix == ".json"
    assert Stats("https://x.dev/spec.yaml", MD5).suffix == ".yaml"
    assert Stats("https://x.dev/spec.YML", MD5).suffix == ".yml"
    assert Stats("https://x.dev/spec", MD5).suffix == ".yml"


@pytest.mark.parametrize(
    ("text", "needle"),
    [
        ("openapi_spec_hash: " + MD5 + "\n", "openapi_spec_url is missing"),
        (stats(digest="nothex"), "not an md5"),
        (stats(url="http://storage.example.com/spec.json"), "only https"),
        (stats(url="https://user:pw@storage.example.com/s.json"), "only https"),
        (stats(url="file:///etc/passwd"), "only https"),
    ],
)
def test_parse_stats_errors(text: str, needle: str) -> None:
    with pytest.raises(FerruleError, match=needle):
        parse_stats(text)


def test_check_url_accepts_https() -> None:
    check_url(URL)


def test_rescue_good_md5(tmp_path: Path) -> None:
    calls: list[str] = []
    rescued = rescue(make_repo(tmp_path, stats()), fetcher(SPEC, calls))
    assert rescued.data == SPEC
    assert calls == [URL]
    out = write_spec(rescued, tmp_path / "openapi.json")
    assert out.read_bytes() == SPEC


def test_rescue_bad_md5(tmp_path: Path) -> None:
    with pytest.raises(FerruleError) as exc:
        rescue(make_repo(tmp_path, stats()), fetcher(b"tampered"))
    assert exc.value.diagnostic.code == "FER4101"


def test_rescue_refuses_http_before_fetching(tmp_path: Path) -> None:
    calls: list[str] = []
    repo = make_repo(tmp_path, stats(url="http://storage.example.com/s.json"))
    with pytest.raises(FerruleError, match="only https"):
        rescue(repo, fetcher(SPEC, calls))
    assert calls == []


def test_rescue_size_cap(tmp_path: Path) -> None:
    def fetch(url: str, limit: int, timeout: float) -> bytes:
        del url, timeout
        return b"x" * (limit + 1)

    with pytest.raises(FerruleError, match="larger than 4 bytes") as exc:
        rescue(make_repo(tmp_path, stats()), fetch, limit=4)
    assert exc.value.diagnostic.code == "FER4104"


def test_rescue_fetch_error(tmp_path: Path) -> None:
    def fetch(url: str, limit: int, timeout: float) -> bytes:
        del url, limit, timeout
        raise TimeoutError

    with pytest.raises(FerruleError, match="download failed"):
        rescue(make_repo(tmp_path, stats()), fetch)


def test_rescue_missing_stats(tmp_path: Path) -> None:
    with pytest.raises(FerruleError, match="does not exist"):
        rescue(tmp_path, fetcher(SPEC))


def test_rescue_refuses_symlinked_stats(tmp_path: Path) -> None:
    (tmp_path / "real.yml").write_text(stats())
    (tmp_path / ".stats.yml").symlink_to(tmp_path / "real.yml")
    with pytest.raises(FerruleError) as exc:
        rescue(tmp_path, fetcher(SPEC))
    assert exc.value.diagnostic.code == "FER4007"


def test_write_spec_refuses_symlink(tmp_path: Path) -> None:
    rescued = rescue(make_repo(tmp_path, stats()), fetcher(SPEC))
    (tmp_path / "link.json").symlink_to(tmp_path / "elsewhere.json")
    with pytest.raises(FerruleError, match="symlink"):
        write_spec(rescued, tmp_path / "link.json")


def test_cli_rescue(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    repo = make_repo(tmp_path, stats())
    monkeypatch.setattr(cli, "rescue", functools.partial(rescue, fetch=fetcher(SPEC)))
    assert cli.main(["rescue", str(repo)]) == 0
    assert (repo / "openapi.json").read_bytes() == SPEC
    assert MD5 in capsys.readouterr().out
    assert cli.main(["rescue", str(repo), "--out", str(tmp_path / "spec.json")]) == 0
    assert (tmp_path / "spec.json").read_bytes() == SPEC


def test_cli_rescue_error(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    assert cli.main(["rescue", str(tmp_path)]) == 1
    assert "FER4001" in capsys.readouterr().err
