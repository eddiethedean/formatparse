"""Tests for findall_iter / FindallIter (issue #13 MVP)."""

import pytest

from formatparse import ParseResult, compile, findall, findall_iter, with_pattern


def _assert_parse_like_equal(a, b):
    """Compare ParseResult-like objects (after Match.evaluate_result if needed)."""
    if not isinstance(a, ParseResult):
        a = a.evaluate_result()
    if not isinstance(b, ParseResult):
        b = b.evaluate_result()
    assert a.named == b.named
    assert a.fixed == b.fixed


def test_findall_iter_empty_string():
    assert list(findall_iter("ID:{id:d}", "")) == []


def test_findall_iter_no_matches():
    assert list(findall_iter("ID:{id:d}", "nothing")) == []


def test_findall_iter_parity_fast_path_many():
    pattern = "ID:{id:d}"
    text = " ".join(f"ID:{i}" for i in range(50))
    a = list(findall(pattern, text))
    b = list(findall_iter(pattern, text))
    assert len(a) == len(b) == 50
    for x, y in zip(a, b):
        _assert_parse_like_equal(x, y)


def test_findall_iter_parity_fast_path_extract():
    s = "".join(r.fixed[0] for r in findall_iter(">{}<", "<p>some <b>bold</b> text</p>"))
    assert s == "some bold text"


def test_findall_iter_parity_extra_types():
    @with_pattern(r"\d+")
    def as_int(text):
        return int(text)

    pattern = "v:{:N}"
    text = "v:1 v:2 v:3"
    extra = {"N": as_int}
    a = list(findall(pattern, text, extra_types=extra))
    b = list(findall_iter(pattern, text, extra_types=extra))
    assert len(a) == len(b) == 3
    for x, y in zip(a, b):
        _assert_parse_like_equal(x, y)


def test_findall_iter_evaluate_result_false_parity():
    pattern = ">{}<"
    text = "<p>a</p> <p>b</p>"
    a = list(findall(pattern, text, evaluate_result=False))
    b = list(findall_iter(pattern, text, evaluate_result=False))
    assert len(a) == len(b)
    for x, y in zip(a, b):
        _assert_parse_like_equal(x, y)


def test_findall_iter_null_byte_raises():
    with pytest.raises(ValueError, match="null byte"):
        list(findall_iter("{}", "a\x00b"))


def test_format_parser_findall_iter_parity():
    pattern = "ID:{id:d}"
    parser = compile(pattern)
    text = "ID:1 ID:2"
    a = list(findall(pattern, text))
    b = list(parser.findall_iter(text))
    assert len(a) == len(b) == 2
    for x, y in zip(a, b):
        _assert_parse_like_equal(x, y)
