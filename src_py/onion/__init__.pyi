from typing import Any, Optional, Callable, Awaitable, List, overload

__version__: str

class PyOnionObject:
    """
    Python binding for Onion VM object. Provides type checks, value conversion, and operator overloads.
    """

    def __init__(self, obj: Any) -> None:
        """Create a new PyOnionObject from a Python value."""
        ...
    # --- Type check methods ---
    def is_integer(self) -> bool:
        """Return True if the object is an integer."""
        ...

    def is_float(self) -> bool:
        """Return True if the object is a float."""
        ...

    def is_string(self) -> bool:
        """Return True if the object is a string."""
        ...

    def is_bytes(self) -> bool:
        """Return True if the object is bytes."""
        ...

    def is_boolean(self) -> bool:
        """Return True if the object is a boolean."""
        ...

    def is_null(self) -> bool:
        """Return True if the object is null."""
        ...

    def is_undefined(self) -> bool:
        """Return True if the object is undefined."""
        ...

    def is_range(self) -> bool:
        """Return True if the object is a range."""
        ...

    def is_tuple(self) -> bool:
        """Return True if the object is a tuple."""
        ...

    def is_pair(self) -> bool:
        """Return True if the object is a pair."""
        ...

    def is_named(self) -> bool:
        """Return True if the object is a named value."""
        ...

    def is_custom(self) -> bool:
        """Return True if the object is a custom type."""
        ...

    # --- Value conversion methods ---
    def as_integer(self) -> int:
        """Convert the object to a Python int."""
        ...

    def as_float(self) -> float:
        """Convert the object to a Python float."""
        ...

    def as_string(self) -> str:
        """Convert the object to a Python str."""
        ...

    def as_bytes(self) -> bytes:
        """Convert the object to a Python bytes."""
        ...

    def as_boolean(self) -> bool:
        """Convert the object to a Python bool."""
        ...

    def as_range(self) -> "PyOnionObject":
        """Convert the object to a range object."""
        ...

    def as_tuple(self) -> "PyOnionObject":
        """Convert the object to a tuple object."""
        ...

    def as_pair(self) -> "PyOnionObject":
        """Convert the object to a pair object."""
        ...

    def as_named(self) -> "PyOnionObject":
        """Convert the object to a named object."""
        ...

    def unwrap_py(self) -> Any:
        """Unwrap the Python object from custom types."""
        ...

    # --- Other operations ---
    def type_name(self) -> str:
        """Get the Onion type name of the object."""
        ...

    def __repr__(self) -> str:
        """Return the string representation of the object."""
        ...

    def __str__(self) -> str:
        """Return the string conversion of the object."""
        ...

    def len(self) -> "PyOnionObject":
        """Get the length of the object as a PyOnionObject."""
        ...

    def key(self) -> "PyOnionObject":
        """Get the key of a pair or named object."""
        ...

    def value(self) -> "PyOnionObject":
        """Get the value of a pair or named object."""
        ...

    def __len__(self) -> int:
        """Return the Pythonic length of the object."""
        ...

    def __contains__(self, item: Any) -> bool:
        """Perform a membership test."""
        ...

    def __getitem__(self, index: Any) -> "PyOnionObject":
        """Enable indexing operations."""
        ...

    def __getattr__(self, attr: str) -> "PyOnionObject":
        """Enable attribute access."""
        ...

    def __setattr__(self, attr: str, value: Any) -> None:
        """Prevent attribute setting (always raises an error)."""
        ...

    def __eq__(self, other: Any) -> bool: ...
    def __lt__(self, other: Any) -> bool: ...
    def __gt__(self, other: Any) -> bool: ...
    def __add__(self, other: Any) -> "PyOnionObject": ...
    def __sub__(self, other: Any) -> "PyOnionObject": ...
    def __mul__(self, other: Any) -> "PyOnionObject": ...
    def __truediv__(self, other: Any) -> "PyOnionObject": ...
    def __mod__(self, other: Any) -> "PyOnionObject": ...
    def __pow__(self, other: Any, modulo: Optional[Any] = ...) -> "PyOnionObject": ...
    def __and__(self, other: Any) -> "PyOnionObject": ...
    def __or__(self, other: Any) -> "PyOnionObject": ...
    def __xor__(self, other: Any) -> "PyOnionObject": ...
    def __lshift__(self, other: Any) -> "PyOnionObject": ...
    def __rshift__(self, other: Any) -> "PyOnionObject": ...
    def __neg__(self) -> "PyOnionObject": ...
    def __pos__(self) -> "PyOnionObject": ...
    def __invert__(self) -> "PyOnionObject": ...
    @staticmethod
    def pair(k: Any, v: Any) -> "PyOnionObject":
        """Create a new pair object."""
        ...

    @staticmethod
    def named(k: Any, v: Any) -> "PyOnionObject":
        """Create a new named object."""
        ...

    @staticmethod
    def tuple(elements: List[Any]) -> "PyOnionObject":
        """Create a new tuple object from a list of elements."""
        ...

async def eval(
    code: str,
    work_dir: Optional[str] = ...,
    context: Optional[List[PyOnionObject]] = ...,
) -> PyOnionObject:
    """
    Evaluate Onion script asynchronously.

    :param code: Onion script code
    :param work_dir: Optional working directory
    :param context: Optional context variables, as a list of OnionNamed objects
    :return: Result as PyOnionObject
    """
    ...

def wrap_py_function(
    params: Any,
    signature: str,
    function: Callable[[PyOnionObject, PyOnionObject], Any],
    capture: Optional[Any] = ...,
    self_object: Optional[Any] = ...,
) -> PyOnionObject:
    """
    Wrap a Python function as an Onion callable object.

    :param params: Parameter specification, as a list of OnionNamed objects
    :param signature: Function signature string
    :param function: Python function to be wrapped, (self_object: PyOnionObject, arguments: PyOnionObject) -> Any
    :param capture: Optional captured variables
    :param self_object: Optional self object for methods
    :return: PyOnionObject representing the function
    """
    ...

def wrap_py_coroutine(
    params: Any,
    signature: str,
    coroutine: Callable[[PyOnionObject, PyOnionObject], Awaitable[Any]],
    capture: Optional[Any] = ...,
    self_object: Optional[Any] = ...,
) -> PyOnionObject:
    """
    Wrap a Python coroutine as an Onion callable coroutine object.

    :param params: Parameter specification, as a list of OnionNamed objects
    :param signature: Function signature string
    :param coroutine: Python async function to be wrapped, (self_object: PyOnionObject, arguments: PyOnionObject) -> Awaitable[Any]
    :param capture: Optional captured variables
    :param self_object: Optional self object for methods
    :return: PyOnionObject representing the coroutine
    """
    ...

class OnionRuntimeError(RuntimeError):
    """
    OnionRuntimeError is raised when an Onion script evaluation fails.
    It wraps a PyOnionObject that contains the error details.
    """
    ...

async def eval_or_throw(
    code: str,
    work_dir: Optional[str] = ...,
    context: Optional[List[PyOnionObject]] = ...,
) -> PyOnionObject:
    """
    Evaluate Onion script asynchronously.

    :param code: Onion script code
    :param work_dir: Optional working directory
    :param context: Optional context variables, as a list of OnionNamed objects
    :return: Result as PyOnionObject
    :raises OnionRuntimeError: If the evaluation fails, it raises OnionRuntimeError with the error details.
    """
    ...