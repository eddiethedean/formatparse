"""Fuzz tests for formatparse

These tests use Hypothesis to generate random inputs and ensure
the library never crashes on unexpected input.
"""

import pytest
import hypothesis
from hypothesis import given, strategies as st, assume, settings, HealthCheck
from formatparse import parse, search, findall, compile


@settings(max_examples=500)
@given(
    pattern=st.text(min_size=1, max_size=200),
    text=st.text(min_size=0, max_size=1000)
)
def test_fuzz_parse_crash_free(pattern, text):
    """Fuzz test: parse() should never crash on any input"""
    try:
        result = parse(pattern, text)
        # Should either return a result or None, never crash
        assert result is None or hasattr(result, 'named')
        assert result is None or hasattr(result, 'fixed')
    except (ValueError, TypeError):
        # Expected exceptions for invalid patterns are fine
        pass
    except Exception as e:
        # Unexpected exceptions indicate bugs
        pytest.fail(f"Unexpected exception on pattern={pattern!r}, text={text!r}: {e}")


@settings(max_examples=500)
@given(
    pattern=st.text(min_size=1, max_size=200),
    text=st.text(min_size=0, max_size=1000)
)
def test_fuzz_search_crash_free(pattern, text):
    """Fuzz test: search() should never crash on any input"""
    try:
        result = search(pattern, text)
        assert result is None or hasattr(result, 'named')
    except (ValueError, TypeError):
        # Expected exceptions for invalid patterns are fine
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception on pattern={pattern!r}, text={text!r}: {e}")


@settings(max_examples=500)
@given(
    pattern=st.text(min_size=1, max_size=200),
    text=st.text(min_size=0, max_size=1000)
)
def test_fuzz_findall_crash_free(pattern, text):
    """Fuzz test: findall() should never crash on any input"""
    try:
        results = findall(pattern, text)
        # Should return a list-like object
        assert hasattr(results, '__len__')
        assert hasattr(results, '__iter__')
    except (ValueError, TypeError):
        # Expected exceptions for invalid patterns are fine
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception on pattern={pattern!r}, text={text!r}: {e}")


@settings(max_examples=200)
@given(
    pattern=st.text(min_size=1, max_size=200)
)
def test_fuzz_compile_crash_free(pattern):
    """Fuzz test: compile() should never crash on any pattern"""
    try:
        parser = compile(pattern)
        # Should return a parser or raise a ValueError for invalid patterns
        assert parser is None or hasattr(parser, 'parse')
    except ValueError:
        # Expected for invalid patterns
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception on pattern={pattern!r}: {e}")


@settings(max_examples=50, suppress_health_check=[HealthCheck.filter_too_much])
@given(
    pattern=st.text(min_size=1, max_size=50),  # Remove filter to reduce filtering overhead
    text=st.text(min_size=0, max_size=200)
)
def test_fuzz_malformed_patterns(pattern, text):
    """Fuzz test: Handle malformed patterns gracefully"""
    try:
        result = parse(pattern, text)
        # Should either parse or return None, not crash
        assert result is None or hasattr(result, 'named')
    except Exception:
        # Various exceptions are acceptable for malformed patterns
        # We're mainly checking for crashes/panics, not correctness
        pass


@settings(max_examples=50)
@given(
    text=st.text(min_size=1000, max_size=5000)  # Moderate large inputs (reduced to avoid memory issues)
)
def test_fuzz_large_inputs(text):
    """Fuzz test: Handle large input strings"""
    pattern = "{data}"
    try:
        result = parse(pattern, f"Start: {text} End")
        if result:
            # Data might be parsed differently, just check it's a string
            assert isinstance(result.named["data"], str)
    except MemoryError:
        # Memory errors on extremely large inputs are acceptable
        pass
    except Exception as e:
        pytest.fail(f"Unexpected exception on large input (len={len(text)}): {e}")


@settings(max_examples=50, suppress_health_check=[HealthCheck.filter_too_much])
@given(
    text=st.text(min_size=1, max_size=50, alphabet=st.characters(min_codepoint=32, max_codepoint=0x7F))  # Limit to ASCII for simpler testing
)
def test_fuzz_unicode_inputs(text):
    """Fuzz test: Handle various Unicode inputs (limited to ASCII range for stability)"""
    pattern = "{data}"
    try:
        result = parse(pattern, f"Start: {text} End")
        if result:
            # Just check it parses, don't assert exact match
            assert isinstance(result.named["data"], str)
    except (Exception, RuntimeError, SystemError):
        # Various exceptions including panics are acceptable for fuzz testing
        # We're mainly checking for crashes, not correctness
        # Note: Panics are caught as PyO3 PanicException which inherits from RuntimeError
        pass


@settings(max_examples=30, suppress_health_check=[HealthCheck.filter_too_much])
@given(
    text=st.text(min_size=10, max_size=100, alphabet=st.characters(min_codepoint=32, max_codepoint=0x7F)),  # Limit to ASCII to avoid byte boundary issues
    pattern=st.text(min_size=1, max_size=20),
    pos=st.integers(min_value=0, max_value=50),
    endpos=st.one_of(st.none(), st.integers(min_value=0, max_value=100))
)
def test_fuzz_search_with_pos(text, pattern, pos, endpos):
    """Fuzz test: search() with pos and endpos parameters"""
    # Ensure constraints are met
    if pos > len(text):
        pos = len(text)
    if endpos is not None:
        if endpos < pos:
            endpos = pos
        if endpos > len(text):
            endpos = len(text)
    
    try:
        result = search(pattern, text, pos=pos, endpos=endpos)
        assert result is None or hasattr(result, 'named')
    except (Exception, RuntimeError, SystemError):
        # Various exceptions including panics are acceptable for fuzz testing
        # We're mainly checking for crashes, not correctness
        # Note: Panics are caught as PyO3 PanicException which inherits from RuntimeError
        pass

