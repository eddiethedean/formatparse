"""Integer (and radix) fields with both width and precision (GitHub #82, parse#107)."""

from formatparse import parse


def test_issue82_hex_white_six_nibbles_three_fields():
    r = parse("#{:2.2x}{:2.2x}{:2.2x}", "#FFFFFF")
    assert r is not None
    assert r.fixed == (255, 255, 255)


def test_issue82_decimal_six_digits_three_fields():
    r = parse("{:2.2d}{:2.2d}{:2.2d}", "123456")
    assert r is not None
    assert r.fixed == (12, 34, 56)


def test_issue82_decimal_four_digits_insufficient_for_three_fields():
    assert parse("{:2.2d}{:2.2d}{:2.2d}", "9999") is None


def test_issue82_hex_four_nibbles_insufficient_for_three_fields():
    assert parse("#{:2.2x}{:2.2x}{:2.2x}", "#FFFF") is None


def test_issue82_hex_three_nibbles_insufficient_for_three_fields():
    assert parse("#{:2.2x}{:2.2x}{:2.2x}", "#FFF") is None


def test_issue82_binary_adjacent_bounded():
    r = parse("{:2.2b}{:2.2b}", "1111")
    assert r is not None
    assert r.fixed == (3, 3)


def test_issue82_octal_adjacent_bounded():
    r = parse("{:2.2o}{:2.2o}", "7777")
    assert r is not None
    assert r.fixed == (63, 63)


def test_issue82_zero_pad_width_and_precision():
    r = parse("v:{n:02.2d}", "v:99")
    assert r is not None
    assert r.named["n"] == 99
    assert parse("v:{n:02.2d}", "v:999") is None


def test_issue82_width_2_precision_5_allows_three_digit_decimal():
    r = parse("n:{n:2.5d}", "n:999")
    assert r is not None
    assert r.named["n"] == 999


def test_issue82_regression_leading_space_decimal_still_allowed():
    r = parse("x:{a:2.2d}", "x:  99")
    assert r is not None
    assert r.named["a"] == 99
