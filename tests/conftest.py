"""Pytest configuration and shared fixtures for formatparse tests"""

import pytest
from formatparse import with_pattern

_INDENT_BLOCK_SKIP_REASON = (
    "Indent-block :blk tests need a matching _formatparse build. From the repo root "
    "run `maturin develop` or `pip install -e .`, then run pytest with that environment's Python."
)


def _native_indent_block_capable() -> bool:
    """True when the native extension implements :blk (issue #69)."""
    try:
        from formatparse import compile
    except ImportError:
        return False
    try:
        p = compile("{x:blk}")
        r = p.parse("\n  hi\n  there")
    except Exception:
        return False
    return r is not None and r.named.get("x") == "hi\nthere"


def pytest_collection_modifyitems(config, items):
    if _native_indent_block_capable():
        return
    skip_m = pytest.mark.skip(reason=_INDENT_BLOCK_SKIP_REASON)
    for item in items:
        if "test_indent_block.py" in item.nodeid:
            item.add_marker(skip_m)


@pytest.fixture
def sample_patterns():
    """Common test patterns used across smoke and integration tests."""
    return {
        "named": "{name}: {age:d}",
        "positional": "{}, {}",
        "mixed": "{name}, {} years old",
        "typed": "{value:f}",
        "boolean": "{flag:b}",
    }


@pytest.fixture
def sample_strings():
    """Test strings paired with :func:`sample_patterns`."""
    return {
        "named": "Alice: 30",
        "positional": "Hello, World",
        "mixed": "Alice, 30 years old",
        "typed": "3.14",
        "boolean": "True",
    }


@pytest.fixture
def custom_type_converters():
    """Common custom type converters for tests that need extra_types."""

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


def assert_parse_result(result, expected_named=None, expected_fixed=None):
    """Assert a parse/search result has expected named and fixed values."""
    assert result is not None
    if expected_named:
        for key, value in expected_named.items():
            assert result.named[key] == value
    if expected_fixed is not None:
        assert result.fixed == expected_fixed
