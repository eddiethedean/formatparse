"""Comprehensive tests for error handling and edge cases"""

import pytest
from formatparse import parse, compile


def test_invalid_pattern_unmatched_brace():
    """Malformed pattern yields None from parse (compile raises PatternParseMismatch)."""
    assert parse("{unclosed", "text") is None


def test_invalid_pattern_double_open():
    """{{ escapes to literal {; remainder is still an unclosed field → no match."""
    assert parse("{{unclosed", "text") is None


def test_invalid_pattern_invalid_type_specifier():
    """Unknown type without extra_types does not match."""
    assert parse("{value:xyz}", "value: test") is None


def test_repeated_name_same_type():
    """Test that repeated names with same type are allowed"""
    result = parse("{name} {name}", "Alice Alice")
    assert result is not None
    assert result.named["name"] == "Alice"


def test_empty_string_input():
    """Non-greedy {} matches empty string with empty fixed capture."""
    result = parse("{}", "")
    assert result is not None
    assert result.fixed[0] == ""


def test_empty_pattern():
    """Test parsing with empty pattern"""
    result = parse("", "")
    assert result is not None


def test_none_values():
    """Test that None values are handled"""
    # None as string input
    with pytest.raises((TypeError, AttributeError)):
        parse("{}", None)


def test_very_large_string():
    """Test with extremely long string"""
    from formatparse import search

    # Use a smaller size to avoid memory issues, but still test large strings
    # parse() requires full match, so use search() for large strings
    large_text = "x" * 10000 + "age: 30"
    result = search("age: {age:d}", large_text)
    assert result is not None
    assert result.named["age"] == 30


def test_very_large_pattern():
    """Test with very long pattern"""
    # Create pattern with many fields
    pattern = " ".join(f"{{field{i}}}" for i in range(100))
    text = " ".join(f"value{i}" for i in range(100))
    result = parse(pattern, text)
    assert result is not None
    assert len(result.named) == 100


def test_type_conversion_error_integer():
    """Test type conversion error for integer"""
    result = parse("{value:d}", "value: abc")
    # Should return None if conversion fails
    assert result is None


def test_type_conversion_error_float():
    """Test type conversion error for float"""
    result = parse("{value:f}", "value: not_a_number")
    assert result is None


def test_custom_type_error():
    """Test error in custom type converter"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        if text == "0":
            raise ValueError("Zero not allowed")
        return int(text)

    # Should raise error when custom converter raises
    with pytest.raises(ValueError, match="Zero not allowed"):
        parse("Value: {:Number}", "Value: 0", {"Number": parse_number})


def test_unicode_edge_cases():
    """Test unicode edge cases"""
    # Emoji
    result = parse("{text}", "text: 😀")
    assert result is not None
    assert "😀" in result.named["text"]

    # Combining characters
    result = parse("{text}", "text: café")
    assert result is not None
    assert "café" in result.named["text"]


def test_special_regex_characters_in_pattern():
    """Test special regex characters in pattern"""
    # Pattern with special characters
    result = parse("price: ${price:f}", "price: $3.14")
    assert result is not None
    assert result.named["price"] == 3.14


def test_special_regex_characters_in_string():
    """Test special regex characters in string"""
    # String with special characters
    result = parse("text: {text}", "text: test (with) [special] {chars}")
    assert result is not None
    assert "test" in result.named["text"]


def test_unicode_in_field_names():
    """Test unicode in field names"""
    result = parse("{名字}: {age:d}", "名字: 30")
    assert result is not None
    assert result.named["名字"] == "名字"
    assert result.named["age"] == 30


def test_very_deep_nesting():
    """Nested dict field names parse into nested mapping structure."""
    pattern = "{a[b[c[d]]]}"
    result = parse(pattern, "value")
    assert result is not None
    assert result.named["a"]["b"]["c"]["d"] == {"]]": "value"}


def test_invalid_width_precision():
    """Absurdly large width fails at pattern parse time."""
    with pytest.raises(ValueError, match="1000"):
        parse("{value:1000000}", "value: test")


def test_malformed_format_spec():
    """Double colon in format spec is not a valid integer field."""
    assert parse("{value::d}", "value: 42") is None


def test_empty_field_name():
    """Test empty field name"""
    result = parse("{}", "test")
    assert result is not None
    assert result.fixed[0] == "test"


def test_whitespace_only_pattern():
    """Test pattern with only whitespace"""
    result = parse("   ", "   ")
    assert result is not None


def test_whitespace_only_string():
    """Test string with only whitespace"""
    result = parse("{}", "   ")
    assert result is not None
    assert result.fixed[0] == "   "


def test_newline_in_pattern():
    """Test pattern with newlines"""
    result = parse("hello\n{name}\nworld", "hello\nAlice\nworld")
    assert result is not None
    assert result.named["name"] == "Alice"


def test_newline_in_string():
    """Default . does not match newline; full-string parse fails."""
    assert parse("hello {name} world", "hello Alice\nworld") is None


def test_tab_characters():
    """Test tab characters in pattern and string"""
    result = parse("hello\t{name}\tworld", "hello\tAlice\tworld")
    assert result is not None
    assert result.named["name"] == "Alice"


def test_carriage_return():
    """Test carriage return characters"""
    result = parse("hello\r{name}\rworld", "hello\rAlice\rworld")
    assert result is not None
    assert result.named["name"] == "Alice"


def test_null_byte():
    """Test null byte handling - now rejected for security"""
    # Null bytes are now rejected for security reasons
    with pytest.raises(ValueError, match="contains null byte"):
        parse("{}", "\x00")


def test_compile_invalid_pattern():
    """Test compiling invalid pattern"""
    with pytest.raises(ValueError):
        compile("{unclosed")


def test_pattern_parse_mismatch_subclass_of_valueerror():
    """PatternParseMismatch is a ValueError subclass (compile / except ValueError)."""
    from formatparse import PatternParseMismatch

    with pytest.raises(PatternParseMismatch) as exc_info:
        compile("{unclosed")
    assert isinstance(exc_info.value, ValueError)


def test_parse_returns_none_for_pattern_parse_mismatch():
    """Malformed pattern that raises PatternParseMismatch on compile yields None from parse."""
    assert parse("{unclosed", "text") is None


def test_parse_batch_returns_nones_for_pattern_parse_mismatch():
    from formatparse import parse_batch

    out = parse_batch("{unclosed", ["a", "b"])
    assert out == [None, None]


def test_parse_propagates_not_implemented_for_unsupported_pattern():
    """Unsupported pattern features still raise NotImplementedError from parse()."""
    with pytest.raises(NotImplementedError):
        parse("{['x]:d}", "1")


def test_compile_empty_pattern():
    """Test compiling empty pattern"""
    parser = compile("")
    assert parser is not None


def test_parse_with_invalid_extra_types():
    """Test parse with invalid extra_types"""
    # extra_types that's not a dict
    with pytest.raises((TypeError, AttributeError)):
        parse("{value}", "value: test", extra_types="not a dict")


def test_search_invalid_pos():
    """Negative pos is clamped to 0; search still finds the match."""
    from formatparse import search

    result = search("age: {age:d}", "age: 30", pos=-1)
    assert result is not None
    assert result.named["age"] == 30


def test_search_invalid_endpos():
    """Test search with invalid endpos"""
    from formatparse import search

    # endpos < pos - should handle gracefully
    result = search("age: {age:d}", "age: 30", pos=10, endpos=5)
    assert result is None  # No match in invalid range
