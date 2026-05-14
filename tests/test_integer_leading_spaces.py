"""Integer {:d} with leading whitespace before digits (GitHub issue #81, parse#133)."""

from formatparse import parse


def test_decimal_d_allows_multiple_leading_spaces_before_zero():
    assert parse("{a:d}", "0").named["a"] == 0
    assert parse("{a:d}", " 0").named["a"] == 0
    assert parse("{a:d}", "   0").named["a"] == 0
    assert parse("{a:d}", "    0").named["a"] == 0


def test_decimal_d_leading_spaces_before_nonzero():
    assert parse("{a:d}", "   42").named["a"] == 42


def test_decimal_d_tabs_and_spaces():
    r = parse("{a:d}", "  \t 9")
    assert r is not None
    assert r.named["a"] == 9


def test_hex_field_unprefixed_still_parses():
    r = parse("{h:x}", "ff")
    assert r is not None
    assert r.named["h"] == 255


def test_literal_then_field_still_requires_digits_after_literal():
    r = parse("x{a:d}", "x  3")
    assert r is not None
    assert r.named["a"] == 3
