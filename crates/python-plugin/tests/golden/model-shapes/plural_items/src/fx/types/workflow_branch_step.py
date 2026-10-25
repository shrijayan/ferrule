# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import Dict, List, Optional

from .._models import BaseModel

__all__ = ["WorkflowBranchStep", "Branch", "ExamplePayload"]


class Branch(BaseModel):
    name: Optional[str] = None
    """The name of the branch."""


class ExamplePayload(BaseModel):
    body: Optional[Dict[str, object]] = None
    """Example payload body."""


class WorkflowBranchStep(BaseModel):
    """A branch function step."""

    branches: List[Branch]
    """A list of workflow branches to be evaluated."""

    example_payloads: Optional[Dict[str, List[ExamplePayload]]] = None
    """Example payloads keyed by event type."""
