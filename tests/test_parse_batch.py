"""Tests for parse_batch (issue #14)."""

import pytest

from formatparse import parse, parse_batch, with_pattern


def test_parse_batch_empty():
    assert parse_batch("{:d}", []) == []


def test_parse_batch_all_match():
    out = parse_batch("{:d}-{:d}", ["1-2", "3-4"])
    assert len(out) == 2
    assert out[0] is not None and out[0].fixed == (1, 2)
    assert out[1] is not None and out[1].fixed == (3, 4)


def test_parse_batch_mixed_none():
    out = parse_batch("{:d}", ["1", "x", "3"])
    assert out[0] is not None and out[0].fixed[0] == 1
    assert out[1] is None
    assert out[2] is not None and out[2].fixed[0] == 3


def test_parse_batch_tuple_input():
    out = parse_batch("{:d}", ("10", "20"))
    assert [r.fixed[0] if r else None for r in out] == [10, 20]


def _assert_parse_result_equal(a, b):
    assert (a is None) == (b is None)
    if a is None:
        return
    assert a.named == b.named
    assert a.fixed == b.fixed


def test_parse_batch_parity_with_parse():
    pattern = "{name}: {age:d}"
    strings = ["Alice: 30", "nope", "Bob: 25"]
    batch = parse_batch(pattern, strings)
    for s, b in zip(strings, batch):
        _assert_parse_result_equal(b, parse(pattern, s))


def test_parse_batch_extra_types():
    @with_pattern(r"\d+")
    def as_int(text):
        return int(text)

    extra = {"N": as_int}
    strings = ["v: 1", "v: 2", "v: x"]
    batch = parse_batch("v: {:N}", strings, extra_types=extra)
    singles = [parse("v: {:N}", s, extra_types=extra) for s in strings]
    assert len(batch) == len(singles)
    for b, s in zip(batch, singles):
        _assert_parse_result_equal(b, s)


def test_parse_batch_case_sensitive():
    pattern = "Hello, {name}"
    strings = ["Hello, Alice", "hello, Bob"]
    ins = parse_batch(pattern, strings, case_sensitive=True)
    assert ins[0] is not None and ins[0].named["name"] == "Alice"
    assert ins[1] is None

    outs = parse_batch(pattern, strings, case_sensitive=False)
    assert outs[0] is not None and outs[0].named["name"] == "Alice"
    assert outs[1] is not None and outs[1].named["name"] == "Bob"


def test_parse_batch_evaluate_result_false():
    pattern = "hello {}"
    strings = ["hello world", "hello there"]
    batch = parse_batch(pattern, strings, evaluate_result=False)
    singles = [parse(pattern, s, evaluate_result=False) for s in strings]
    assert len(batch) == 2
    for b, s in zip(batch, singles):
        assert b is not None
        assert s is not None
        assert b.evaluate_result().fixed == s.evaluate_result().fixed


def test_parse_batch_null_byte_in_string_raises():
    with pytest.raises(ValueError, match="null byte"):
        parse_batch("{:d}", ["1", "bad\x00"])


def test_parse_batch_smoke_many():
    n = 500
    strings = [str(i) for i in range(n)]
    out = parse_batch("{:d}", strings)
    assert len(out) == n
    for i, r in enumerate(out):
        assert r is not None and r.fixed[0] == i
