# File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

from typing import List, Union, Optional
from typing_extensions import Literal, Annotated, TypeAlias

from ..._utils import PropertyInfo
from ..._models import BaseModel

__all__ = ["Extensions", "ExtensionsItem", "ExtensionsItemRemoveBg", "ExtensionsItemAutoDescription"]


class ExtensionsItemRemoveBg(BaseModel):
    name: Literal["remove-bg"]

    add_shadow: Optional[bool] = None


class ExtensionsItemAutoDescription(BaseModel):
    name: Literal["auto-description"]


ExtensionsItem: TypeAlias = Annotated[
    Union[ExtensionsItemRemoveBg, ExtensionsItemAutoDescription], PropertyInfo(discriminator="name")
]

Extensions: TypeAlias = List[ExtensionsItem]
