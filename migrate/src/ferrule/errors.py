# Copyright 2026 Ferrule contributors
# SPDX-License-Identifier: Apache-2.0
"""Diagnostics for the ``ferrule`` CLI: every error carries a FER code, a pointer and a fix.

Codes in the FER4xxx range belong to migrate/ (generate, verify, adopt, rescue, merge).
"""

from __future__ import annotations

from dataclasses import dataclass

MISSING_INPUT = "FER4001"
TOOL_NOT_FOUND = "FER4002"
STEP_FAILED = "FER4003"
NO_RUNTIME = "FER4004"
NO_PACKAGE = "FER4005"
OUT_NOT_EMPTY = "FER4006"
SYMLINK_REFUSED = "FER4007"
UNSAFE_PATH = "FER4008"
LOAD_FAILED = "FER4010"
BAD_REF = "FER4011"


@dataclass(frozen=True)
class Diagnostic:
    """One addressed problem: what went wrong, where, and how to fix it."""

    code: str
    message: str
    pointer: str
    fix: str

    def render(self) -> str:
        """Format the diagnostic the way every Ferrule tool prints errors."""
        return f"{self.code} at {self.pointer}: {self.message}. Fix: {self.fix}"


class FerruleError(Exception):
    """An error that stops a command; carries its diagnostic."""

    def __init__(self, diagnostic: Diagnostic) -> None:
        """Wrap ``diagnostic``."""
        super().__init__(diagnostic.render())
        self.diagnostic = diagnostic


def fail(code: str, message: str, pointer: str, fix: str) -> FerruleError:
    """Return a :class:`FerruleError` built from its diagnostic fields, ready to raise."""
    return FerruleError(Diagnostic(code, message, pointer, fix))
