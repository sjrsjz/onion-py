"""Onion Python 高级封装，导出核心 API。"""

from __future__ import annotations
from typing import Optional, List
from onion.onion import eval, PyOnionObject, wrap_py_function, wrap_py_coroutine


class OnionRuntimeError(RuntimeError):
    """
    OnionRuntimeError is raised when an Onion script evaluation fails.
    It wraps a PyOnionObject that contains the error details.
    """
    def __init__(self, v):
        super().__init__(v)
        self.value = v


async def eval_or_throw(
    code: str,
    work_dir: Optional[str] = None,
    context: Optional[List[PyOnionObject]] = None,
) -> PyOnionObject:
    """
    Evaluate Onion script asynchronously.

    :param code: Onion script code
    :param work_dir: Optional working directory
    :param context: Optional context variables, as a list of OnionNamed objects
    :return: Result as PyOnionObject
    :raises OnionRuntimeError: If the evaluation fails, it raises OnionRuntimeError with the error details.
    """
    result = await eval(code, work_dir, context)
    if not result.is_pair():
        raise RuntimeError(f"Cannot resolve result: {result}")
    k = result.key()
    v = result.value()
    if k.as_boolean():
        return v
    raise OnionRuntimeError(v)


__all__ = [
    "eval",
    "PyOnionObject",
    "wrap_py_function",
    "wrap_py_coroutine",
    "OnionRuntimeError",
    "eval_or_throw",
]
