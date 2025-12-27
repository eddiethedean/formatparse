"""Basic tests for formatparse"""

import pytest
from formatparse import parse, search, findall


def test_simple_string():
    """Test parsing a simple string pattern"""
    result = parse("Hello, {}!", "Hello, World!")
    assert result is not None
    # The {} field matches "World" (between "Hello, " and "!")
    assert result.fixed == ("World",)


def test_named_fields():
    """Test parsing with named fields"""
    result = parse("{name}: {age:d}", "Alice: 30")
    assert result is not None
    assert result.named["name"] == "Alice"
    assert result.named["age"] == 30


def test_integer_parsing():
    """Test integer type conversion"""
    result = parse("Number: {num:d}", "Number: 42")
    assert result is not None
    assert result.named["num"] == 42
    assert isinstance(result.named["num"], int)


def test_float_parsing():
    """Test float type conversion"""
    result = parse("Pi: {pi:f}", "Pi: 3.14")
    assert result is not None
    assert abs(result.named["pi"] - 3.14) < 0.001
    assert isinstance(result.named["pi"], float)


def test_positional_fields():
    """Test parsing with positional fields"""
    result = parse("{}, {}", "Hello, World")
    assert result is not None
    assert len(result.fixed) == 2
    assert result.fixed[0] == "Hello"
    assert result.fixed[1] == "World"


def test_no_match():
    """Test when pattern doesn't match"""
    result = parse("{name}: {age:d}", "Not a match")
    assert result is None


def test_search():
    """Test search function"""
    result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
    assert result is not None
    assert result.named["age"] == 30


def test_findall():
    """Test findall function"""
    # The pattern "{}" matches non-greedily, so in "a b c" it matches
    # single characters: "a", " ", "b", " ", "c"
    results = findall("{}", "a b c")
    assert len(results) >= 3
    # First match should be "a"
    assert results[0].fixed[0] == "a"

    # Test with a pattern that has literal text
    results2 = findall("x{}y", "xay xby xcy")
    assert len(results2) == 3
    assert results2[0].fixed[0] == "a"
    assert results2[1].fixed[0] == "b"
    assert results2[2].fixed[0] == "c"


def test_result_indexing():
    """Test ParseResult indexing"""
    result = parse("{name}: {age:d}", "Alice: 30")
    assert result is not None
    # Named fields go to .named, not .fixed, so indexing by position doesn't work
    # But we can index by name
    assert result["name"] == "Alice"
    assert result["age"] == 30

    # Test positional fields
    result2 = parse("{}, {}", "Hello, World")
    assert result2[0] == "Hello"
    assert result2[1] == "World"


def test_result_contains():
    """Test ParseResult __contains__"""
    result = parse("{name}: {age:d}", "Alice: 30")
    assert result is not None
    # Named fields don't have positional indices
    assert "name" in result
    assert "age" in result
    assert "unknown" not in result

    # Test with positional fields
    result2 = parse("{}, {}", "Hello, World")
    assert 0 in result2
    assert 1 in result2
    assert 2 not in result2


def test_case_sensitive():
    """Test case-sensitive matching"""
    # For case-sensitive, exact match required
    result1 = parse("Hello, {name}", "Hello, World", case_sensitive=True)
    assert result1 is not None  # Exact match works

    # Case-insensitive should match even if case differs
    result2 = parse("hello, {name}", "Hello, World", case_sensitive=False)
    assert result2 is not None  # Case-insensitive match
    assert result2.named["name"] == "World"


if __name__ == "__main__":
    pytest.main([__file__])
