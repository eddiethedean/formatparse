"""Indent-block field type ``:blk`` (GitHub issue #69).

Avoid a newline immediately before ``{...:blk}`` in the *same* literal chunk: literals
that end with trailing whitespace are compiled as ``\\s+``, which would consume the
block's leading indentation. Prefer ``key:{body:blk}`` (then ``\\n`` / ``END`` in later
literals) so the first line of the capture participates in the margin.
"""

import pytest

from formatparse import compile, parse


def test_blk_strips_common_indent():
    r = parse("key:{body:blk}\nEND", "key:\n  line1\n  line2\nEND")
    assert r.named["body"] == "line1\nline2"


def test_blk_blank_lines_in_block():
    r = parse("H:{body:blk}\nX", "H:\n  a\n\n  b\nX")
    assert r.named["body"] == "a\n\nb"


def test_blk_crlf_input():
    r = parse("H:{body:blk}\r\nX", "H:\r\n  a\r\n  b\r\nX")
    assert r.named["body"] == "a\nb"


def test_blk_tabs_in_indent():
    r = parse("H:{body:blk}\nX", "H:\n\tx\n\ty\nX")
    assert r.named["body"] == "x\ny"


def test_blk_same_boundaries_as_ml():
    text = "BEGIN\n  one\n  two\nEND"
    ml = parse("BEGIN{body:ml}\nEND", text)
    blk = parse("BEGIN{body:blk}\nEND", text)
    assert ml.named["body"] == "\n  one\n  two"
    assert blk.named["body"] == "one\ntwo"


def test_compile_blk():
    p = compile("{x:blk}")
    r = p.parse("\n  hi\n  there")
    assert r.named["x"] == "hi\nthere"


def test_blk_rejects_sign():
    with pytest.raises(ValueError, match=":blk"):
        compile("{x:+blk}")


def test_blk_rejects_equals_alignment():
    with pytest.raises(ValueError, match=":blk"):
        compile("{x:=10blk}")


def test_blk_input_line_continuation_then_dedent_issue80():
    """Input continuations run before :blk dedent (issue #80)."""
    r = parse("BEGIN\n{body:blk}\nEND", "BEGIN\n  x\\\n  y\nEND")
    assert r.named["body"] == "xy"
