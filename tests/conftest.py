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
