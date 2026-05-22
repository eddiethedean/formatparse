"""Bidirectional parse and format."""

from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple, Union, cast

from ._native import FormatParser, ParseResult
from .api import compile
from .types import ExtraTypes, FieldConstraint


def _constraints_from_parser(parser: FormatParser) -> List[FieldConstraint]:
    """Build validation constraints from compiled field metadata."""
    constraints: List[FieldConstraint] = []
    for item in parser.field_constraints:
        name = item.get("name")
        if name is not None and not isinstance(name, str):
            name = None
        width = item.get("width")
        precision = item.get("precision")
        constraints.append(
            cast(
                FieldConstraint,
                {
                    "name": name,
                    "type": str(item["type"]),
                    "width": int(width) if width is not None else None,
                    "precision": int(precision) if precision is not None else None,
                },
            )
        )
    return constraints


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
        self._parser: FormatParser = compile(pattern, extra_types=extra_types)
        self._pattern: str = pattern
        self._extra_types: Optional[ExtraTypes] = extra_types
        self._field_constraints: List[FieldConstraint] = _constraints_from_parser(
            self._parser
        )

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
            return self._pattern.format(**cast(Dict[str, object], values))
        elif isinstance(values, tuple):
            return self._pattern.format(*values)
        elif isinstance(values, ParseResult):
            # Convert ParseResult to dict or tuple
            if values.named:
                return self._pattern.format(**cast(Dict[str, object], values.named))
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

            # Type validation (single-letter built-in tags only; custom types are multi-char)
            if len(field_type) == 1:
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
        self._named: Dict[str, Any] = dict(result.named) if result.named else {}
        self._fixed: List[Any] = list(result.fixed) if result.fixed else []

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
        return self._named

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
        return self._fixed

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
        if self._named:
            return self._pattern.format(self._named)
        return self._pattern.format(tuple(self._fixed))

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
        if self._named:
            return self._pattern.validate(self._named)
        return self._pattern.validate(tuple(self._fixed))

    def __repr__(self) -> str:
        """String representation"""
        if self._named:
            return f"<BidirectionalResult {self._named}>"
        return f"<BidirectionalResult {self._fixed}>"
