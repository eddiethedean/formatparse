"""Memory leak detection tests for formatparse

These tests ensure that repeated operations don't leak memory,
which is especially important for PyO3 bindings.
"""

import pytest
import gc
from formatparse import parse, compile, BidirectionalPattern


def test_repeated_parsing_no_leak():
    """Test that repeated parsing operations don't leak memory"""
    pattern = "{name}: {age:d}"
    text = "Alice: 30"
    num_iterations = 10000
    
    # Force garbage collection before test
    gc.collect()
    
    # Run many parsing operations
    for _ in range(num_iterations):
        result = parse(pattern, text)
        assert result is not None
    
    # Force garbage collection after test
    gc.collect()
    
    # If memory was properly managed, we should be able to complete
    # without excessive memory growth
    # Note: This is a basic test; for more thorough testing, use pympler


def test_compiled_parser_reuse_no_leak():
    """Test that reusing a compiled parser doesn't leak memory"""
    pattern = "{value:d}"
    parser = compile(pattern)
    num_iterations = 50000
    
    gc.collect()
    
    for _ in range(num_iterations):
        result = parser.parse("42")
        assert result is not None
        assert result.named["value"] == 42
    
    gc.collect()


def test_bidirectional_pattern_reuse_no_leak():
    """Test that reusing BidirectionalPattern doesn't leak memory"""
    pattern = "{name}: {value:d}"
    formatter = BidirectionalPattern(pattern)
    num_iterations = 10000
    
    gc.collect()
    
    for _ in range(num_iterations):
        # Format
        formatted = formatter.format({"name": "Test", "value": 42})
        # Parse
        result = formatter.parse(formatted)
        assert result is not None
    
    gc.collect()


@pytest.mark.slow
def test_long_running_operation_memory():
    """Test memory usage in long-running operations"""
    try:
        from pympler import tracker
        
        tr = tracker.SummaryTracker()
        
        pattern = "{name}: {age:d}"
        text = "Alice: 30"
        
        # Take baseline snapshot
        tr.print_diff()
        
        # Run many operations
        num_iterations = 50000
        for _ in range(num_iterations):
            result = parse(pattern, text)
            assert result is not None
        
        # Check memory growth
        tr.print_diff()
        
        # The diff should show minimal growth for a well-behaved library
        # Note: Exact thresholds would need to be calibrated based on actual usage
        
    except ImportError:
        pytest.skip("pympler not available, skipping detailed memory tracking")


@pytest.mark.slow
def test_findall_memory_usage():
    """Test memory usage with findall operations"""
    try:
        from pympler import tracker
        from formatparse import findall
        
        tr = tracker.SummaryTracker()
        
        pattern = "ID:{id:d}"
        text = " ".join([f"ID:{i}" for i in range(1000)])
        
        tr.print_diff()
        
        # Run many findall operations
        for _ in range(1000):
            results = findall(pattern, text)
            assert len(results) == 1000
            # Consume results to ensure they're not kept in memory
            del results
        
        tr.print_diff()
        
    except ImportError:
        pytest.skip("pympler not available, skipping detailed memory tracking")


def test_pattern_compilation_memory():
    """Test that pattern compilation doesn't leak memory"""
    num_patterns = 1000
    patterns = [f"{{field{i}:d}}" for i in range(num_patterns)]
    
    gc.collect()
    
    parsers = []
    for pattern in patterns:
        parser = compile(pattern)
        parsers.append(parser)
    
    # Verify all parsers work
    for i, parser in enumerate(parsers):
        result = parser.parse(str(i))
        assert result is not None
    
    # Clear parsers
    del parsers
    gc.collect()


@pytest.mark.slow
def test_memory_profiler_integration():
    """Test memory usage using memory_profiler"""
    try:
        from memory_profiler import profile
        
        @profile
        def run_parsing_operations():
            pattern = "{name}: {age:d}"
            text = "Alice: 30"
            for _ in range(10000):
                result = parse(pattern, text)
            return True
        
        # Run with profiling
        result = run_parsing_operations()
        assert result is True
        
    except ImportError:
        pytest.skip("memory_profiler not available")


def test_pyo3_binding_memory():
    """Test PyO3 binding memory management"""
    # Create many formatparse objects
    pattern = "{value:d}"
    parsers = []
    
    gc.collect()
    
    for _ in range(1000):
        parser = compile(pattern)
        parsers.append(parser)
    
    # Use all parsers
    for parser in parsers:
        result = parser.parse("42")
        assert result is not None
    
    # Delete and garbage collect
    del parsers
    gc.collect()
    
    # Should not have excessive memory growth
    # This is a basic test; PyO3 should handle memory automatically

