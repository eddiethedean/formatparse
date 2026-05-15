"""Tests for field alignment with precision validation

These tests cover alignment + precision rules (issue #3 / parse#218) and
parse parity for **zero-filled, right-aligned** string fields when width,
precision, and the captured slice length are all equal (issue #40): ``0``
padding cannot be distinguished from content, matching the original ``parse``
library for those cases.
"""

from formatparse import parse


def test_issue40_zero_fill_right_align_width_precision():
    """Regression: parse parity for {s:0>18.18} (GitHub issue #40)."""
    result = parse("{s:0>18.18}", "000000000000100000")
    assert result is not None
    assert result.named["s"] == "000000000000100000"

    # Shorter pattern: capture must stay zero-padded (not stripped to "xx")
    result = parse("{s:0>4.4}", "00xx")
    assert result is not None
    assert result.named["s"] == "00xx"


def test_right_aligned_precision_invalid_both_sides():
    """Over-long input still does not match the anchored pattern."""
    result = parse("{s:>4.4}", " aaa ")
    assert result is None


def test_right_aligned_precision_invalid_extra_char():
    """Test that right-aligned precision rejects when fill enables extra char"""
    # Should fail: one fill char enables extra char (exceeds width)
    # {s:>4.4} means width=4, precision=4, so total must be <= 4
    # " aaaa" has 5 chars (1 space + 4 content), which exceeds width 4
    result = parse("{s:>4.4}", " aaaa")
    assert result is None, "Should reject when total width exceeds specified width"


def test_left_aligned_precision_invalid_too_many_fills():
    """Test that left-aligned precision rejects too many fill chars"""
    # Should fail: too many fill chars (spaces) after the content
    # {s:<4.4} means width=4, precision=4, so total must be <= 4
    # "aaaa                    " has many chars, which exceeds width 4
    result = parse("{s:<4.4}", "aaaa                    ")
    assert result is None, "Should reject when total width exceeds specified width"


def test_right_aligned_precision_valid():
    """Test that right-aligned precision accepts valid cases"""
    # Valid: no padding, exactly precision chars (total = width = precision)
    result = parse("{s:>4.4}", "aaaa")
    assert result is not None
    assert result.named["s"] == "aaaa"

    # Note: "{s:>4.4}" with " aaa" (4 chars: 1 space + 3 content) matches as a full 4-char cell


def test_left_aligned_precision_valid():
    """Test that left-aligned precision accepts valid cases"""
    # Valid: no padding, exactly precision chars (total = width = precision)
    result = parse("{s:<4.4}", "aaaa")
    assert result is not None
    assert result.named["s"] == "aaaa"

    # Note: "{s:<4.4}" with "aaaa " (5 chars: 4 content + 1 space) exceeds width 4
    # This is correctly rejected by validation


def test_left_aligned_precision_preserves_trailing_spaces_issue_39():
    """Trailing spaces inside width/precision are content, not padding (parse parity)."""
    result = parse("{s:<15.15}", "xxxxxxxxxx     ")
    assert result is not None
    assert result.named["s"] == "xxxxxxxxxx     "
    assert len(result.named["s"]) == 15


def test_center_aligned_precision():
    """Test center-aligned precision validation"""
    # Valid: no padding, exactly precision chars (total = width = precision)
    result = parse("{s:^4.4}", "aaaa")
    assert result is not None
    assert result.named["s"] == "aaaa"

    # Invalid: too many chars (exceeds width)
    result = parse("{s:^4.4}", " aaaa ")
    assert result is None, "Should reject when total width exceeds specified width"

    # Invalid: content exceeds precision
    # Note: The regex pattern limits matches, so this might be handled by regex
    result = parse("{s:^4.4}", " aaaa ")
    assert result is None, "Should reject when content exceeds precision"


def test_alignment_precision_with_fill_character():
    """Test alignment with precision and custom fill characters"""
    # Valid: dot fill, right-aligned, exact precision (no fill chars, just content)
    # Note: When width == precision, the pattern requires exactly precision chars
    result = parse("{s:.>4.4}", "aaaa")
    assert result is not None
    assert result.named["s"] == "aaaa"

    # Invalid: fill char on both sides - formatparse rejects (parse accepts; we stay stricter here)
    result = parse("{s:.>4.4}", ".aa.")
    assert result is None, "Should reject fill character on both sides"

    # Valid: dot fill, left-aligned, exact precision (no fill chars, just content)
    result = parse("{s:.<4.4}", "aaaa")
    assert result is not None
    assert result.named["s"] == "aaaa"


def test_alignment_precision_field_boundaries():
    """Test that alignment+precision doesn't affect following fields"""
    # This was the main concern: field boundaries should be correct
    # When width == precision, no fill chars are allowed, so "aaaa" is valid
    # Use a simpler second field to avoid validation edge cases with zero-padding
    result = parse("{s:<4.4}{n:d}", "aaaa42")
    assert result is not None
    assert result.named["s"] == "aaaa"
    assert result.named["n"] == 42

    # Invalid: first field exceeds width, should fail
    result = parse("{s:<4.4}{n:d}", "aaaaa42")
    assert result is None, "Should reject when first field exceeds width"


def test_precision_without_alignment():
    """Test precision without alignment (should work normally)"""
    # Precision without alignment should work
    result = parse("{s:.4}", "abcd")
    assert result is not None
    assert result.named["s"] == "abcd"

    # Exceeds precision
    result = parse("{s:.4}", "abcde")
    assert result is None, "Should reject when exceeds precision"


def test_alignment_without_precision():
    """Test alignment without precision (should work normally)"""
    # Alignment without precision should work
    result = parse("{s:>10}", "     hello")
    assert result is not None
    assert result.named["s"] == "hello"

    result = parse("{s:<10}", "hello     ")
    assert result is not None
    assert result.named["s"] == "hello     "


def test_issue88_aligned_string_then_zero_padded_int():
    """Regression: right/zero-filled width+precision string + ``{:02d}`` (GitHub issue #88)."""
    r = parse("{n:>10.10}{x:02d}", "000001000099")
    assert r is not None
    assert r.named["n"] == "0000010000"
    assert r.named["x"] == 99

    r = parse("{n:>10.10}{x:02d}", "     1000099")
    assert r is not None
    assert r.named["x"] == 99
    assert r.named["n"] == "10000"

    r = parse("{n:0>10.10}{x:02d}", "000001000099")
    assert r is not None
    assert r.named["n"] == "0000010000"
    assert r.named["x"] == 99

    r = parse("{s:<10.10}{n:0>10.10}{x:02d}", "0000bbbb  000001000099")
    assert r is not None
    assert r.named["s"] == "0000bbbb  "
    assert r.named["n"] == "0000010000"
    assert r.named["x"] == 99


def test_issue95_leading_space_trailing_lf_after_width_precision_string():
    """Regression: literal newline after field must not be eaten by DOTALL `.` (issue #95)."""
    r = parse(" {s:<4.4}\n", "     \n")
    assert r is not None
    assert r.span == (0, 6)
    assert r.named["s"] == "    "
