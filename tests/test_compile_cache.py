"""Tests for compile() sharing the pattern LRU cache (issue #29)."""

from __future__ import annotations

import formatparse
from formatparse import compile, parse, search, with_pattern


def test_compile_twice_same_pattern_both_parse() -> None:
    """Two compile() calls with the same pattern yield independent usable parsers."""
    p1 = formatparse.compile("{x}")
    p2 = formatparse.compile("{x}")
    r1 = p1.parse("hello")
    r2 = p2.parse("world")
    assert r1 is not None and r1.named["x"] == "hello"
    assert r2 is not None and r2.named["x"] == "world"


def test_compile_cache_distinguishes_extra_types_converter_patterns() -> None:
    """Cache must not reuse a parser when converter patterns differ for the same key."""

    @with_pattern(r"\d+")
    def as_int(text):
        return int(text)

    @with_pattern(r"[a-z]+")
    def as_lower(text):
        return text.lower()

    fmt = "Value is {x:T}"
    r1 = parse(fmt, "Value is 42", extra_types={"T": as_int})
    assert r1.named["x"] == 42
    r2 = parse(fmt, "Value is hello", extra_types={"T": as_lower})
    assert r2.named["x"] == "hello"

    s1 = search(fmt, "xx Value is 99 yy", extra_types={"T": as_int})
    assert s1.named["x"] == 99
    s2 = search(fmt, "xx Value is abc yy", extra_types={"T": as_lower})
    assert s2.named["x"] == "abc"


def test_compile_equivalent_extra_types_produces_consistent_results() -> None:
    """Repeated compile with the same extra_types mapping parses consistently."""

    @with_pattern(r"\d+")
    def conv(text):
        return int(text)

    extra = {"CacheInt": conv}
    p1 = compile("{v:CacheInt}", extra_types=extra)
    p2 = compile("{v:CacheInt}", extra_types=extra)
    assert p1.parse("7").named["v"] == p2.parse("7").named["v"] == 7
