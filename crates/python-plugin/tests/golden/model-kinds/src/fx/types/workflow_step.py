# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from typing import TYPE_CHECKING, Union
from typing_extensions import TypeAlias, TypeAliasType

from .._compat import PYDANTIC_V1
from .workflow_delay_step import WorkflowDelayStep

__all__ = ["WorkflowStep"]

if TYPE_CHECKING or not PYDANTIC_V1:
    WorkflowStep = TypeAliasType("WorkflowStep", Union[WorkflowDelayStep, "WorkflowBranchStep"])
else:
    WorkflowStep: TypeAlias = Union[WorkflowDelayStep, "WorkflowBranchStep"]

from .workflow_branch_step import WorkflowBranchStep
