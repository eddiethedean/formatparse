"""Bidirectional parse and format."""
from __future__ import annotations

import re
from typing import Any, Dict, List, Optional, Tuple, Union

from ._native import FormatParser, ParseResult
from .api import compile
from .types import ExtraTypes, FieldConstraint

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

    def __init__(self, pattern: str, extra_types: Optional[ExtraTypes] = None) -> None:
        """Initialize a bidirectional pattern.

        :param pattern: Format string pattern (e.g., ``"{name:>10}: {value:05d}"``)
        :type pattern: str
        :param extra_types: Optional dictionary of custom type converters
        :type extra_types: dict, optional
        """
        self._parser: FormatParser = compile(pattern)
        self._pattern: str = pattern
        self._extra_types: Optional[ExtraTypes] = extra_types
        # Parse pattern to extract field constraints for validation
        self._field_constraints: List[FieldConstraint] = self._parse_constraints(
            pattern
        )

    def _parse_constraints(self, pattern: str) -> List[FieldConstraint]:
        """Parse pattern string to extract field constraints for validation"""
        constraints = []
        # Match field patterns: {name:format} or {name} or {}
        field_pattern = r"\{([^}]*)\}"

        for match in re.finditer(field_pattern, pattern):
            field_spec = match.group(1)
            constraint: FieldConstraint
            if not field_spec:
                # Positional field with no spec
                constraint = {
                    "name": None,
                    "type": "s",
                    "width": None,
                    "precision": None,
                }
                constraints.append(constraint)
                continue

            # Parse field name and format spec
            parts = field_spec.split(":", 1)
            name = parts[0] if parts[0] else None
            format_spec = parts[1] if len(parts) > 1 else ""

            # Parse format spec (e.g., ">10", "05d", ".2f", ">10.5s")
            constraint = {
                "name": name,
                "type": "s",
                "width": None,
                "precision": None,
            }

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

    def __init__(self, pattern: BidirectionalPattern, result: ParseResult) -> None:
        """Initialize a bidirectional result.

        :param pattern: The BidirectionalPattern that created this result
        :type pattern: BidirectionalPattern
        :param result: The ParseResult from parsing
        :type result: ParseResult
        """
        self._pattern: BidirectionalPattern = pattern
        self._result: ParseResult = result
        # Store values in mutable dict/list
        self._values: Dict[str, Union[Dict[str, Any], List[Any]]] = {
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
