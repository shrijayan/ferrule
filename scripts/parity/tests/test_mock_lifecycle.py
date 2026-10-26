# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""The mock server's port and shutdown: a busy port must never be mistaken for our mock."""

from __future__ import annotations

import socket
import subprocess
import sys
import time
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


def free_port() -> int:
    with socket.socket() as s:
        s.bind((MOCK_HOST, 0))
        return int(s.getsockname()[1])


def test_one_run_owns_a_port_at_a_time() -> None:
    port = free_port()
    with run.own_port(port, timeout=5) as first:
        assert first
        with run.own_port(port, timeout=0.2) as second:
            assert not second
    with run.own_port(port, timeout=0.2) as after:
        assert after


def test_run_tests_refuses_a_port_another_run_owns(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    (tmp_path / "sdk" / "tests").mkdir(parents=True)
    port = free_port()
    monkeypatch.setattr(run, "MOCK_PORT", port)
    monkeypatch.setattr(run, "MOCK_LOCK_TIMEOUT_S", 0.2)
    with run.own_port(port, timeout=5):
        result = run.run_tests(
            tmp_path / "out", tmp_path / "sdk", tmp_path / "spec.yml", tmp_path / "work", "x"
        )
    assert result["status"] == "mock-port-busy"
    assert "FER4021" in str(result["error"])


def test_launcher_exited_does_not_reap() -> None:
    proc = subprocess.Popen([sys.executable, "-c", "pass"], start_new_session=True)
    deadline = time.monotonic() + 30
    while not run.launcher_exited(proc):
        assert time.monotonic() < deadline
        time.sleep(0.05)
    assert proc.returncode is None
    proc.wait(timeout=30)


def test_stop_mock_stops_a_listener_that_outlives_its_launcher() -> None:
    port = free_port()
    listener = (
        "import socket, time\n"
        "s = socket.socket()\n"
        "s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\n"
        f"s.bind(({MOCK_HOST!r}, {port}))\n"
        "s.listen()\n"
        "time.sleep(300)\n"
    )
    launcher = f"import subprocess, sys; subprocess.Popen([sys.executable, '-c', {listener!r}])"
    proc = subprocess.Popen([sys.executable, "-c", launcher], start_new_session=True)  # noqa: S603
    assert run.wait_for_port(port, proc, timeout=30) or run.port_in_use(port)
    deadline = time.monotonic() + 30
    while not run.launcher_exited(proc):
        assert time.monotonic() < deadline
        time.sleep(0.05)
    run.stop_mock(proc, port)
    assert not run.port_in_use(port)
