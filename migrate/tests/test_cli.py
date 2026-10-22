# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
import subprocess
import sys

import pytest

from ferrule import __version__
from ferrule.cli import SUBCOMMANDS, main


@pytest.mark.parametrize("command", sorted(SUBCOMMANDS))
def test_unimplemented(command: str, capsys: pytest.CaptureFixture[str]) -> None:
    assert main([command]) == 2
    assert "FER0001 not implemented in this version" in capsys.readouterr().err


def test_version(capsys: pytest.CaptureFixture[str]) -> None:
    with pytest.raises(SystemExit) as exc:
        main(["--version"])
    assert exc.value.code == 0
    assert __version__ in capsys.readouterr().out


def test_no_command() -> None:
    with pytest.raises(SystemExit) as exc:
        main([])
    assert exc.value.code == 2


def test_module_entry() -> None:
    proc = subprocess.run(
        [sys.executable, "-m", "ferrule", "verify"],
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 2
    assert "FER0001" in proc.stderr
