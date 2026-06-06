# coding: utf-8
from datetime import date
from datetime import datetime
from datetime import time

import pytest

from formatparse import (
    FixedTzOffset,
    Parser,
    RepeatedNameError,
    compile,
    dt_format_to_regex,
    parse,
    search,
)


def test_no_match():
    # string does not match format
    assert parse("{{hello}}", "hello") is None


def test_nothing():
    # do no actual parsing
    r = parse("{{hello}}", "{hello}")
    assert r.fixed == ()
    assert r.named == {}


def test_no_evaluate_result():
    # pull a fixed value out of string
    match = parse("hello {}", "hello world", evaluate_result=False)
    r = match.evaluate_result()
    assert r.fixed == ("world",)


def test_regular_expression():
    # match an actual regular expression
    s = r"^(hello\s[wW]{}!+.*)$"
    e = s.replace("{}", "orld")
    r = parse(s, e)
    assert r.fixed == ("orld",)
    e = s.replace("{}", ".*?")
    r = parse(s, e)
    assert r.fixed == (".*?",)


def test_question_mark():
    # issue9: make sure a ? in the parse string is handled correctly
    r = parse('"{}"?', '"teststr"?')
    assert r[0] == "teststr"


def test_pipe():
    # issue22: make sure a | in the parse string is handled correctly
    r = parse("| {}", "| teststr")
    assert r[0] == "teststr"


def test_unicode():
    # issue29: make sure unicode is parsable
    r = parse("{}", "t€ststr")
    assert r[0] == "t€ststr"


def test_hexadecimal():
    # issue42: make sure bare hexadecimal isn't matched as "digits"
    r = parse("{:d}", "abcdef")
    assert r is None


def test_fixed():
    # pull a fixed value out of string
    r = parse("hello {}", "hello world")
    assert r.fixed == ("world",)


def test_left():
    # pull left-aligned text out of string
    r = parse("{:<} world", "hello       world")
    assert r.fixed == ("hello",)


def test_right():
    # pull right-aligned text out of string
    r = parse("hello {:>}", "hello       world")
    assert r.fixed == ("world",)


def test_center():
    # pull center-aligned text out of string
    r = parse("hello {:^} world", "hello  there     world")
    assert r.fixed == ("there",)


def test_typed():
    # pull a named, typed values out of string
    r = parse("hello {:d} {:w}", "hello 12 people")
    assert r.fixed == (12, "people")
    r = parse("hello {:w} {:w}", "hello 12 people")
    assert r.fixed == ("12", "people")


def test_sign():
    # sign is ignored
    r = parse("Pi = {:.7f}", "Pi = 3.1415926")
    assert r.fixed == (3.1415926,)
    r = parse("Pi = {:+.7f}", "Pi = 3.1415926")
    assert r.fixed == (3.1415926,)
    r = parse("Pi = {:-.7f}", "Pi = 3.1415926")
    assert r.fixed == (3.1415926,)
    r = parse("Pi = {: .7f}", "Pi = 3.1415926")
    assert r.fixed == (3.1415926,)


def test_precision():
    # pull a float out of a string
    r = parse("Pi = {:.7f}", "Pi = 3.1415926")
    assert r.fixed == (3.1415926,)
    r = parse("Pi/10 = {:8.5f}", "Pi/10 =  0.31415")
    assert r.fixed == (0.31415,)
    # float may have not leading zero
    r = parse("Pi/10 = {:8.5f}", "Pi/10 =  .31415")
    assert r.fixed == (0.31415,)
    r = parse("Pi/10 = {:8.5f}", "Pi/10 = -.31415")
    assert r.fixed == (-0.31415,)


def test_custom_type():
    # use a custom type
    r = parse(
        "{:shouty} {:spam}",
        "hello world",
        {"shouty": lambda s: s.upper(), "spam": lambda s: "".join(reversed(s))},
    )
    assert r.fixed == ("HELLO", "dlrow")
    r = parse("{:d}", "12", {"d": lambda s: int(s) * 2})
    assert r.fixed == (24,)
    r = parse("{:d}", "12")
    assert r.fixed == (12,)


def test_typed_fail():
    # pull a named, typed values out of string
    assert parse("hello {:d} {:w}", "hello people 12") is None


def test_named():
    # pull a named value out of string
    r = parse("hello {name}", "hello world")
    assert r.named == {"name": "world"}


def test_named_repeated():
    # test a name may be repeated
    r = parse("{n} {n}", "x x")
    assert r.named == {"n": "x"}


def test_named_repeated_type():
    # test a name may be repeated with type conversion
    r = parse("{n:d} {n:d}", "1 1")
    assert r.named == {"n": 1}


def test_named_repeated_fail_value():
    # test repeated name fails if value mismatches
    r = parse("{n} {n}", "x y")
    assert r is None


def test_named_repeated_type_fail_value():
    # test repeated name with type conversion fails if value mismatches
    r = parse("{n:d} {n:d}", "1 2")
    assert r is None


def test_named_repeated_type_mismatch():
    # test repeated name with mismatched type
    with pytest.raises(RepeatedNameError):
        compile("{n:d} {n:w}")


def test_mixed():
    # pull a fixed and named values out of string
    r = parse("hello {} {name} {} {spam}", "hello world and other beings")
    assert r.fixed == ("world", "other")
    assert r.named == {"name": "and", "spam": "beings"}


def test_named_typed():
    # pull a named, typed values out of string
    r = parse("hello {number:d} {things}", "hello 12 people")
    assert r.named == {"number": 12, "things": "people"}
    r = parse("hello {number:w} {things}", "hello 12 people")
    assert r.named == {"number": "12", "things": "people"}


def test_named_aligned_typed():
    # pull a named, typed values out of string
    r = parse("hello {number:<d} {things}", "hello 12      people")
    assert r.named == {"number": 12, "things": "people"}
    r = parse("hello {number:>d} {things}", "hello      12 people")
    assert r.named == {"number": 12, "things": "people"}
    r = parse("hello {number:^d} {things}", "hello      12      people")
    assert r.named == {"number": 12, "things": "people"}


def test_multiline():
    r = parse("hello\n{}\nworld", "hello\nthere\nworld")
    assert r.fixed[0] == "there"


def test_spans():
    # test the string sections our fields come from
    string = "hello world"
    r = parse("hello {}", string)
    assert r.spans == {0: (6, 11)}
    start, end = r.spans[0]
    assert string[start:end] == r.fixed[0]

    string = "hello     world"
    r = parse("hello {:>}", string)
    assert r.spans == {0: (10, 15)}
    start, end = r.spans[0]
    assert string[start:end] == r.fixed[0]

    string = "hello 0x12 world"
    r = parse("hello {val:x} world", string)
    assert r.spans == {"val": (6, 10)}
    start, end = r.spans["val"]
    assert string[start:end] == "0x%x" % r.named["val"]

    string = "hello world and other beings"
    r = parse("hello {} {name} {} {spam}", string)
    assert r.spans == {0: (6, 11), "name": (12, 15), 1: (16, 21), "spam": (22, 28)}


def test_two_datetimes():
    r = parse("a {:ti} {:ti} b", "a 1997-07-16 2012-08-01 b")
    assert len(r.fixed) == 2
    assert r[0] == datetime(1997, 7, 16)
    assert r[1] == datetime(2012, 8, 1)


def test_flexible_datetimes():
    r = parse("a {:%Y-%m-%d} b", "a 1997-07-16 b")
    assert len(r.fixed) == 1
    assert r[0] == date(1997, 7, 16)

    r = parse("a {:%Y-%b-%d} b", "a 1997-Feb-16 b")
    assert len(r.fixed) == 1
    assert r[0] == date(1997, 2, 16)

    r = parse("a {:%Y-%b-%d} {:d} b", "a 1997-Feb-16 8 b")
    assert len(r.fixed) == 2
    assert r[0] == date(1997, 2, 16)

    r = parse("a {my_date:%Y-%b-%d} {num:d} b", "a 1997-Feb-16 8 b")
    assert (r.named["my_date"]) == date(1997, 2, 16)
    assert (r.named["num"]) == 8

    r = parse("a {:%Y-%B-%d} b", "a 1997-February-16 b")
    assert r[0] == date(1997, 2, 16)

    r = parse("a {:%Y%m%d} b", "a 19970716 b")
    assert r[0] == date(1997, 7, 16)


def test_flexible_datetime_with_colon():
    r = parse("{dt:%Y-%m-%d %H:%M:%S}", "2023-11-21 13:23:27")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27)


def test_datetime_with_various_subsecond_precision():
    r = parse("{dt:%Y-%m-%d %H:%M:%S.%f}", "2023-11-21 13:23:27.123456")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, 123456)

    r = parse("{dt:%Y-%m-%d %H:%M:%S.%f}", "2023-11-21 13:23:27.12345")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, 123450)

    r = parse("{dt:%Y-%m-%d %H:%M:%S.%f}", "2023-11-21 13:23:27.1234")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, 123400)

    r = parse("{dt:%Y-%m-%d %H:%M:%S.%f}", "2023-11-21 13:23:27.123")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, 123000)

    r = parse("{dt:%Y-%m-%d %H:%M:%S.%f}", "2023-11-21 13:23:27.0")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, 0)


def test_flexible_datetime_with_timezone():
    from datetime import timezone

    r = parse("{dt:%Y-%m-%d %H:%M:%S %z}", "2023-11-21 13:23:27 +0000")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, tzinfo=timezone.utc)


def test_flexible_datetime_with_timezone_that_has_colons():
    from datetime import timezone

    r = parse("{dt:%Y-%m-%d %H:%M:%S %z}", "2023-11-21 13:23:27 +00:00:00")
    assert r.named["dt"] == datetime(2023, 11, 21, 13, 23, 27, tzinfo=timezone.utc)


def test_flexible_time():
    r = parse("a {time:%H:%M:%S} b", "a 13:23:27 b")
    assert r.named["time"] == time(13, 23, 27)


def test_flexible_time_no_hour():
    r = parse("a {time:%M:%S} b", "a 23:27 b")
    assert r.named["time"] == time(0, 23, 27)


def test_flexible_time_ms():
    r = parse("a {time:%M:%S:%f} b", "a 23:27:123456 b")
    assert r.named["time"] == time(0, 23, 27, 123456)


def test_flexible_dates_single_digit():
    r = parse("{dt:%Y/%m/%d}", "2023/1/1")
    assert r.named["dt"] == date(2023, 1, 1)


def test_flexible_dates_j():
    r = parse("{dt:%Y/%j}", "2023/9")
    assert r.named["dt"] == date(2023, 1, 9)

    r = parse("{dt:%Y/%j}", "2023/009")
    assert r.named["dt"] == date(2023, 1, 9)


def test_flexible_dates_year_current_year_inferred():
    r = parse("{dt:%j}", "9")
    assert r.named["dt"] == date(datetime.today().year, 1, 9)


def test_datetimes():
    def y(fmt, s, e, tz=None):
        p = compile(fmt)
        r = p.parse(s)
        assert r is not None
        r = r.fixed[0]
        assert r == e
        assert tz is None or r.tzinfo == tz

    utc = FixedTzOffset(0, "UTC")
    assert repr(utc) == "<FixedTzOffset UTC 0:00:00>"
    aest = FixedTzOffset(10 * 60, "+1000")
    tz60 = FixedTzOffset(60, "+01:00")

    # ISO 8660 variants
    # YYYY-MM-DD (eg 1997-07-16)
    y("a {:ti} b", "a 1997-07-16 b", datetime(1997, 7, 16))

    # YYYY-MM-DDThh:mmTZD (eg 1997-07-16T19:20+01:00)
    y("a {:ti} b", "a 1997-07-16 19:20 b", datetime(1997, 7, 16, 19, 20, 0))
    y("a {:ti} b", "a 1997-07-16T19:20 b", datetime(1997, 7, 16, 19, 20, 0))
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20Z b",
        datetime(1997, 7, 16, 19, 20, tzinfo=utc),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20+0100 b",
        datetime(1997, 7, 16, 19, 20, tzinfo=tz60),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20+01:00 b",
        datetime(1997, 7, 16, 19, 20, tzinfo=tz60),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20 +01:00 b",
        datetime(1997, 7, 16, 19, 20, tzinfo=tz60),
    )

    # YYYY-MM-DDThh:mm:ssTZD (eg 1997-07-16T19:20:30+01:00)
    y("a {:ti} b", "a 1997-07-16 19:20:30 b", datetime(1997, 7, 16, 19, 20, 30))
    y("a {:ti} b", "a 1997-07-16T19:20:30 b", datetime(1997, 7, 16, 19, 20, 30))
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30Z b",
        datetime(1997, 7, 16, 19, 20, 30, tzinfo=utc),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30+01:00 b",
        datetime(1997, 7, 16, 19, 20, 30, tzinfo=tz60),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30 +01:00 b",
        datetime(1997, 7, 16, 19, 20, 30, tzinfo=tz60),
    )

    # YYYY-MM-DDThh:mm:ss.sTZD (eg 1997-07-16T19:20:30.45+01:00)
    y(
        "a {:ti} b",
        "a 1997-07-16 19:20:30.500000 b",
        datetime(1997, 7, 16, 19, 20, 30, 500000),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30.500000 b",
        datetime(1997, 7, 16, 19, 20, 30, 500000),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30.5Z b",
        datetime(1997, 7, 16, 19, 20, 30, 500000, tzinfo=utc),
    )
    y(
        "a {:ti} b",
        "a 1997-07-16T19:20:30.5+01:00 b",
        datetime(1997, 7, 16, 19, 20, 30, 500000, tzinfo=tz60),
    )

    aest_d = datetime(2011, 11, 21, 10, 21, 36, tzinfo=aest)
    dt = datetime(2011, 11, 21, 10, 21, 36)
    dt00 = datetime(2011, 11, 21, 10, 21)
    d = datetime(2011, 11, 21)

    # te   RFC2822 e-mail format        datetime
    y("a {:te} b", "a Mon, 21 Nov 2011 10:21:36 +1000 b", aest_d)
    y("a {:te} b", "a Mon, 21 Nov 2011 10:21:36 +10:00 b", aest_d)
    y("a {:te} b", "a 21 Nov 2011 10:21:36 +1000 b", aest_d)

    # tg   global (day/month) format datetime
    y("a {:tg} b", "a 21/11/2011 10:21:36 AM +1000 b", aest_d)
    y("a {:tg} b", "a 21/11/2011 10:21:36 AM +10:00 b", aest_d)
    y("a {:tg} b", "a 21-11-2011 10:21:36 AM +1000 b", aest_d)
    y("a {:tg} b", "a 21/11/2011 10:21:36 +1000 b", aest_d)
    y("a {:tg} b", "a 21/11/2011 10:21:36 b", dt)
    y("a {:tg} b", "a 21/11/2011 10:21 b", dt00)
    y("a {:tg} b", "a 21-11-2011 b", d)
    y("a {:tg} b", "a 21-Nov-2011 10:21:36 AM +1000 b", aest_d)
    y("a {:tg} b", "a 21-November-2011 10:21:36 AM +1000 b", aest_d)

    # ta   US (month/day) format     datetime
    y("a {:ta} b", "a 11/21/2011 10:21:36 AM +1000 b", aest_d)
    y("a {:ta} b", "a 11/21/2011 10:21:36 AM +10:00 b", aest_d)
    y("a {:ta} b", "a 11-21-2011 10:21:36 AM +1000 b", aest_d)
    y("a {:ta} b", "a 11/21/2011 10:21:36 +1000 b", aest_d)
    y("a {:ta} b", "a 11/21/2011 10:21:36 b", dt)
    y("a {:ta} b", "a 11/21/2011 10:21 b", dt00)
    y("a {:ta} b", "a 11-21-2011 b", d)
    y("a {:ta} b", "a Nov-21-2011 10:21:36 AM +1000 b", aest_d)
    y("a {:ta} b", "a November-21-2011 10:21:36 AM +1000 b", aest_d)
    y("a {:ta} b", "a November-21-2011 b", d)

    # ts   Linux System log format        datetime
    y(
        "a {:ts} b",
        "a Nov 21 10:21:36 b",
        datetime(datetime.today().year, 11, 21, 10, 21, 36),
    )
    y(
        "a {:ts} b",
        "a Nov  1 10:21:36 b",
        datetime(datetime.today().year, 11, 1, 10, 21, 36),
    )
    y(
        "a {:ts} b",
        "a Nov  1 03:21:36 b",
        datetime(datetime.today().year, 11, 1, 3, 21, 36),
    )

    # th   HTTP log format date/time                   datetime
    y("a {:th} b", "a 21/Nov/2011:10:21:36 +1000 b", aest_d)
    y("a {:th} b", "a 21/Nov/2011:10:21:36 +10:00 b", aest_d)

    d = datetime(2011, 11, 21, 10, 21, 36)

    # tc   ctime() format           datetime
    y("a {:tc} b", "a Mon Nov 21 10:21:36 2011 b", d)

    t530 = FixedTzOffset(-5 * 60 - 30, "-5:30")
    t830 = FixedTzOffset(-8 * 60 - 30, "-8:30")

    # tt   Time                                        time
    y("a {:tt} b", "a 10:21:36 AM +1000 b", time(10, 21, 36, tzinfo=aest))
    y("a {:tt} b", "a 10:21:36 AM +10:00 b", time(10, 21, 36, tzinfo=aest))
    y("a {:tt} b", "a 10:21:36 AM b", time(10, 21, 36))
    y("a {:tt} b", "a 10:21:36 PM b", time(22, 21, 36))
    y("a {:tt} b", "a 10:21:36 b", time(10, 21, 36))
    y("a {:tt} b", "a 10:21 b", time(10, 21))
    y("a {:tt} b", "a 10:21:36 PM -5:30 b", time(22, 21, 36, tzinfo=t530))
    y("a {:tt} b", "a 10:21:36 PM -530 b", time(22, 21, 36, tzinfo=t530))
    y("a {:tt} b", "a 10:21:36 PM -05:30 b", time(22, 21, 36, tzinfo=t530))
    y("a {:tt} b", "a 10:21:36 PM -0530 b", time(22, 21, 36, tzinfo=t530))
    y("a {:tt} b", "a 10:21:36 PM -08:30 b", time(22, 21, 36, tzinfo=t830))
    y("a {:tt} b", "a 10:21:36 PM -0830 b", time(22, 21, 36, tzinfo=t830))


def test_datetime_group_count():
    # test we increment the group count correctly for datetimes
    r = parse("{:ti} {}", "1972-01-01 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:tg} {}", "1-1-1972 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:ta} {}", "1-1-1972 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:th} {}", "21/Nov/2011:10:21:36 +1000 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:te} {}", "21 Nov 2011 10:21:36 +1000 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:tc} {}", "Mon Nov 21 10:21:36 2011 spam")
    assert r.fixed[1] == "spam"
    r = parse("{:tt} {}", "10:21 spam")
    assert r.fixed[1] == "spam"


def test_mixed_types():
    # stress-test: pull one of everything out of a string
    r = parse(
        """
        letters: {:w}
        non-letters: {:W}
        whitespace: "{:s}"
        non-whitespace: \t{:S}\n
        digits: {:d} {:d}
        non-digits: {:D}
        numbers with thousands: {:n}
        fixed-point: {:f}
        floating-point: {:e}
        general numbers: {:g} {:g}
        binary: {:b}
        octal: {:o}
        hex: {:x}
        ISO 8601 e.g. {:ti}
        RFC2822 e.g. {:te}
        Global e.g. {:tg}
        US e.g. {:ta}
        ctime() e.g. {:tc}
        HTTP e.g. {:th}
        time: {:tt}
        final value: {}
    """,
        """
        letters: abcdef_GHIJLK
        non-letters: !@#%$ *^%
        whitespace: "   \t\n"
        non-whitespace: \tabc\n
        digits: 12345 0b1011011
        non-digits: abcdef
        numbers with thousands: 1,000
        fixed-point: 100.2345
        floating-point: 1.1e-10
        general numbers: 1 1.1
        binary: 0b1000
        octal: 0o1000
        hex: 0x1000
        ISO 8601 e.g. 1972-01-20T10:21:36Z
        RFC2822 e.g. Mon, 20 Jan 1972 10:21:36 +1000
        Global e.g. 20/1/1972 10:21:36 AM +1:00
        US e.g. 1/20/1972 10:21:36 PM +10:30
        ctime() e.g. Sun Sep 16 01:03:52 1973
        HTTP e.g. 21/Nov/2011:00:07:11 +0000
        time: 10:21:36 PM -5:30
        final value: spam
    """,
    )
    assert r is not None
    assert r.fixed[22] == "spam"


def test_mixed_type_variant():
    r = parse(
        """
        letters: {:w}
        non-letters: {:W}
        whitespace: "{:s}"
        non-whitespace: \t{:S}\n
        digits: {:d}
        non-digits: {:D}
        numbers with thousands: {:n}
        fixed-point: {:f}
        floating-point: {:e}
        general numbers: {:g} {:g}
        binary: {:b}
        octal: {:o}
        hex: {:x}
        ISO 8601 e.g. {:ti}
        RFC2822 e.g. {:te}
        Global e.g. {:tg}
        US e.g. {:ta}
        ctime() e.g. {:tc}
        HTTP e.g. {:th}
        time: {:tt}
        final value: {}
    """,
        """
        letters: abcdef_GHIJLK
        non-letters: !@#%$ *^%
        whitespace: "   \t\n"
        non-whitespace: \tabc\n
        digits: 0xabcdef
        non-digits: abcdef
        numbers with thousands: 1.000.000
        fixed-point: 0.00001
        floating-point: NAN
        general numbers: 1.1e10 nan
        binary: 0B1000
        octal: 0O1000
        hex: 0X1000
        ISO 8601 e.g. 1972-01-20T10:21:36Z
        RFC2822 e.g. Mon, 20 Jan 1972 10:21:36 +1000
        Global e.g. 20/1/1972 10:21:36 AM +1:00
        US e.g. 1/20/1972 10:21:36 PM +10:30
        ctime() e.g. Sun Sep 16 01:03:52 1973
        HTTP e.g. 21/Nov/2011:00:07:11 +0000
        time: 10:21:36 PM -5:30
        final value: spam
    """,
    )
    assert r is not None
    assert r.fixed[21] == "spam"


def test_too_many_fields_rejected_at_compile():
    """Patterns with more than MAX_FIELDS (100) fields are rejected at compile time."""
    pattern = "".join(f"{{{i}:d}}" for i in range(101))
    with pytest.raises(ValueError, match="more than 100 fields"):
        compile(pattern)


def test_letters():
    res = parse("{:l}", "")
    assert res is None
    res = parse("{:l}", "sPaM")
    assert res.fixed == ("sPaM",)
    res = parse("{:l}", "sP4M")
    assert res is None
    res = parse("{:l}", "sP_M")
    assert res is None


def test_strftime_strptime_roundtrip():
    dt = datetime.now()
    fmt = "_".join([k for k in dt_format_to_regex if k != "%z"])
    s = dt.strftime(fmt)
    [res] = parse("{:" + fmt + "}", s)
    assert res == dt


def test_parser_format():
    parser = compile("hello {}")
    assert parser.format.format("world") == "hello world"
    with pytest.raises(AttributeError):
        parser.format = "hi {}"


def test_hyphen_inside_field_name():
    # https://github.com/r1chardj0n3s/parse/issues/86
    # https://github.com/python-openapi/openapi-core/issues/672
    template = "/local/sub/{user-id}/duration"
    assert Parser(template).named_fields == ["user_id"]
    string = "https://dummy_server.com/local/sub/1647222638/duration"
    result = search(template, string)
    assert result["user-id"] == "1647222638"


def test_hyphen_inside_field_name_collision_handling():
    template = "/foo/{user-id}/{user_id}/{user.id}/bar/"
    assert Parser(template).named_fields == ["user_id", "user__id", "user___id"]
    string = "/foo/1/2/3/bar/"
    result = search(template, string)
    assert result["user-id"] == "1"
    assert result["user_id"] == "2"
    assert result["user.id"] == "3"
