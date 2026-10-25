# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The mock server's port and shutdown: a busy port must never be mistaken for our mock."""

from __future__ import annotations

import socket
import subprocess
import sys
from typing import TYPE_CHECKING

from scripts.parity import run
from scripts.parity.harness import MOCK_HOST

if TYPE_CHECKING:
    from pathlib import Path

    import pytest


def test_port_in_use() -> None:
    with socket.socket() as s:
        s.bind((MOCK_HOST, 0))
        s.listen()
        port = s.getsockname()[1]
        assert run.port_in_use(port)
    assert not run.port_in_use(port)


def test_stop_mock_tolerates_an_exited_process() -> None:
    proc = subprocess.Popen([sys.executable, "-c", "pass"], start_new_session=True)
    proc.wait(timeout=30)
    run.stop_mock(proc)


def test_run_tests_refuses_a_busy_port(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (tmp_path / "sdk" / "tests").mkdir(parents=True)
    with socket.socket() as s:
        s.bind((MOCK_HOST, 0))
        s.listen()
        monkeypatch.setattr(run, "MOCK_PORT", s.getsockname()[1])
        result = run.run_tests(
            tmp_path / "out", tmp_path / "sdk", tmp_path / "spec.yml", tmp_path / "work", "x"
        )
    assert result["status"] == "mock-port-busy"
    assert "FER4021" in str(result["error"])
