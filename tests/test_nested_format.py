"""Nested format patterns in field specs (GitHub issue #12; upstream parse#206)."""

from __future__ import annotations

import pytest

import formatparse


def test_nested_named_integer() -> None:
    r = formatparse.parse("{outer:{inner:d}}", "42")
    assert r is not None
    outer = r.named["outer"]
    assert isinstance(outer, formatparse.ParseResult)
    assert outer.named["inner"] == 42


def test_nested_compile_regex_contains_inner_named_group() -> None:
    p = formatparse.compile("{outer:{inner:d}}")
    assert "(?P<inner>" in p.regex_subpattern


def test_nested_findall() -> None:
    matches = formatparse.findall("{a:{b:d}}", "x12y34")
    assert len(matches) == 2
    assert matches[0].named["a"].named["b"] == 12
    assert matches[1].named["a"].named["b"] == 34


def test_nested_max_depth_compile_error() -> None:
    # `MAX_NESTED_FORMAT_DEPTH` is 10: nested `parse_pattern` runs at depth 10 for `{leaf:d}`
    # only when there are ten wrappers (depths 0..9 inclusive, then leaf). An eleventh
    # wrapper forces nested compilation at depth 10 and triggers the cap.
    def chain(n: int) -> str:
        body = "{leaf:d}"
        for i in range(n):
            body = "{a%d:" % i + body + "}"
        return body

    formatparse.compile(chain(10))
    with pytest.raises(ValueError, match="max depth|too many nested"):
        formatparse.compile(chain(11))


def test_unclosed_nested_spec_pattern_error() -> None:
    with pytest.raises(ValueError, match="Unclosed"):
        formatparse.compile("{a:{b:d}")
