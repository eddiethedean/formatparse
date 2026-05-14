"""Brace-delimited payload in the input string (issue #15 / parse#146)."""

import pytest

from formatparse import compile, parse


def test_brace_empty_inner():
    line = "v:1 t:CON c:PUT i:cdcb {} [ Observe:0 ]"
    pat = "v:1 t:CON c:PUT i:cdcb {payload:brace} [ Observe:0 ]"
    r = parse(pat, line)
    assert r is not None
    assert r.named["payload"] == ""


def test_brace_inner_text():
    line = "v:1 t:CON c:PUT i:cdcb {telemetry} [ Observe:0 ]"
    pat = "v:1 t:CON c:PUT i:cdcb {payload:brace} [ Observe:0 ]"
    r = parse(pat, line)
    assert r is not None
    assert r.named["payload"] == "telemetry"


def test_brace_stops_at_first_close_when_suffix_matches():
    r = parse("a{x:brace}z", "a{inner}z")
    assert r is not None
    assert r.named["x"] == "inner"


def test_brace_with_later_braces_backtracks_to_close_before_suffix():
    # Literal "b" after "}" forces the match to use the "}" that still allows "b" to match.
    r = parse("a{x:brace}b", "a{in}side}b")
    assert r is not None
    assert r.named["x"] == "in}side"


def test_brace_requires_name():
    with pytest.raises(ValueError, match=":brace format requires"):
        compile("{:brace}")


def test_brace_rejects_numbered_field():
    with pytest.raises(ValueError, match=":brace format cannot"):
        compile("{0:brace}")


def test_brace_search():
    from formatparse import search

    s = search("x={v:brace}y", "prefix x={hello}y suffix")
    assert s is not None
    assert s.named["v"] == "hello"


def test_brace_findall():
    from formatparse import findall

    xs = list(findall("{v:brace}", "{a}{b}"))
    assert [m.named["v"] for m in xs] == ["a", "b"]
