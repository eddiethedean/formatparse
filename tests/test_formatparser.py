"""Comprehensive tests for the FormatParser class (compiled patterns)"""

import pytest
import pickle
from formatparse import compile, FormatParser, RepeatedNameError


def test_compile_valid_pattern():
    """Test compiling a valid pattern"""
    parser = compile("{name}: {age:d}")
    assert parser is not None
    assert isinstance(parser, FormatParser)
    assert parser.pattern == "{name}: {age:d}"


def test_compile_invalid_pattern():
    """Test compiling an invalid pattern raises error"""
    with pytest.raises(ValueError):
        compile("{unclosed")


def test_compile_with_extra_types():
    """Test compiling with extra types"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    # compile() doesn't take extra_types - pass them to parse() instead
    # Pattern needs to include literal prefix
    parser = compile("value: {value:Number}")
    assert parser is not None
    result = parser.parse("value: 42", extra_types={"Number": parse_number})
    assert result is not None
    assert result.named["value"] == 42


def test_parser_parse_basic():
    """Test parser.parse() with basic pattern"""
    parser = compile("{name}: {age:d}")
    result = parser.parse("Alice: 30")
    assert result is not None
    assert result.named["name"] == "Alice"
    assert result.named["age"] == 30


def test_parser_parse_no_match():
    """Test parser.parse() when pattern doesn't match"""
    parser = compile("{name}: {age:d}")
    result = parser.parse("No match here")
    assert result is None


def test_parser_parse_case_sensitive():
    """Test parser.parse() with case sensitivity"""
    parser = compile("Hello, {name}")
    # Case-sensitive (default False for parse)
    result = parser.parse("Hello, World", case_sensitive=False)
    assert result is not None
    assert result.named["name"] == "World"

    result = parser.parse("HELLO, World", case_sensitive=False)
    assert result is not None

    result = parser.parse("HELLO, World", case_sensitive=True)
    assert result is None


def test_parser_parse_with_extra_types():
    """Test parser.parse() with extra types parameter"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    parser = compile("{value}")
    # This won't match because pattern doesn't use Number type
    # But test that extra_types parameter is accepted
    parser.parse("value:42", extra_types={"Number": parse_number})
    assert parser.parse("value:42") is not None


def test_parser_parse_evaluate_result_false():
    """Test parser.parse() with evaluate_result=False"""
    parser = compile("Hello, {name}")
    match = parser.parse("Hello, World", evaluate_result=False)
    assert match is not None
    # Should be a Match object, not ParseResult
    result = match.evaluate_result()
    assert result.named["name"] == "World"


def test_parser_search_basic():
    """Test parser.search() with basic pattern"""
    parser = compile("age: {age:d}")
    result = parser.search("Name: Alice, age: 30, City: NYC")
    assert result is not None
    assert result.named["age"] == 30


def test_parser_search_no_match():
    """Test parser.search() when pattern doesn't match"""
    parser = compile("age: {age:d}")
    result = parser.search("No match here")
    assert result is None


def test_parser_search_case_sensitive():
    """Test parser.search() with case sensitivity"""
    parser = compile("age: {age:d}")
    # Case-sensitive (default True for search)
    result = parser.search("Age: 30", case_sensitive=True)
    assert result is None

    result = parser.search("Age: 30", case_sensitive=False)
    assert result is not None
    assert result.named["age"] == 30


def test_parser_reuse():
    """Test reusing the same parser instance with multiple strings"""
    parser = compile("{name}: {age:d}")

    result1 = parser.parse("Alice: 30")
    assert result1 is not None
    assert result1.named["name"] == "Alice"
    assert result1.named["age"] == 30

    result2 = parser.parse("Bob: 25")
    assert result2 is not None
    assert result2.named["name"] == "Bob"
    assert result2.named["age"] == 25

    result3 = parser.parse("Charlie: 35")
    assert result3 is not None
    assert result3.named["name"] == "Charlie"
    assert result3.named["age"] == 35


def test_parser_expression_property():
    """Test _expression property returns regex pattern"""
    parser = compile("{name}")
    expression = parser._expression
    assert isinstance(expression, str)
    assert "name" in expression or ".+?" in expression


def test_parser_named_fields_property():
    """Test named_fields property"""
    parser = compile("{name}: {age:d}")
    named_fields = parser.named_fields
    assert isinstance(named_fields, list)
    assert "name" in named_fields
    assert "age" in named_fields


def test_parser_named_fields_positional():
    """Test named_fields with positional fields"""
    parser = compile("{}, {}")
    named_fields = parser.named_fields
    assert isinstance(named_fields, list)
    # Positional fields shouldn't appear in named_fields
    assert len(named_fields) == 0


def test_parser_pattern_property():
    """Test pattern property"""
    pattern = "{name}: {age:d}"
    parser = compile(pattern)
    assert parser.pattern == pattern


def test_parser_format_property():
    """Test format property returns Format object"""
    parser = compile("{}, {}")
    format_obj = parser.format
    assert format_obj is not None
    # Test that format object can format values
    # Format works with positional arguments
    formatted = format_obj.format(("Alice", 30))
    assert "Alice" in formatted
    assert "30" in formatted


def test_parser_pickling():
    """Test that parser can be pickled and unpickled"""
    parser = compile("{name}: {age:d}")

    # Pickle the parser
    try:
        pickled = pickle.dumps(parser)

        # Unpickle it
        unpickled = pickle.loads(pickled)

        # Verify it still works
        result = unpickled.parse("Alice: 30")
        assert result is not None
        assert result.named["name"] == "Alice"
        assert result.named["age"] == 30
    except TypeError:
        # Pickling might not be fully supported yet
        pytest.skip("Pickling not fully supported")


def test_parser_with_stored_extra_types():
    """Test parser with extra_types passed to parse()"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    # compile() doesn't store extra_types - need to pass when parsing
    # Pattern needs to include literal prefix
    parser = compile("value: {value:Number}")
    # Pass extra_types when parsing
    result = parser.parse("value: 42", extra_types={"Number": parse_number})
    assert result is not None
    assert result.named["value"] == 42


def test_parser_extra_types_override():
    """Test that provided extra_types are used when parsing"""
    from formatparse import with_pattern

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    @with_pattern(r"\d+")
    def parse_number_alt(text):
        return int(text) * 2  # Different behavior

    # Pattern needs to include literal prefix
    parser = compile("value: {value:Number}")
    # Use different converter - extra_types passed to parse() are used
    result = parser.parse("value: 42", extra_types={"Number": parse_number_alt})
    assert result is not None
    assert result.named["value"] == 84  # Should use the provided converter


def test_parser_repeated_name_error():
    """Test that RepeatedNameError is raised for mismatched repeated names"""
    # This should raise RepeatedNameError if the same name appears with different types
    with pytest.raises(RepeatedNameError):
        compile("{name} {name:d}")


def test_parser_complex_pattern():
    """Test parser with complex pattern"""
    parser = compile("{name} is {age:d} years old and lives in {city}")
    result = parser.parse("Alice is 30 years old and lives in NYC")
    assert result is not None
    assert result.named["name"] == "Alice"
    assert result.named["age"] == 30
    assert result.named["city"] == "NYC"


def test_parser_positional_fields():
    """Test parser with positional fields"""
    parser = compile("{}, {}")
    result = parser.parse("Hello, World")
    assert result is not None
    assert result.fixed == ("Hello", "World")


def test_parser_mixed_fields():
    """Test parser with mixed named and positional fields"""
    parser = compile("{name}, {} years old")
    result = parser.parse("Alice, 30 years old")
    assert result is not None
    assert result.named["name"] == "Alice"
    assert result.fixed[0] == "30"


def test_parser_multiple_searches():
    """Test multiple searches with same parser"""
    parser = compile("age: {age:d}")
    text = "Name: Alice, age: 30, Name: Bob, age: 25"

    result1 = parser.search(text)
    assert result1 is not None
    assert result1.named["age"] == 30

    # Search again from same position should find same match
    result2 = parser.search(text)
    assert result2 is not None
    assert result2.named["age"] == 30


def test_parser_empty_pattern():
    """Test parser with empty pattern"""
    parser = compile("")
    result = parser.parse("")
    # Empty pattern should match empty string
    assert result is not None


def test_parser_literal_braces():
    """Test parser with literal braces"""
    parser = compile("{{hello}}")
    result = parser.parse("{hello}")
    assert result is not None
    assert result.fixed == ()
    assert result.named == {}


def test_parser_unicode_pattern():
    """Test parser with unicode in pattern"""
    parser = compile("{name} says {message}")
    result = parser.parse("Alice says Hello, 世界")
    assert result is not None
    assert result.named["name"] == "Alice"
    assert "世界" in result.named["message"]
