# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from .._models import BaseModel

__all__ = ["WorkflowActivateResponse"]


class WorkflowActivateResponse(BaseModel):
    workflow: "Workflow"
    """A workflow."""


from .workflow import Workflow
