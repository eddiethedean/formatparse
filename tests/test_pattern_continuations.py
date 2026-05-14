"""Pattern backslash line continuations (GitHub issue #68)."""

from formatparse import compile, parse, search


def _assert_same_parse(pattern_a: str, pattern_b: str, text: str) -> None:
    """Compare parse outcomes; ``ParseResult`` does not implement value equality."""
    ra, rb = parse(pattern_a, text), parse(pattern_b, text)
    assert (ra is None) == (rb is None)
    if ra is None:
        return
    assert ra.span == rb.span
    assert dict(ra.named) == dict(rb.named)
    assert tuple(ra.fixed) == tuple(rb.fixed)


def test_parse_continued_pattern_matches_single_line():
    continued = "Hello, \\\n{name}!"
    single = "Hello, {name}!"
    _assert_same_parse(continued, single, "Hello, world!")


def test_parse_crlf_continuation():
    continued = "x\\\r\ny"
    assert parse(continued, "xy") is not None


def test_double_backslash_keeps_newline_in_pattern():
    # Even number of backslashes before newline: newline stays in the pattern.
    pat = "a\\\\\nb"
    p = compile(pat)
    assert p.pattern == pat
    text = "a\\\\\nb"
    assert p.parse(text) is not None


def test_triple_backslash_continuation():
    continued = "a\\\\\\\nb"
    single = "a\\b"
    _assert_same_parse(continued, single, "a\\b")


def test_continued_line_leading_whitespace_stripped():
    continued = "foo\\\n    bar"
    assert parse(continued, "foobar") is not None


def test_ml_field_equivalent_single_line():
    # Blank line after the continuation keeps a literal newline before ``{body:ml}`` (``\\\n`` drops one).
    continued = "BEGIN\\\n\n{body:ml}\nEND"
    single = "BEGIN\n{body:ml}\nEND"
    text = "BEGIN\na\nb\nEND"
    _assert_same_parse(continued, single, text)


def test_search_continuation():
    continued = "foo\\\n{name}"
    single = "foo{name}"
    assert search(continued, "foox", evaluate_result=False) is not None
    assert search(single, "foox", evaluate_result=False) is not None


def test_compile_stores_normalized_pattern():
    p = compile("a\\\nb")
    assert p.pattern == "ab"
