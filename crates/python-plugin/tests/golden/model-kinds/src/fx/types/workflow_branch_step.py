# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import List, Optional
from typing_extensions import Literal

from .._models import BaseModel

__all__ = ["WorkflowBranchStep", "Case"]


class Case(BaseModel):
    steps: List["WorkflowStep"]

    name: Optional[str] = None


class WorkflowBranchStep(BaseModel):
    cases: List[Case]

    type: Literal["branch"]


from .workflow_step import WorkflowStep
