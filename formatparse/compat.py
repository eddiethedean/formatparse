"""Compatibility aliases with the original parse library."""
from __future__ import annotations

from typing import Dict

from ._native import FormatParser, ParseResult

# Export with names matching original parse library API
Result = ParseResult
Parser = FormatParser

# Module attribute for compatibility with original parse library
# Maps strftime format codes to their regex patterns
dt_format_to_regex: Dict[str, str] = {
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
