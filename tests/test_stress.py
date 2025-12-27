"""Stress and load tests for formatparse

These tests ensure the library handles large inputs, many operations,
and edge cases at scale correctly.
"""

import pytest
import threading
from formatparse import parse, search, findall, compile, BidirectionalPattern


@pytest.mark.stress
def test_large_string_parsing():
    """Test parsing with very long strings (1MB+)"""
    pattern = "Start: {data} End"
    large_text = "Start: " + "x" * 1000000 + " End"
    result = parse(pattern, large_text)
    
    assert result is not None
    assert len(result.named["data"]) == 1000000
    assert result.named["data"] == "x" * 1000000


@pytest.mark.stress
def test_pattern_with_many_fields():
    """Test patterns with many fields (100+)"""
    num_fields = 100
    pattern = " ".join([f"{{field{i}:d}}" for i in range(num_fields)])
    text = " ".join([str(i) for i in range(num_fields)])
    
    result = parse(pattern, text)
    assert result is not None
    assert len(result.named) == num_fields
    
    for i in range(num_fields):
        assert result.named[f"field{i}"] == i


@pytest.mark.stress
def test_findall_with_many_matches():
    """Test findall with many matches (1000+)"""
    pattern = "ID:{id:d}"
    num_matches = 1000
    text = " ".join([f"ID:{i}" for i in range(num_matches)])
    
    results = findall(pattern, text)
    assert len(results) == num_matches
    
    for i, result in enumerate(results):
        assert result.named["id"] == i


@pytest.mark.stress
def test_repeated_operations():
    """Test repeated parsing operations (1M+ iterations)"""
    pattern = "{value:d}"
    text = "42"
    num_iterations = 1000000
    
    # Use compiled parser for efficiency
    parser = compile(pattern)
    
    for _ in range(num_iterations):
        result = parser.parse(text)
        assert result is not None
        assert result.named["value"] == 42


@pytest.mark.stress
def test_concurrent_access():
    """Test thread safety with concurrent access"""
    pattern = "{name}: {age:d}"
    parser = compile(pattern)
    text = "Alice: 30"
    num_threads = 10
    iterations_per_thread = 1000
    
    results = []
    errors = []
    
    def parse_worker():
        try:
            for _ in range(iterations_per_thread):
                result = parser.parse(text)
                if result:
                    results.append(result.named["age"])
        except Exception as e:
            errors.append(e)
    
    threads = []
    for _ in range(num_threads):
        thread = threading.Thread(target=parse_worker)
        threads.append(thread)
        thread.start()
    
    for thread in threads:
        thread.join()
    
    # Should have no errors
    assert len(errors) == 0, f"Errors occurred: {errors}"
    
    # Should have correct number of results
    assert len(results) == num_threads * iterations_per_thread
    
    # All results should be correct
    assert all(age == 30 for age in results)


@pytest.mark.stress
def test_concurrent_findall():
    """Test thread safety with concurrent findall operations"""
    pattern = "ID:{id:d}"
    text = " ".join([f"ID:{i}" for i in range(100)])
    num_threads = 10
    
    results_list = []
    errors = []
    
    def findall_worker():
        try:
            results = findall(pattern, text)
            results_list.append(len(results))
        except Exception as e:
            errors.append(e)
    
    threads = []
    for _ in range(num_threads):
        thread = threading.Thread(target=findall_worker)
        threads.append(thread)
        thread.start()
    
    for thread in threads:
        thread.join()
    
    assert len(errors) == 0, f"Errors occurred: {errors}"
    assert all(count == 100 for count in results_list)


@pytest.mark.stress
def test_large_pattern_compilation():
    """Test compilation of patterns with many fields"""
    # MAX_FIELDS is 100, so test with exactly that many fields
    num_fields = 100
    pattern = " ".join([f"{{field{i}:d}}" for i in range(num_fields)])
    
    parser = compile(pattern)
    assert parser is not None
    assert len(parser.named_fields) == num_fields
    
    # Test that exceeding the limit fails
    num_fields_too_many = 101
    pattern_too_many = " ".join([f"{{field{i}:d}}" for i in range(num_fields_too_many)])
    
    with pytest.raises(ValueError, match="exceeds the maximum allowed count"):
        compile(pattern_too_many)


@pytest.mark.stress
def test_search_in_large_text():
    """Test search operation in very long text"""
    pattern = "target: {value:d}"
    # Create large text with target near the end
    large_text = "x" * 500000 + " target: 42 " + "y" * 500000
    
    result = search(pattern, large_text)
    assert result is not None
    assert result.named["value"] == 42


@pytest.mark.stress
def test_memory_pressure_repeated_compilation():
    """Test repeated pattern compilation under memory pressure"""
    patterns = [
        f"{{field{i}:d}}" for i in range(100)
    ]
    
    # Compile many different patterns
    parsers = []
    for pattern in patterns:
        parser = compile(pattern)
        parsers.append(parser)
    
    # All parsers should work
    for i, parser in enumerate(parsers):
        result = parser.parse(str(i))
        assert result is not None
        assert result.named[f"field{i}"] == i


@pytest.mark.stress
def test_bidirectional_pattern_repeated_format():
    """Test repeated formatting with BidirectionalPattern"""
    pattern = "{name}: {value:d}"
    formatter = BidirectionalPattern(pattern)
    values = {"name": "Test", "value": 42}
    
    for _ in range(100000):
        formatted = formatter.format(values)
        assert "Test" in formatted
        assert "42" in formatted
        
        # Parse it back
        result = formatter.parse(formatted)
        assert result is not None
        assert result.named["value"] == 42

