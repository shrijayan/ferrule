# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union, Optional
from typing_extensions import Literal, TypeAlias

from .._models import BaseModel

__all__ = [
    "EmailTemplate",
    "VisualBlock",
    "VisualBlockEmailDividerBlock",
    "VisualBlockEmailHTMLBlock",
    "VisualBlockEmailHTMLBlockLayoutAttrs",
]


class VisualBlockEmailDividerBlock(BaseModel):
    """A divider block in an email template."""

    type: Literal["divider"]
    """The type of the block."""


class VisualBlockEmailHTMLBlockLayoutAttrs(BaseModel):
    """The layout attributes of the block."""

    padding_top: int
    """The padding_top layout attribute of the block."""


class VisualBlockEmailHTMLBlock(BaseModel):
    """An HTML block in an email template."""

    content: str
    """The HTML content of the block."""

    type: Literal["html"]
    """The type of the block."""

    layout_attrs: Optional[VisualBlockEmailHTMLBlockLayoutAttrs] = None
    """The layout attributes of the block."""


VisualBlock: TypeAlias = Union[VisualBlockEmailDividerBlock, VisualBlockEmailHTMLBlock]


class EmailTemplate(BaseModel):
    """An email message template."""

    visual_blocks: Optional[List[VisualBlock]] = None
    """The visual blocks that make up the email template."""
