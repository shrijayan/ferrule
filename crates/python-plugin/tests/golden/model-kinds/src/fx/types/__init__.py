# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from __future__ import annotations

from . import workflow, workflow_branch_step, workflow_activate_response
from .. import _compat
from .shared import Extensions as Extensions, StreamingResolution as StreamingResolution
from .source import Source as Source
from .audience import Audience as Audience
from .workflow import Workflow as Workflow
from .channel_data import ChannelData as ChannelData
from .workflow_step import WorkflowStep as WorkflowStep
from .condition_group import ConditionGroup as ConditionGroup
from .static_audience import StaticAudience as StaticAudience
from .dynamic_audience import DynamicAudience as DynamicAudience
from .workflow_delay_step import WorkflowDelayStep as WorkflowDelayStep
from .workflow_branch_step import WorkflowBranchStep as WorkflowBranchStep
from .workflow_activate_response import WorkflowActivateResponse as WorkflowActivateResponse
from .workflow_list_audiences_response import WorkflowListAudiencesResponse as WorkflowListAudiencesResponse

# Rebuild cyclical models only after all modules are imported.
# This ensures that, when building the deferred (due to cyclical references) model schema,
# Pydantic can resolve the necessary references.
# See: https://github.com/pydantic/pydantic/issues/11250 for more context.
if _compat.PYDANTIC_V1:
    workflow.Workflow.update_forward_refs()  # type: ignore
    workflow_branch_step.WorkflowBranchStep.update_forward_refs()  # type: ignore
    workflow_activate_response.WorkflowActivateResponse.update_forward_refs()  # type: ignore
else:
    workflow.Workflow.model_rebuild(_parent_namespace_depth=0)
    workflow_branch_step.WorkflowBranchStep.model_rebuild(_parent_namespace_depth=0)
    workflow_activate_response.WorkflowActivateResponse.model_rebuild(_parent_namespace_depth=0)
