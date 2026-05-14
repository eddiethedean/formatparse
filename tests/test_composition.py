"""Composition MVP: embed compiled parsers via extra_types (GitHub issue #7)."""

import pytest

from formatparse import ComposedType, ParseResult, compile, composed_type


def test_composed_log_line_nested_parse_result():
    ts = compile("{year:d}-{month:02d}-{day:02d}")
    log = compile(
        "{ts:Timestamp} [{level}] {msg}",
        extra_types={"Timestamp": composed_type(ts)},
    )
    r = log.parse("2024-01-15 [ERROR] oops")
    assert r is not None
    assert r.named["level"] == "ERROR"
    assert r.named["msg"] == "oops"
    inner = r.named["ts"]
    assert isinstance(inner, ParseResult)
    assert inner.named["year"] == 2024
    assert inner.named["month"] == 1
    assert inner.named["day"] == 15


def test_composed_type_exposes_pattern_and_group_count():
    ts = compile("{a:d}")
    c = composed_type(ts)
    assert isinstance(c, ComposedType)
    assert isinstance(c.pattern, str) and len(c.pattern) > 0
    assert isinstance(c.regex_group_count, int)
    assert c.regex_group_count >= 1


def test_wrong_regex_group_count_still_errors_at_parent_compile():
    ts = compile("{x:d}-{y:d}")
    good = composed_type(ts)

    class BadConverter:
        pattern = good.pattern
        regex_group_count = 0

        def __call__(self, text: str) -> None:
            return None

    with pytest.raises(ValueError, match="capturing groups"):
        compile("{z:Bad}", extra_types={"Bad": BadConverter()})
