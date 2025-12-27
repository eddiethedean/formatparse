"""
Parse strings using a specification based on the Python format() syntax.

This is a Rust-backed implementation of the parse library for better performance.
"""

from datetime import timedelta, tzinfo
from typing import Callable

# Import from the Rust extension module
from _formatparse import (  # type: ignore[import-not-found]
    parse as _parse,
    search as _search,
    findall as _findall,
    compile as _compile,
    extract_format,
    ParseResult,
    FormatParser,
    FixedTzOffset as _FixedTzOffset,
    Match,
)


# Define RepeatedNameError exception (matches original parse library)
class RepeatedNameError(ValueError):
    """Exception raised when a repeated field name has mismatched types"""

    pass


# Wrap compile to catch RepeatedNameError
def compile(pattern: str):
    """Compile a pattern into a FormatParser"""
    try:
        return _compile(pattern)
    except ValueError as e:
        if "Repeated name" in str(e) and "mismatched types" in str(e):
            raise RepeatedNameError(str(e)) from e
        raise


# Wrap parse, search, findall to match original API
def parse(
    pattern: str,
    string: str,
    extra_types=None,
    case_sensitive=False,
    evaluate_result=True,
):
    """Parse a string using a format specification"""
    return _parse(pattern, string, extra_types, case_sensitive, evaluate_result)


def search(
    pattern: str,
    string: str,
    pos=0,
    endpos=None,
    extra_types=None,
    case_sensitive=True,
    evaluate_result=True,
):
    """Search for a pattern in a string"""
    # Validate pos parameter - handle negative values
    if pos < 0:
        pos = 0
    if pos > len(string):
        return None

    # Validate endpos parameter
    if endpos is not None:
        if endpos < 0:
            endpos = 0
        if endpos > len(string):
            endpos = len(string)
        if endpos < pos:
            return None

    return _search(
        pattern, string, pos, endpos, extra_types, case_sensitive, evaluate_result
    )


def findall(
    pattern: str,
    string: str,
    extra_types=None,
    case_sensitive=False,
    evaluate_result=True,
):
    """Find all matches of a pattern in a string"""
    return _findall(pattern, string, extra_types, case_sensitive, evaluate_result)


# Create a tzinfo-compatible wrapper for FixedTzOffset
class FixedTzOffset(tzinfo):
    """Fixed timezone offset compatible with datetime.tzinfo"""

    def __init__(self, offset_minutes, name):
        self._rust_tz = _FixedTzOffset(offset_minutes, name)
        self._offset_minutes = offset_minutes
        self._name = name

    def __repr__(self):
        return repr(self._rust_tz)

    def __str__(self):
        return str(self._rust_tz)

    def __eq__(self, other):
        if isinstance(other, FixedTzOffset):
            return self._rust_tz == other._rust_tz
        elif (
            hasattr(other, "__class__") and other.__class__.__name__ == "FixedTzOffset"
        ):
            # Handle comparison with Rust FixedTzOffset
            return self._rust_tz == other
        return False

    def __ne__(self, other):
        return not self.__eq__(other)

    def utcoffset(self, dt):
        return timedelta(minutes=self._offset_minutes)

    def dst(self, dt):
        return None

    def tzname(self, dt):
        return self._name


# Export with names matching original parse library API
Result = ParseResult
Parser = FormatParser

# Module attribute for compatibility with original parse library
# Maps strftime format codes to their regex patterns
dt_format_to_regex = {
    "%Y": r"\d{4}",  # Year with century
    "%y": r"\d{2}",  # Year without century
    "%m": r"\d{1,2}",  # Month (1-12 or 01-12) - flexible
    "%d": r"\d{1,2}",  # Day (1-31 or 01-31) - flexible
    "%H": r"\d{1,2}",  # Hour (0-23 or 00-23) - flexible
    "%M": r"\d{1,2}",  # Minute (0-59 or 00-59) - flexible
    "%S": r"\d{1,2}",  # Second (0-59 or 00-59) - flexible
    "%f": r"\d{1,6}",  # Microseconds
    "%b": r"[A-Za-z]{3}",  # Abbreviated month name
    "%B": r"[A-Za-z]+",  # Full month name
    "%a": r"[A-Za-z]{3}",  # Abbreviated weekday
    "%A": r"[A-Za-z]+",  # Full weekday
    "%w": r"\d",  # Weekday as decimal (0=Sunday)
    "%j": r"\d{1,3}",  # Day of year (1-366, flexible padding)
    "%U": r"\d{2}",  # Week number (Sunday as first day)
    "%W": r"\d{2}",  # Week number (Monday as first day)
    "%c": r".+",  # Date and time representation (locale dependent)
    "%x": r".+",  # Date representation (locale dependent)
    "%X": r".+",  # Time representation (locale dependent)
    "%%": "%",  # Literal %
}


def with_pattern(pattern: str, regex_group_count: int = 0):
    """
    Decorator to create a custom type converter with a regex pattern.

    This decorator adds a `pattern` attribute to the converter function,
    which is used by the parse functions when matching custom types.

    Args:
        pattern: The regex pattern to match
        regex_group_count: Number of regex groups in the pattern (for parentheses)

    Returns:
        A decorator that adds the pattern attribute to the converter function

    Example:
        @with_pattern(r'\\d+')
        def parse_number(text):
            return int(text)

        result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
    """

    def decorator(func: Callable) -> Callable:
        func.pattern = pattern  # type: ignore[attr-defined]
        func.regex_group_count = regex_group_count  # type: ignore[attr-defined]
        return func

    return decorator


__all__ = [
    "parse",
    "search",
    "findall",
    "compile",
    "extract_format",
    "with_pattern",
    "ParseResult",
    "FormatParser",
    "FixedTzOffset",
    "RepeatedNameError",
    "Match",
    "Result",  # Alias for ParseResult (original parse library name)
    "Parser",  # Alias for FormatParser (original parse library name)
]
