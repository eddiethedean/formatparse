"""
Parse strings using a specification based on the Python format() syntax.

This is a Rust-backed implementation of the parse library for better performance.
"""

from datetime import timedelta, tzinfo
from typing import Any, Callable, Optional, Union
import re

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


class BidirectionalPattern:
    """
    A bidirectional pattern that can parse and format strings.

    Enables round-trip parsing: parse → modify → format back, with built-in validation.

    Example:
        >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
        >>> result = formatter.parse("      John: 00042")
        >>> result.named['name'] == 'John'
        True
        >>> result.named['value'] == 42
        True
        >>> result.format()
        '      John: 00042'
        >>> result.named['value'] = 100
        >>> result.format()
        '      John: 00100'
    """

    def __init__(self, pattern: str, extra_types=None):
        """
        Initialize a bidirectional pattern.

        Args:
            pattern: Format string pattern (e.g., "{name:>10}: {value:05d}")
            extra_types: Optional dict of custom type converters
        """
        self._parser = compile(pattern)
        self._pattern = pattern
        self._extra_types = extra_types
        # Parse pattern to extract field constraints for validation
        self._field_constraints = self._parse_constraints(pattern)

    def _parse_constraints(self, pattern: str) -> list[dict]:
        """Parse pattern string to extract field constraints for validation"""
        constraints = []
        # Match field patterns: {name:format} or {name} or {}
        field_pattern = r"\{([^}]*)\}"

        for match in re.finditer(field_pattern, pattern):
            field_spec = match.group(1)
            if not field_spec:
                # Positional field with no spec
                constraints.append(
                    {"name": None, "type": "s", "width": None, "precision": None}
                )
                continue

            # Parse field name and format spec
            parts = field_spec.split(":", 1)
            name = parts[0] if parts[0] else None
            format_spec = parts[1] if len(parts) > 1 else ""

            # Parse format spec (e.g., ">10", "05d", ".2f", ">10.5s")
            constraint = {"name": name, "type": "s", "width": None, "precision": None}

            # Extract type character (last letter if present)
            type_match = re.search(r"([a-zA-Z%])$", format_spec)
            if type_match:
                constraint["type"] = type_match.group(1)
                format_spec = format_spec[:-1]

            # Extract width and precision
            # Format: [fill][align][sign][width][.precision]
            # Handle formats like: "05d" (width=5), ">10" (width=10), ".5s" (precision=5), ">10.5s" (width=10, precision=5)

            # Check for precision first (after dot)
            dot_pos = format_spec.find(".")
            if dot_pos >= 0:
                # Has precision
                precision_str = format_spec[dot_pos + 1 :]
                # Remove type char from precision if present
                precision_str = re.sub(r"[a-zA-Z%]$", "", precision_str)
                if precision_str:
                    precision_match = re.search(r"(\d+)", precision_str)
                    if precision_match:
                        constraint["precision"] = int(precision_match.group(1))
                # Width is before the dot
                width_str = format_spec[:dot_pos]
            else:
                width_str = format_spec

            # Extract width from width_str (remove type char, fill, align, sign)
            # Remove type char if still present
            width_str = re.sub(r"[a-zA-Z%]$", "", width_str)
            # Remove fill, align, sign characters
            width_str = re.sub(r"[<>=^+\- ]", "", width_str)
            if width_str:
                width_match = re.search(r"(\d+)", width_str)
                if width_match:
                    constraint["width"] = int(width_match.group(1))

            constraints.append(constraint)

        return constraints

    def parse(
        self, string: str, case_sensitive: bool = False, evaluate_result: bool = True
    ) -> Optional["BidirectionalResult"]:
        """
        Parse a string and return BidirectionalResult.

        Args:
            string: String to parse
            case_sensitive: Whether matching is case-sensitive
            evaluate_result: Whether to evaluate result (convert types)

        Returns:
            BidirectionalResult if match found, None otherwise
        """
        result = self._parser.parse(
            string,
            extra_types=self._extra_types,
            case_sensitive=case_sensitive,
            evaluate_result=evaluate_result,
        )
        if result:
            return BidirectionalResult(self, result)
        return None

    def format(self, values: Union[dict, tuple, ParseResult]) -> str:
        """
        Format values back into the pattern.

        Args:
            values: Dict (for named fields), tuple (for positional), or ParseResult

        Returns:
            Formatted string
        """
        # Format.format() expects args or kwargs, not a dict directly
        # For named fields, we need to unpack the dict as kwargs
        if isinstance(values, dict):
            # Use Python's format() method directly with **kwargs
            return self._pattern.format(**values)
        elif isinstance(values, tuple):
            return self._pattern.format(*values)
        elif isinstance(values, ParseResult):
            # Convert ParseResult to dict or tuple
            if values.named:
                return self._pattern.format(**dict(values.named))
            else:
                return self._pattern.format(*values.fixed)
        else:
            return self._pattern.format(values)

    def validate(
        self, values: Union[dict, tuple, ParseResult]
    ) -> tuple[bool, list[str]]:
        """
        Validate values against format constraints.

        Args:
            values: Dict (for named fields), tuple (for positional), or ParseResult

        Returns:
            Tuple of (is_valid, list_of_errors)
        """
        errors = []

        # Convert values to dict/list format
        if isinstance(values, ParseResult):
            named_values = dict(values.named) if values.named else {}
            fixed_values = list(values.fixed) if values.fixed else []
        elif isinstance(values, dict):
            named_values = values
            fixed_values = []
        elif isinstance(values, tuple):
            named_values = {}
            fixed_values = list(values)
        else:
            return False, ["Invalid values type: expected dict, tuple, or ParseResult"]

        # Validate each field
        for i, constraint in enumerate(self._field_constraints):
            field_name = constraint["name"]
            field_type = constraint["type"]
            width = constraint["width"]
            precision = constraint["precision"]

            # Get value
            if field_name:
                if field_name not in named_values:
                    continue  # Field not present, skip validation
                value = named_values[field_name]
            else:
                if i >= len(fixed_values):
                    continue  # Positional field not present
                value = fixed_values[i]

            # Type validation
            if field_type == "d" and not isinstance(value, int):
                errors.append(
                    f"Field '{field_name or i}': expected int, got {type(value).__name__}"
                )
            elif field_type == "f" and not isinstance(value, (int, float)):
                errors.append(
                    f"Field '{field_name or i}': expected float, got {type(value).__name__}"
                )

            # Width/precision validation for strings
            if isinstance(value, str):
                if precision is not None and len(value) > precision:
                    errors.append(
                        f"Field '{field_name or i}': string length {len(value)} exceeds precision {precision}"
                    )
                if width is not None and len(value) > width:
                    errors.append(
                        f"Field '{field_name or i}': string length {len(value)} exceeds width {width}"
                    )

            # Width validation for integers (zero-padded)
            if isinstance(value, int) and width is not None:
                # Check if value fits in width with zero-padding
                # Need to account for sign if negative
                value_str = str(abs(value))
                sign_len = 1 if value < 0 else 0
                if len(value_str) + sign_len > width:
                    errors.append(
                        f"Field '{field_name or i}': integer {value} exceeds width {width} (with zero-padding)"
                    )

        return len(errors) == 0, errors


class BidirectionalResult:
    """
    Result from BidirectionalPattern.parse() that allows modification and formatting.

    Stores parsed values in a mutable format and provides methods to format back
    and validate against the original pattern constraints.
    """

    def __init__(self, pattern: BidirectionalPattern, result: ParseResult):
        """
        Initialize a bidirectional result.

        Args:
            pattern: The BidirectionalPattern that created this result
            result: The ParseResult from parsing
        """
        self._pattern = pattern
        self._result = result
        # Store values in mutable dict/list
        self._values = {
            "named": dict(result.named) if result.named else {},
            "fixed": list(result.fixed) if result.fixed else [],
        }

    @property
    def named(self) -> dict[str, Any]:
        """Mutable named fields dictionary"""
        return self._values["named"]  # type: ignore[return-value]

    @property
    def fixed(self) -> list[Any]:
        """Mutable fixed (positional) fields list"""
        return self._values["fixed"]  # type: ignore[return-value]

    def format(self) -> str:
        """
        Format values back using the pattern.

        Returns:
            Formatted string matching the original pattern
        """
        if self._values["named"]:
            return self._pattern.format(self._values["named"])
        else:
            return self._pattern.format(tuple(self._values["fixed"]))

    def validate(self) -> tuple[bool, list[str]]:
        """
        Validate current values against format constraints.

        Returns:
            Tuple of (is_valid, list_of_errors)
        """
        # Pass the actual values dict/list, not the wrapper structure
        if self._values["named"]:
            return self._pattern.validate(self._values["named"])
        else:
            return self._pattern.validate(tuple(self._values["fixed"]))

    def __repr__(self) -> str:
        """String representation"""
        if self._values["named"]:
            return f"<BidirectionalResult {self._values['named']}>"
        else:
            return f"<BidirectionalResult {self._values['fixed']}>"


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
    "BidirectionalPattern",
    "BidirectionalResult",
]
