"""Pytest configuration and shared fixtures for formatparse tests"""

import pytest
from formatparse import with_pattern


@pytest.fixture
def sample_patterns():
    """Common test patterns"""
    return {
        "named": "{name}: {age:d}",
        "positional": "{}, {}",
        "mixed": "{name}, {} years old",
        "typed": "{value:f}",
        "boolean": "{flag:b}",
    }


@pytest.fixture
def sample_strings():
    """Common test strings"""
    return {
        "named": "Alice: 30",
        "positional": "Hello, World",
        "mixed": "Alice, 30 years old",
        "typed": "3.14",
        "boolean": "True",
    }


@pytest.fixture
def custom_type_converters():
    """Common custom type converters for testing"""

    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    @with_pattern(r"[A-Za-z]+")
    def parse_word(text):
        return text.upper()

    @with_pattern(r"(\d+)-(\d+)", regex_group_count=2)
    def parse_range(text, start, end):
        return (int(start), int(end))

    return {
        "Number": parse_number,
        "Word": parse_word,
        "Range": parse_range,
    }


@pytest.fixture
def large_text():
    """Generate large text for performance testing"""
    return " ".join(f"ID:{i}" for i in range(1000))


@pytest.fixture
def unicode_samples():
    """Unicode test samples"""
    return {
        "chinese": "你好",
        "japanese": "こんにちは",
        "korean": "안녕하세요",
        "arabic": "مرحبا",
        "emoji": "😀",
        "combining": "café",
    }


def assert_parse_result(result, expected_named=None, expected_fixed=None):
    """Helper to assert parse result"""
    assert result is not None
    if expected_named:
        for key, value in expected_named.items():
            assert result.named[key] == value
    if expected_fixed:
        assert result.fixed == expected_fixed


def assert_search_result(result, expected_named=None, expected_fixed=None):
    """Helper to assert search result"""
    assert_parse_result(result, expected_named, expected_fixed)


def assert_findall_results(results, count, first_named=None, first_fixed=None):
    """Helper to assert findall results"""
    assert len(results) == count
    if first_named:
        assert results[0].named == first_named
    if first_fixed:
        assert results[0].fixed == first_fixed


@pytest.fixture
def common_patterns():
    """Common test patterns organized by category"""
    return {
        "simple_named": "{name}: {age:d}",
        "simple_positional": "{}, {}",
        "mixed": "{name}, {} years old",
        "typed_integer": "{value:d}",
        "typed_float": "{value:f}",
        "typed_string": "{value:s}",
        "with_width": "{name:>10}",
        "with_precision": "{value:.2f}",
        "with_fill": "{value:0>5d}",
        "datetime_iso": "{date:%Y-%m-%d}",
        "custom": "{value:CustomType}",
    }


@pytest.fixture
def common_test_strings():
    """Common test strings organized by category"""
    return {
        "simple_named": "Alice: 30",
        "simple_positional": "Hello, World",
        "mixed": "Alice, 30 years old",
        "typed_integer": "42",
        "typed_float": "3.14",
        "typed_string": "hello",
        "with_width": "      Alice",
        "with_precision": "3.14",
        "with_fill": "00042",
        "datetime_iso": "2024-01-15",
        "custom": "custom_value",
    }


@pytest.fixture
def type_combinations():
    """Common type specifier combinations for testing"""
    return [
        ("d", 42, "42", int),
        ("f", 3.14, "3.14", float),
        ("s", "hello", "hello", str),
        ("b", 10, "1010", int),
        ("o", 10, "12", int),
        ("x", 10, "a", int),
        ("e", 3.14, "3.140000e+00", float),
        ("g", 3.14, "3.14", float),
    ]


@pytest.fixture
def alignment_combinations():
    """Alignment and fill character combinations"""
    return [
        ("<", "left"),
        (">", "right"),
        ("^", "center"),
        (".<", "left_fill"),
        (".>", "right_fill"),
        (".^", "center_fill"),
    ]


@pytest.fixture
def error_patterns():
    """Patterns that should fail parsing"""
    return [
        ("{{unclosed", "Unclosed brace"),
        ("{field:unknown}", "Unknown type specifier"),
        ("{field", "Invalid field syntax"),
    ]


@pytest.fixture
def large_test_data():
    """Large test data for stress testing"""
    return {
        "long_string": "x" * 10000,
        "many_fields_pattern": " ".join([f"{{field{i}:d}}" for i in range(50)]),
        "many_fields_text": " ".join([str(i) for i in range(50)]),
        "findall_text": " ".join([f"ID:{i}" for i in range(1000)]),
    }
