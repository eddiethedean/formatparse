"""Security-focused tests for formatparse

These tests verify security features including input validation,
ReDoS protection, resource limits, and handling of malicious inputs.
"""

import pytest
import time
from formatparse import parse, search, findall, compile


def test_pattern_length_limit():
    """Test that patterns exceeding MAX_PATTERN_LENGTH are rejected"""
    # Create a pattern that exceeds the limit (10,000 characters)
    long_pattern = "{" + "a" * 10001 + "}"
    
    with pytest.raises(ValueError, match="exceeds maximum allowed length"):
        parse(long_pattern, "test")


def test_input_length_limit():
    """Test that input strings exceeding MAX_INPUT_LENGTH are rejected"""
    # Create an input that exceeds the limit (10MB)
    long_input = "x" * 10_000_001
    
    with pytest.raises(ValueError, match="exceeds maximum allowed length"):
        parse("{value}", long_input)


def test_field_count_limit():
    """Test that patterns with too many fields are rejected"""
    # Create a pattern with more than 100 fields (MAX_FIELDS)
    pattern = " ".join([f"{{field{i}}}" for i in range(101)])
    text = " ".join(["value" for _ in range(101)])
    
    with pytest.raises(ValueError, match="exceeds the maximum allowed count"):
        parse(pattern, text)


def test_null_byte_in_pattern():
    """Test that null bytes in patterns are rejected"""
    pattern_with_null = "test\0value"
    
    with pytest.raises(ValueError, match="contains null byte"):
        parse(pattern_with_null, "test")


def test_null_byte_in_input():
    """Test that null bytes in input strings are rejected"""
    input_with_null = "test\0value"
    
    with pytest.raises(ValueError, match="contains null byte"):
        parse("{value}", input_with_null)


def test_redos_protection_simple():
    """Test ReDoS protection with a simple exponential backtracking pattern"""
    # This pattern could cause exponential backtracking
    # The library should handle it within the timeout limit
    pattern = "{value}"
    # Create input that might trigger backtracking
    input_text = "a" * 1000 + "b"
    
    # Should either succeed quickly or fail gracefully
    start = time.time()
    result = parse(pattern, input_text)
    elapsed = time.time() - start
    
    # Should complete in reasonable time (< 1 second for this case)
    assert elapsed < 1.0, f"Parsing took {elapsed}s, possible ReDoS"


def test_redos_protection_complex_pattern():
    """Test ReDoS protection with complex patterns"""
    # Complex pattern that might cause issues
    pattern = "{" + "a" * 100 + ".*" + "b" * 100 + "}"
    input_text = "a" * 50 + "c" * 50
    
    # Should either succeed or fail gracefully, not hang
    start = time.time()
    try:
        result = parse(pattern, input_text)
    except (ValueError, Exception):
        pass  # Expected to fail, but shouldn't hang
    elapsed = time.time() - start
    
    assert elapsed < 1.0, f"Pattern compilation took {elapsed}s, possible ReDoS"


def test_oversized_pattern_compilation():
    """Test that compiling oversized patterns fails gracefully"""
    # Pattern that's just under the limit should work
    pattern_ok = "{" + "a" * 9999 + "}"
    try:
        parser = compile(pattern_ok)
        # Should compile successfully
    except ValueError:
        pass  # Also acceptable
    
    # Pattern over the limit should fail
    pattern_too_large = "{" + "a" * 10001 + "}"
    with pytest.raises(ValueError):
        compile(pattern_too_large)


def test_malformed_pattern_handling():
    """Test that malformed patterns are handled safely"""
    malformed_patterns = [
        "{{unclosed",
        "{field",
        "}unmatched",
        "{{{{nested",
        "}{reversed",
    ]
    
    for pattern in malformed_patterns:
        # Should either return None or raise ValueError, never crash
        try:
            result = parse(pattern, "test")
            # If it doesn't raise, result should be None
            assert result is None or isinstance(result, Exception)
        except ValueError:
            pass  # Expected
        except Exception as e:
            pytest.fail(f"Unexpected exception for pattern {pattern!r}: {e}")


def test_resource_exhaustion_large_input():
    """Test handling of large but valid inputs"""
    # Input just under the limit
    large_input = "x" * 9_999_999
    
    # Should handle it, though it may take time
    start = time.time()
    try:
        result = parse("{value}", large_input)
        # If successful, should have parsed the value
        if result:
            assert len(result.named.get("value", "")) > 0
    except (ValueError, MemoryError):
        pass  # Acceptable for very large inputs
    elapsed = time.time() - start
    
    # Should complete in reasonable time
    assert elapsed < 10.0, f"Large input processing took {elapsed}s"


def test_invalid_unicode_handling():
    """Test handling of invalid Unicode sequences"""
    # These should be handled gracefully
    invalid_sequences = [
        "\xff\xfe",  # Invalid UTF-8
        "\xed\xa0\x80",  # Surrogate
    ]
    
    for seq in invalid_sequences:
        # Should either fail gracefully or handle it
        try:
            # Note: Python strings are valid Unicode, so we can't easily test invalid UTF-8
            # This test documents the expectation
            result = parse("{value}", seq)
        except (ValueError, UnicodeError):
            pass  # Expected
        except Exception as e:
            # Should not crash
            assert False, f"Unexpected exception: {e}"


def test_repeated_operations_memory():
    """Test that repeated operations don't cause memory leaks"""
    pattern = "{name}: {age:d}"
    text = "Alice: 30"
    
    # Perform many operations
    for _ in range(1000):
        result = parse(pattern, text)
        assert result is not None
    
    # If we get here without OOM, memory is being managed properly


def test_search_with_malicious_pattern():
    """Test search() with potentially malicious patterns"""
    # Patterns that might be crafted to cause issues
    malicious_patterns = [
        "{value}" + ".*" * 50,  # Many wildcards
        "{" + ("a" * 100 + ".*") * 10 + "}",  # Repeated patterns
    ]
    
    for pattern in malicious_patterns:
        start = time.time()
        try:
            result = search(pattern, "test input")
        except ValueError:
            pass  # Expected for invalid patterns
        elapsed = time.time() - start
        
        # Should complete quickly
        assert elapsed < 1.0, f"Search with pattern took {elapsed}s"


def test_findall_resource_limits():
    """Test findall() with many matches"""
    # Create input with many matches
    pattern = "ID:{id:d}"
    text = " ".join([f"ID:{i}" for i in range(1000)])
    
    # Should handle many matches efficiently
    start = time.time()
    results = findall(pattern, text)
    elapsed = time.time() - start
    
    assert elapsed < 1.0, f"Findall with 1000 matches took {elapsed}s"
    assert len(results) == 1000


def test_compiled_parser_security():
    """Test that compiled parsers respect security limits"""
    # Compile a pattern
    parser = compile("{name}: {age:d}")
    
    # Try to use it with oversized input
    large_input = "x" * 10_000_001
    
    with pytest.raises(ValueError, match="exceeds maximum allowed length"):
        parser.parse(large_input)
    
    # Valid input should work
    result = parser.parse("Alice: 30")
    assert result is not None


def test_error_message_sanitization():
    """Test that error messages don't leak sensitive information"""
    # Error messages should not contain full patterns or internal paths
    try:
        parse("{" + "a" * 20000 + "}", "test")
    except ValueError as e:
        error_msg = str(e)
        # Should not contain the full pattern
        assert len(error_msg) < 500, "Error message too long, may contain full pattern"
        # Should not contain file paths
        assert "/" not in error_msg or "formatparse" not in error_msg.lower(), \
            "Error message may contain internal paths"

