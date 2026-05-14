"""Multiline field type ``:ml`` (GitHub issue #8)."""

import pytest

from formatparse import compile, parse


def test_multiline_named_field_lf():
    r = parse("BEGIN\n{body:ml}\nEND", "BEGIN\nalpha\nbeta\nEND")
    assert r.named["body"] == "alpha\nbeta"


def test_multiline_named_field_crlf():
    r = parse("BEGIN\r\n{body:ml}\r\nEND", "BEGIN\r\nalpha\r\nbeta\r\nEND")
    assert r.named["body"] == "alpha\r\nbeta"


def test_multiline_then_literal_field():
    r = parse("{a:ml}\n---\n{b}", "first\nblock\n---\nrest")
    assert r.named["a"] == "first\nblock"
    assert r.named["b"] == "rest"


def test_compile_ml_pattern():
    p = compile("{body:ml}")
    r = p.parse("a\nb")
    assert r.named["body"] == "a\nb"


def test_ml_rejects_width():
    with pytest.raises(ValueError, match="Multiline"):
        compile("{x:10ml}")


def test_plain_brace_newline_regression():
    """Unchanged: default ``{}`` between anchors still parses across newlines."""
    r = parse("X{}Y", "Xa\nbY")
    assert r.fixed[0] == "a\nb"
