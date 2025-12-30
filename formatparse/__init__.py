"""
Parse strings using a specification based on the Python format() syntax.

This is a Rust-backed implementation of the parse library for better performance.
"""

from datetime import timedelta, tzinfo
from typing import Any, Callable, Dict, List, Optional, Tuple, Union
import re

# Import from the Rust extension module
from _formatparse import (  # type: ignore[import-not-found]
    parse as _parse,
    search as _search,
    findall as _findall,
    compile as _compile,
    ParseResult,
    FormatParser,
    FixedTzOffset as _FixedTzOffset,
)


# Define RepeatedNameError exception (matches original parse library)
class RepeatedNameError(ValueError):
    """Exception raised when a repeated field name has mismatched types.
    
    This exception is raised when a format pattern contains the same field name
    multiple times with different type specifications (e.g., ``"{age:d}"`` and
    ``"{age:f}"`` in the same pattern).
    
    :raises RepeatedNameError: When a repeated field name has mismatched types
    
    Example::
    
        >>> from formatparse import compile, RepeatedNameError
        >>> try:
        ...     compile("{age:d} years and {age:f} months")
        ... except RepeatedNameError as e:
        ...     print(f"Error: {e}")
    """

    pass


# Wrap compile to catch RepeatedNameError
def compile(pattern: str):
    """Compile a pattern into a FormatParser for repeated use.
    
    Compiling a pattern allows you to reuse the same pattern multiple times
    without recompiling the regex, which improves performance for repeated
    parsing operations.
    
    :param pattern: Format specification pattern (e.g., ``"{name}: {age:d}"``)
    :type pattern: str
    :returns: FormatParser object that can be used to parse strings
    :rtype: FormatParser
    :raises RepeatedNameError: If a repeated field name has mismatched types
    :raises ValueError: If pattern is invalid
    
    Example::
    
        >>> parser = compile("{name}: {age:d}")
        >>> result = parser.parse("Alice: 30")
        >>> result.named['name']
        'Alice'
        >>> result.named['age']
        30
        >>> result2 = parser.parse("Bob: 25")
        >>> result2.named['name']
        'Bob'
        >>> result2.named['age']
        25
    """
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
    """Parse a string using a format specification.
    
    This function parses a string according to a format pattern and extracts
    named or positional fields from it. The pattern syntax is based on Python's
    format() function syntax.
    
    :param pattern: Format specification pattern (e.g., ``"{name}: {age:d}"``)
    :type pattern: str
    :param string: String to parse
    :type string: str
    :param extra_types: Optional dictionary of custom type converters
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: False)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :returns: ParseResult object if match found, None otherwise
    :rtype: ParseResult or None
    :raises ValueError: If pattern is invalid
    
    Example::
    
        >>> result = parse("{name}: {age:d}", "Alice: 30")
        >>> result.named['name']
        'Alice'
        >>> result.named['age']
        30
        >>> result = parse("{}, {}", "Hello, World")
        >>> result.fixed
        ('Hello', 'World')
    """
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
    """Search for a pattern anywhere in a string.
    
    Unlike parse(), which matches the entire string, search() finds the first
    occurrence of the pattern anywhere within the string.
    
    :param pattern: Format specification pattern
    :type pattern: str
    :param string: String to search
    :type string: str
    :param pos: Start position for search (default: 0)
    :type pos: int
    :param endpos: End position for search (default: None for end of string)
    :type endpos: int, optional
    :param extra_types: Optional dictionary of custom type converters
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: True)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :returns: ParseResult object if match found, None otherwise
    :rtype: ParseResult or None
    :raises ValueError: If pattern is invalid
    
    Example::
    
        >>> result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
        >>> result.named['age']
        30
        >>> result = search("age: {age:d}", "No age here")
        >>> result is None
        True
    """
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
    """Find all matches of a pattern in a string.
    
    Searches for all non-overlapping occurrences of the pattern in the string
    and returns a list-like Results object containing all matches.
    
    :param pattern: Format specification pattern
    :type pattern: str
    :param string: String to search
    :type string: str
    :param extra_types: Optional dictionary of custom type converters
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: False)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :returns: Results object (list-like) containing ParseResult objects
    :rtype: Results
    
    Example::
    
        >>> results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
        >>> len(results)
        3
        >>> results[0].named['id']
        1
        >>> results[1].named['id']
        2
        >>> results[2].named['id']
        3
        >>> for result in results:
        ...     print(result.named['id'])
        1
        2
        3
    """
    return _findall(pattern, string, extra_types, case_sensitive, evaluate_result)


# Create a tzinfo-compatible wrapper for FixedTzOffset
class FixedTzOffset(tzinfo):
    """Fixed timezone offset compatible with datetime.tzinfo.
    
    This class provides a fixed timezone offset implementation that is compatible
    with Python's datetime.tzinfo interface. It's used internally for datetime
    parsing when timezone information is present.
    
    :param offset_minutes: Timezone offset in minutes from UTC
    :type offset_minutes: int
    :param name: Timezone name (e.g., "EST", "PST")
    :type name: str
    
    Example::
    
        >>> from formatparse import FixedTzOffset
        >>> from datetime import datetime
        >>> tz = FixedTzOffset(300, "EST")  # UTC-5
        >>> dt = datetime(2024, 1, 1, 12, 0, tzinfo=tz)
        >>> tz.utcoffset(dt)
        datetime.timedelta(seconds=18000)
        >>> tz.dst(dt) is None
        True
        >>> tz.tzname(dt)
        'EST'
    """

    def __init__(self, offset_minutes, name):
        """Initialize a fixed timezone offset.
        
        :param offset_minutes: Timezone offset in minutes from UTC
        :type offset_minutes: int
        :param name: Timezone name (e.g., "EST", "PST")
        :type name: str
        """
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
        """Return the timezone offset from UTC.
        
        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone offset as timedelta
        :rtype: datetime.timedelta
        """
        return timedelta(minutes=self._offset_minutes)

    def dst(self, dt):
        """Return daylight saving time adjustment (always None for fixed offsets).
        
        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Always None for fixed timezone offsets
        :rtype: None
        """
        return None

    def tzname(self, dt):
        """Return the timezone name.
        
        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone name
        :rtype: str
        """
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

    def decorator(func: Callable) -> Callable:
        func.pattern = pattern  # type: ignore[attr-defined]
        func.regex_group_count = regex_group_count  # type: ignore[attr-defined]
        return func

    return decorator


class BidirectionalPattern:
    """A bidirectional pattern that can parse and format strings.

    Enables round-trip parsing: parse → modify → format back, with built-in validation.
    This class combines parsing and formatting capabilities, allowing you to parse
    a string, modify the extracted values, and format them back while maintaining
    the original format constraints.
    
    :param pattern: Format string pattern (e.g., ``"{name:>10}: {value:05d}"``)
    :type pattern: str
    :param extra_types: Optional dictionary of custom type converters
    :type extra_types: dict, optional

    Example::
    
        >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
        >>> result = formatter.parse("      John: 00042")
        >>> result.named['name']
        'John'
        >>> result.named['value']
        42
        >>> result.format()
        '      John: 00042'
        >>> result.named['value'] = 100
        >>> result.format()
        '      John: 00100'
    """

    def __init__(self, pattern: str, extra_types=None):
        """Initialize a bidirectional pattern.

        :param pattern: Format string pattern (e.g., ``"{name:>10}: {value:05d}"``)
        :type pattern: str
        :param extra_types: Optional dictionary of custom type converters
        :type extra_types: dict, optional
        """
        self._parser = compile(pattern)
        self._pattern = pattern
        self._extra_types = extra_types
        # Parse pattern to extract field constraints for validation
        self._field_constraints = self._parse_constraints(pattern)

    def _parse_constraints(self, pattern: str) -> List[Dict]:
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
        """Parse a string and return BidirectionalResult.

        :param string: String to parse
        :type string: str
        :param case_sensitive: Whether matching is case-sensitive (default: False)
        :type case_sensitive: bool
        :param evaluate_result: Whether to evaluate result (convert types) (default: True)
        :type evaluate_result: bool
        :returns: BidirectionalResult if match found, None otherwise
        :rtype: BidirectionalResult or None
        
        Example::
        
            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.named['name']
            'John'
            >>> result.named['value']
            42
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
        """Format values back into the pattern.

        Formats the provided values according to the pattern specification,
        maintaining format constraints like width, precision, and alignment.
        
        :param values: Dictionary (for named fields), tuple (for positional), or ParseResult
        :type values: dict, tuple, or ParseResult
        :returns: Formatted string matching the pattern
        :rtype: str
        
        Example::
        
            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> formatter.format({"name": "John", "value": 42})
            '      John: 00042'
            >>> formatter.format(("John", 42))  # Positional fields
            '      John: 00042'
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
    ) -> Tuple[bool, List[str]]:
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
    """Result from BidirectionalPattern.parse() that allows modification and formatting.

    Stores parsed values in a mutable format and provides methods to format back
    and validate against the original pattern constraints. Unlike ParseResult, this
    class allows you to modify the extracted values and format them back while
    maintaining the original format constraints.
    
    Example::
    
        >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
        >>> result = formatter.parse("      John: 00042")
        >>> result.named['value'] = 100
        >>> result.format()
        '      John: 00100'
        >>> result.validate()
        (True, [])
    """

    def __init__(self, pattern: BidirectionalPattern, result: ParseResult):
        """Initialize a bidirectional result.

        :param pattern: The BidirectionalPattern that created this result
        :type pattern: BidirectionalPattern
        :param result: The ParseResult from parsing
        :type result: ParseResult
        """
        self._pattern = pattern
        self._result = result
        # Store values in mutable dict/list
        self._values = {
            "named": dict(result.named) if result.named else {},
            "fixed": list(result.fixed) if result.fixed else [],
        }

    @property
    def named(self) -> Dict[str, Any]:
        """Mutable named fields dictionary.
        
        :returns: Dictionary of named fields (can be modified)
        :rtype: Dict[str, Any]
        
        Example::
        
            >>> formatter = BidirectionalPattern("{name}: {age:d}")
            >>> result = formatter.parse("Alice: 30")
            >>> result.named['age'] = 31
            >>> result.format()
            'Alice: 31'
        """
        return self._values["named"]  # type: ignore[return-value]

    @property
    def fixed(self) -> List[Any]:
        """Mutable fixed (positional) fields list.
        
        :returns: List of positional fields (can be modified)
        :rtype: List[Any]
        
        Example::
        
            >>> formatter = BidirectionalPattern("{}, {}")
            >>> result = formatter.parse("Hello, World")
            >>> result.fixed[1] = "Python"
            >>> result.format()
            'Hello, Python'
        """
        return self._values["fixed"]  # type: ignore[return-value]

    def format(self) -> str:
        """Format values back using the pattern.

        Formats the current (potentially modified) values according to the
        original pattern specification.
        
        :returns: Formatted string matching the original pattern
        :rtype: str
        
        Example::
        
            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.named['value'] = 100
            >>> result.format()
            '      John: 00100'
        """
        if self._values["named"]:
            return self._pattern.format(self._values["named"])
        else:
            return self._pattern.format(tuple(self._values["fixed"]))

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current values against format constraints.

        Checks if the current (potentially modified) values conform to the
        pattern's constraints (type, width, precision).
        
        :returns: Tuple of (is_valid, list_of_errors)
        :rtype: Tuple[bool, List[str]]
        
        Example::
        
            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.validate()
            (True, [])
            >>> result.named['value'] = "not a number"
            >>> is_valid, errors = result.validate()
            >>> is_valid
            False
            >>> len(errors) > 0
            True
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
    "with_pattern",
]
