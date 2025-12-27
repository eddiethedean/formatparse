"""Performance benchmark tests for formatparse

These tests use pytest-benchmark to measure performance and detect regressions.
Run with: pytest tests/test_performance.py --benchmark-only
Run without benchmarks: pytest tests/test_performance.py --benchmark-skip
"""

import pytest
from formatparse import parse, search, findall, compile, BidirectionalPattern


@pytest.mark.benchmark
def test_parse_simple_named_fields(benchmark):
    """Benchmark: Simple parsing with named fields"""
    pattern = "{name}: {age:d}"
    text = "Alice: 30"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert result.named["name"] == "Alice"
    assert result.named["age"] == 30


@pytest.mark.benchmark
def test_parse_multiple_named_fields(benchmark):
    """Benchmark: Parsing with multiple named fields"""
    pattern = "{name} is {age:d} years old and lives in {city}"
    text = "Alice is 30 years old and lives in NYC"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert len(result.named) == 3


@pytest.mark.benchmark
def test_parse_positional_fields(benchmark):
    """Benchmark: Parsing with positional fields"""
    pattern = "{}, {}"
    text = "Hello, World"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert len(result.fixed) == 2


@pytest.mark.benchmark
def test_parse_complex_pattern(benchmark):
    """Benchmark: Complex pattern with multiple types"""
    pattern = "Value: {value:f}, Count: {count:d}, Name: {name}"
    text = "Value: 3.14159, Count: 42, Name: Test"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert result.named["value"] == 3.14159
    assert result.named["count"] == 42


@pytest.mark.benchmark
def test_parse_no_match(benchmark):
    """Benchmark: Parsing when pattern doesn't match (should fail fast)"""
    pattern = "{name}: {age:d}"
    text = "This doesn't match at all"
    result = benchmark(parse, pattern, text)
    assert result is None


@pytest.mark.benchmark
def test_search_operation(benchmark):
    """Benchmark: Search operation"""
    pattern = "age: {age:d}"
    text = "Name: Alice, age: 30, City: NYC"
    result = benchmark(search, pattern, text)
    assert result is not None
    assert result.named["age"] == 30


@pytest.mark.benchmark
def test_findall_operation(benchmark):
    """Benchmark: Findall operation with multiple matches"""
    pattern = "ID:{id:d}"
    text = " ".join([f"ID:{i}" for i in range(100)])
    results = benchmark(findall, pattern, text)
    assert len(results) == 100


@pytest.mark.benchmark
def test_compile_pattern(benchmark):
    """Benchmark: Pattern compilation"""
    pattern = "{name}: {age:d}"
    parser = benchmark(compile, pattern)
    assert parser is not None
    assert parser.pattern == pattern


@pytest.mark.benchmark
def test_compiled_parser_parse(benchmark):
    """Benchmark: Parsing with pre-compiled pattern"""
    pattern = "{name}: {age:d}"
    parser = compile(pattern)
    text = "Alice: 30"
    
    def run_parse():
        return parser.parse(text)
    
    result = benchmark(run_parse)
    assert result is not None
    assert result.named["name"] == "Alice"


@pytest.mark.benchmark
def test_bidirectional_pattern_parse(benchmark):
    """Benchmark: BidirectionalPattern parsing"""
    pattern = "{name}: {value:d}"
    formatter = BidirectionalPattern(pattern)
    text = "Test: 42"
    result = benchmark(formatter.parse, text)
    assert result is not None
    assert result.named["value"] == 42


@pytest.mark.benchmark
def test_bidirectional_pattern_format(benchmark):
    """Benchmark: BidirectionalPattern formatting"""
    pattern = "{name}: {value:d}"
    formatter = BidirectionalPattern(pattern)
    values = {"name": "Test", "value": 42}
    formatted = benchmark(formatter.format, values)
    assert "Test" in formatted
    assert "42" in formatted


@pytest.mark.benchmark
def test_long_string_parsing(benchmark):
    """Benchmark: Parsing with long input string"""
    pattern = "Start: {data} End"
    long_text = "Start: " + "x" * 10000 + " End"
    result = benchmark(parse, pattern, long_text)
    assert result is not None
    assert len(result.named["data"]) == 10000


@pytest.mark.benchmark
def test_many_fields_pattern(benchmark):
    """Benchmark: Pattern with many fields"""
    num_fields = 20
    pattern = " ".join([f"{{field{i}:d}}" for i in range(num_fields)])
    text = " ".join([str(i) for i in range(num_fields)])
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert len(result.named) == num_fields


@pytest.mark.benchmark
def test_integer_type_conversion(benchmark):
    """Benchmark: Integer type conversion"""
    pattern = "{value:d}"
    text = "12345"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert result.named["value"] == 12345
    assert isinstance(result.named["value"], int)


@pytest.mark.benchmark
def test_float_type_conversion(benchmark):
    """Benchmark: Float type conversion"""
    pattern = "{value:f}"
    text = "3.14159"
    result = benchmark(parse, pattern, text)
    assert result is not None
    assert result.named["value"] == 3.14159
    assert isinstance(result.named["value"], float)

