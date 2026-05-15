# coding: utf-8
"""Numeric format tests (moved from test_parse.py)."""

from formatparse import compile, parse


def test_numbers():
    # pull a numbers out of a string
    def y(fmt, s, e, str_equals=False):
        p = compile(fmt)
        r = p.parse(s)
        assert r is not None
        r = r.fixed[0]
        if str_equals:
            assert str(r) == str(e)
        else:
            assert r == e

    def n(fmt, s, e):
        assert parse(fmt, s) is None

    y("a {:d} b", "a 0 b", 0)
    y("a {:d} b", "a 12 b", 12)
    y("a {:5d} b", "a    12 b", 12)
    y("a {:5d} b", "a   -12 b", -12)
    y("a {:d} b", "a -12 b", -12)
    y("a {:d} b", "a +12 b", 12)
    y("a {:d} b", "a  12 b", 12)
    y("a {:d} b", "a 0b1000 b", 8)
    y("a {:d} b", "a 0o1000 b", 512)
    y("a {:d} b", "a 0x1000 b", 4096)
    y("a {:d} b", "a 0xabcdef b", 0xABCDEF)

    y("a {:%} b", "a 100% b", 1)
    y("a {:%} b", "a 50% b", 0.5)
    y("a {:%} b", "a 50.1% b", 0.501)

    y("a {:n} b", "a 100 b", 100)
    y("a {:n} b", "a 1,000 b", 1000)
    y("a {:n} b", "a 1.000 b", 1000)
    y("a {:n} b", "a -1,000 b", -1000)
    y("a {:n} b", "a 10,000 b", 10000)
    y("a {:n} b", "a 100,000 b", 100000)
    n("a {:n} b", "a 100,00 b", None)
    y("a {:n} b", "a 100.000 b", 100000)
    y("a {:n} b", "a 1.000.000 b", 1000000)

    y("a {:f} b", "a 12.0 b", 12.0)
    y("a {:f} b", "a -12.1 b", -12.1)
    y("a {:f} b", "a +12.1 b", 12.1)
    y("a {:f} b", "a .121 b", 0.121)
    y("a {:f} b", "a -.121 b", -0.121)
    n("a {:f} b", "a 12 b", None)

    y("a {:e} b", "a 1.0e10 b", 1.0e10)
    y("a {:e} b", "a .0e10 b", 0.0e10)
    y("a {:e} b", "a 1.0E10 b", 1.0e10)
    y("a {:e} b", "a 1.10000e10 b", 1.1e10)
    y("a {:e} b", "a 1.0e-10 b", 1.0e-10)
    y("a {:e} b", "a 1.0e+10 b", 1.0e10)
    # can't actually test this one on values 'cos nan != nan
    y("a {:e} b", "a nan b", float("nan"), str_equals=True)
    y("a {:e} b", "a NAN b", float("nan"), str_equals=True)
    y("a {:e} b", "a inf b", float("inf"))
    y("a {:e} b", "a +inf b", float("inf"))
    y("a {:e} b", "a -inf b", float("-inf"))
    y("a {:e} b", "a INF b", float("inf"))
    y("a {:e} b", "a +INF b", float("inf"))
    y("a {:e} b", "a -INF b", float("-inf"))

    y("a {:g} b", "a 1 b", 1)
    y("a {:g} b", "a 1e10 b", 1e10)
    y("a {:g} b", "a 1.0e10 b", 1.0e10)
    y("a {:g} b", "a 1.0E10 b", 1.0e10)

    y("a {:b} b", "a 1000 b", 8)
    y("a {:b} b", "a 0b1000 b", 8)
    y("a {:o} b", "a 12345670 b", int("12345670", 8))
    y("a {:o} b", "a 0o12345670 b", int("12345670", 8))
    y("a {:x} b", "a 1234567890abcdef b", 0x1234567890ABCDEF)
    y("a {:x} b", "a 1234567890ABCDEF b", 0x1234567890ABCDEF)
    y("a {:x} b", "a 0x1234567890abcdef b", 0x1234567890ABCDEF)
    y("a {:x} b", "a 0x1234567890ABCDEF b", 0x1234567890ABCDEF)

    y("a {:05d} b", "a 00001 b", 1)
    y("a {:05d} b", "a -00001 b", -1)
    y("a {:05d} b", "a +00001 b", 1)
    y("a {:02d} b", "a 10 b", 10)

    y("a {:=d} b", "a 000012 b", 12)
    y("a {:x=5d} b", "a xxx12 b", 12)
    y("a {:x=5d} b", "a -xxx12 b", -12)

    # Test that hex numbers that ambiguously start with 0b / 0B are parsed correctly
    # See issue #65 (https://github.com/r1chardj0n3s/parse/issues/65)
    y("a {:x} b", "a 0B b", 0xB)
    y("a {:x} b", "a 0B1 b", 0xB1)
    y("a {:x} b", "a 0b b", 0xB)
    y("a {:x} b", "a 0b1 b", 0xB1)

    # Test that number signs are understood correctly
    y("a {:d} b", "a -0o10 b", -8)
    y("a {:d} b", "a -0b1010 b", -10)
    y("a {:d} b", "a -0x1010 b", -0x1010)
    y("a {:o} b", "a -10 b", -8)
    y("a {:b} b", "a -1010 b", -10)
    y("a {:x} b", "a -1010 b", -0x1010)
    y("a {:d} b", "a +0o10 b", 8)
    y("a {:d} b", "a +0b1010 b", 10)
    y("a {:d} b", "a +0x1010 b", 0x1010)
    y("a {:o} b", "a +10 b", 8)
    y("a {:b} b", "a +1010 b", 10)
    y("a {:x} b", "a +1010 b", 0x1010)
