# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union
from typing_extensions import Literal, TypeAlias

from .._models import BaseModel

__all__ = ["CompactedResponse", "Output", "OutputMessageOutput", "OutputFunctionToolCall", "OutputCompaction"]


class OutputMessageOutput(BaseModel):
    id: str

    type: Literal["message"]


class OutputFunctionToolCall(BaseModel):
    name: str

    type: Literal["function_call"]


class OutputCompaction(BaseModel):
    encrypted_content: str

    type: Literal["compaction"]


Output: TypeAlias = Union[OutputMessageOutput, OutputFunctionToolCall, OutputCompaction]


class CompactedResponse(BaseModel):
    id: str

    output: List[Output]
