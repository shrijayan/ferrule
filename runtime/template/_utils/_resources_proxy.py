from __future__ import annotations

from typing import Any
from typing_extensions import override

from ._proxy import LazyProxy


class ResourcesProxy(LazyProxy[Any]):
    """A proxy for the `__ferrule_pkg__.resources` module.

    This is used so that we can lazily import `__ferrule_pkg__.resources` only when
    needed *and* so that users can just import `__ferrule_pkg__` and reference `__ferrule_pkg__.resources`
    """

    @override
    def __load__(self) -> Any:
        import importlib

        mod = importlib.import_module("__ferrule_pkg__.resources")
        return mod


resources = ResourcesProxy().__as_proxied__()
