"""Float width + zero fractional digits (GitHub issue #84, parse#159)."""

from formatparse import parse


def test_float_02_0f_matches_integer_formatted_text():
    """{:02.0f} formats like an integer; parsing must accept the same text."""
    assert parse("foo_{:02d}t", "foo_20t") is not None
    r = parse("foo_{:02.0f}t", "foo_20t")
    assert r is not None
    assert r.fixed[0] == 20.0


def test_float_zero_precision_without_width():
    r = parse("x{:.0f}y", "x42y")
    assert r is not None
    assert r.fixed[0] == 42.0


def test_float_nonzero_precision_still_requires_fraction():
    r = parse("a{:02.2f}b", "a12.34b")
    assert r is not None
    assert abs(r.fixed[0] - 12.34) < 1e-9


def test_float_precision_two_no_width():
    r = parse("{:.2f}", "3.14")
    assert r is not None
    assert abs(r.fixed[0] - 3.14) < 1e-9
