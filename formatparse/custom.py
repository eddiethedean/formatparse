"""Custom type converters and composition."""

from __future__ import annotations

from typing import Any, Callable, TypeVar, cast

from ._native import FormatParser, ParseResult
from .types import ConverterProtocol


F = TypeVar("F", bound=Callable[[str], Any])


def with_pattern(
    pattern: str, regex_group_count: int = 0
) -> Callable[[F], ConverterProtocol]:
    """Decorator to create a custom type converter with a regex pattern.

    This decorator adds a ``pattern`` attribute to the converter function,
    which is used by the parse functions when matching custom types.

    :param pattern: The regex pattern to match
    :type pattern: str
    :param regex_group_count: Number of regex groups in the pattern (for parentheses) (default: 0)
    :type regex_group_count: int
    :returns: Decorator function that adds the pattern attribute
    :rtype: Callable

    Example::

        >>> @with_pattern(r'\\d+')
        ... def parse_number(text):
        ...     return int(text)
        >>> result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
        >>> result.fixed[0]
        42
        >>> type(result.fixed[0])
        <class 'int'>

        >>> @with_pattern(r'[A-Z]{2,3}')
        ... def parse_code(text):
        ...     return text.upper()
        >>> result = parse("Code: {:Code}", "Code: abc", {"Code": parse_code})
        >>> result.fixed[0]
        'ABC'
    """

    def decorator(func: F) -> ConverterProtocol:
        setattr(func, "pattern", pattern)
        setattr(func, "regex_group_count", regex_group_count)
        return cast(ConverterProtocol, func)

    return decorator


class ComposedType:
    """Wrap a compiled :class:`FormatParser` for use as one ``extra_types`` converter.

    Instances expose ``pattern`` and ``regex_group_count`` for the parent's regex
    builder (GitHub issue #7). Prefer constructing via :func:`composed_type`.

    Pickling a parent :class:`FormatParser` still does not restore ``extra_types``;
    rebuild composed mappings after unpickling.
    """

    __slots__ = ("_parser", "pattern", "regex_group_count")

    def __init__(self, parser: FormatParser) -> None:
        self._parser = parser
        self.pattern = parser.regex_subpattern
        self.regex_group_count = parser.regex_capturing_group_count

    def __call__(self, text: str) -> ParseResult:
        result = self._parser.parse(text)
        if result is None:
            raise ValueError(
                "Composed sub-parser did not match the captured text; the child "
                "pattern must accept the substring matched by the parent field."
            )
        return result


def composed_type(parser: FormatParser) -> ComposedType:
    """Wrap a compiled parser for embedding in another pattern's ``extra_types``.

    The parent pattern refers to a custom type name; the value is this wrapper,
    which delegates parsing of the captured substring to ``parser``.

    Example::

        >>> from formatparse import compile, composed_type
        >>> ts = compile("{year:d}-{month:02d}-{day:02d}")
        >>> log = compile(
        ...     "{ts:Timestamp} [{level}] {msg}",
        ...     extra_types={"Timestamp": composed_type(ts)},
        ... )
        >>> r = log.parse("2024-01-15 [ERROR] oops")
        >>> r.named["level"]
        'ERROR'
        >>> r.named["msg"]
        'oops'
        >>> inner = r.named["ts"]
        >>> inner.named["year"], inner.named["month"], inner.named["day"]
        (2024, 1, 15)

    :param parser: Child parser produced by :func:`compile`.
    :returns: Callable with ``pattern`` / ``regex_group_count`` set for composition.

    .. note::
        Pattern ``+``, inheritance, and flattening nested fields into the parent
        result are not implemented yet (see issue #7).
    """
    return ComposedType(parser)
