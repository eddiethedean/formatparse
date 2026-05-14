"""Multiline field type ``:ml`` (issues #8, #70)."""

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


def test_compile_ml_with_width():
    p = compile("START\n{body:5ml}\nEND")
    r = p.parse("START\nabcde\nEND")
    assert r is not None
    assert r.named["body"] == "abcde"


def test_ml_width_greedy_between_anchors():
    r = parse("BEGIN\n{body:3ml}\nEND", "BEGIN\nabc\nEND")
    assert r is not None
    assert r.named["body"] == "abc"


def test_ml_alignment_right_strip_leading_spaces():
    r = parse("<<<{body:>ml}>>>", "<<<      hi\nthere>>>")
    assert r is not None
    assert r.named["body"] == "hi\nthere"


def test_ml_alignment_left_trailing_spaces_trimmed():
    r = parse("<<<{body:<ml}>>>", "<<<hi\nthere      >>>")
    assert r is not None
    assert r.named["body"] == "hi\nthere"


def test_ml_alignment_center():
    r = parse("<<<{body:^ml}>>>", "<<<   x\ny   >>>")
    assert r is not None
    assert r.named["body"] == "x\ny"


def test_ml_left_align_with_width_keeps_padding():
    r = parse("value: {body:<10ml}", "value: hi\nthere     ")
    assert r is not None
    assert r.named["body"] == "hi\nthere     "


def test_ml_precision_exact_multiline():
    r = parse("X{body:.5ml}Y", "Xa\nb\ncY")
    assert r is not None
    assert r.named["body"] == "a\nb\nc"


def test_ml_rejects_sign():
    with pytest.raises(ValueError, match="sign"):
        compile("{x:+ml}")


def test_ml_rejects_zero_pad():
    with pytest.raises(ValueError, match="zero"):
        compile("{x:05ml}")


def test_ml_rejects_equals_alignment():
    with pytest.raises(ValueError, match="='"):
        compile("{x:=10ml}")


def test_plain_brace_newline_regression():
    """Unchanged: default ``{}`` between anchors still parses across newlines."""
    r = parse("X{}Y", "Xa\nbY")
    assert r.fixed[0] == "a\nb"
