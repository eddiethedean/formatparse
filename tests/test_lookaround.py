"""Regex lookaround assertions after integer/float type tokens (issues #9 / parse#209)."""

import pytest

from formatparse import compile, parse


def test_integer_positive_lookahead_px():
    r = parse("{v:d(?=px)}", "100px")
    assert r is not None
    assert r.named["v"] == 100
    assert r.spans["v"] == (0, 3)


def test_integer_positive_lookbehind_dollar():
    r = parse(r"{v:d(?<=\$)}", "$100")
    assert r is not None
    assert r.named["v"] == 100
    assert r.spans["v"] == (1, 4)


def test_integer_negative_lookahead():
    assert parse("{v:d(?!px)}", "100") is not None
    assert parse("{v:d(?!px)}", "100px") is None


def test_integer_negative_lookbehind():
    assert parse(r"{v:d(?<!A)}", "12") is not None
    assert parse(r"{v:d(?<!A)}", "A12") is None


def test_float_positive_lookahead():
    r = parse("{w:f(?=kg)}", "12.5kg")
    assert r is not None
    assert abs(r.named["w"] - 12.5) < 1e-9
    start, end = r.spans["w"]
    assert "12.5kg"[start:end] == "12.5"


def test_lookbehind_then_lookahead_order():
    r = parse(r"{v:d(?<=\$)(?=px)}", "$99px")
    assert r is not None
    assert r.named["v"] == 99


def test_compile_rejects_capturing_group_inside_lookaround():
    with pytest.raises(ValueError, match="capturing"):
        compile(r"{v:d(?=([0-9]))}")


def test_strftime_rejects_trailing_lookaround():
    with pytest.raises(ValueError, match="strftime"):
        compile(r"{t:%Y(?=x)}")
