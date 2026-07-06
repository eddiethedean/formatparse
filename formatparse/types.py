"""Type aliases and protocols for formatparse."""

from __future__ import annotations

from typing import (
    Any,
    Callable,
    Dict,
    Literal,
    Optional,
    Protocol,
    TypedDict,
    Union,
)


class ConverterProtocol(Protocol):
    """Protocol for custom type converter functions."""

    pattern: str
    regex_group_count: int

    def __call__(self, text: str) -> Any:
        """Convert a text string to a value."""
        ...


ExtraTypes = Dict[str, ConverterProtocol]


class FieldConstraint(TypedDict, total=False):
    """Type for field constraint dictionaries."""

    name: Optional[str]
    type: str
    width: Optional[int]
    precision: Optional[int]


ValidationMode = Literal["strict", "collect", "lenient"]

ValidatorMap = Dict[Union[str, int], Callable[..., Any]]
"""Validators keyed by field name (:class:`str`) or positional index (:class:`int`)."""
