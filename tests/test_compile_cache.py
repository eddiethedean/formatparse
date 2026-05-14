"""Tests for compile() sharing the pattern LRU cache (issue #29)."""

from __future__ import annotations

import formatparse


def test_compile_twice_same_pattern_both_parse() -> None:
    """Two compile() calls with the same pattern should yield usable parsers."""
    p1 = formatparse.compile("{x}")
    p2 = formatparse.compile("{x}")
    r1 = p1.parse("hello")
    r2 = p2.parse("world")
    assert r1 is not None and r1.named["x"] == "hello"
    assert r2 is not None and r2.named["x"] == "world"
