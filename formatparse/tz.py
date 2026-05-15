"""Timezone helpers."""

from __future__ import annotations

from datetime import datetime, timedelta, tzinfo
from typing import Optional

from ._native import _FixedTzOffset


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

    def __init__(self, offset_minutes: int, name: str) -> None:
        """Initialize a fixed timezone offset.

        :param offset_minutes: Timezone offset in minutes from UTC
        :type offset_minutes: int
        :param name: Timezone name (e.g., "EST", "PST")
        :type name: str
        """
        self._rust_tz: _FixedTzOffset = _FixedTzOffset(offset_minutes, name)
        self._offset_minutes: int = offset_minutes
        self._name: str = name

    def __repr__(self) -> str:
        return repr(self._rust_tz)

    def __str__(self) -> str:
        return str(self._rust_tz)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, FixedTzOffset):
            return self._rust_tz == other._rust_tz
        elif (
            hasattr(other, "__class__") and other.__class__.__name__ == "FixedTzOffset"
        ):
            # Handle comparison with Rust FixedTzOffset
            return self._rust_tz == other
        return False

    def __ne__(self, other: object) -> bool:
        return not self.__eq__(other)

    def utcoffset(self, dt: Optional[datetime]) -> timedelta:
        """Return the timezone offset from UTC.

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone offset as timedelta
        :rtype: datetime.timedelta
        """
        return timedelta(minutes=self._offset_minutes)

    def dst(self, dt: Optional[datetime]) -> None:
        """Return daylight saving time adjustment (always None for fixed offsets).

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Always None for fixed timezone offsets
        :rtype: None
        """
        return None

    def tzname(self, dt: Optional[datetime]) -> str:
        """Return the timezone name.

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone name
        :rtype: str
        """
        return self._name
