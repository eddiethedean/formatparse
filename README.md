# formatparse

A Rust-backed implementation of the [parse](https://github.com/r1chardj0n3s/parse) library for Python. This library provides the same API as the original `parse` library but with improved performance and reliability thanks to Rust.

## Installation

```bash
pip install formatparse
```

Or build from source:

```bash
pip install maturin
maturin develop
```

## Usage

Parse strings using a specification based on the Python `format()` syntax:

```python
from formatparse import parse

result = parse("Hello, {}!", "Hello, World!")
print(result.fixed)  # ['World!']

result = parse("{name}: {age:d}", "Alice: 30")
print(result.named)  # {'name': 'Alice', 'age': 30}
```

### Basic Parsing

```python
from formatparse import parse

# Named fields
result = parse("{name} is {age:d} years old", "Alice is 30 years old")
print(result.named['name'])  # 'Alice'
print(result.named['age'])   # 30

# Positional fields
result = parse("{}, {}", "Hello, World")
print(result.fixed)  # ['Hello', 'World']

# Type conversion
result = parse("Value: {value:f}", "Value: 3.14")
print(result.named['value'])  # 3.14
```

### Searching

```python
from formatparse import search

result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
if result:
    print(result.named['age'])  # 30
```

### Finding All Matches

```python
from formatparse import findall

results = findall("{word}", "hello world python")
for result in results:
    print(result.fixed[0])
# hello
# world
# python
```

### Custom Types

```python
from formatparse import parse, with_pattern

@with_pattern(r'\d+')
def parse_number(text):
    return int(text)

result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
print(result.fixed[0])  # 42
```

## API Reference

### `parse(pattern, string, case_sensitive=True, extra_types=None)`

Parse a string using a format specification.

- **pattern**: Format specification pattern (e.g., `"{name}: {age:d}"`)
- **string**: String to parse
- **case_sensitive**: Whether matching should be case sensitive (default: `True`)
- **extra_types**: Dictionary of custom type converters

Returns: `ParseResult` if match found, `None` otherwise

### `search(pattern, string, pos=0, endpos=None, case_sensitive=True, extra_types=None)`

Search for a pattern in a string.

- **pattern**: Format specification pattern
- **string**: String to search
- **pos**: Start position (default: 0)
- **endpos**: End position (default: `None` for end of string)
- **case_sensitive**: Whether matching should be case sensitive (default: `True`)
- **extra_types**: Dictionary of custom type converters

Returns: `ParseResult` if match found, `None` otherwise

### `findall(pattern, string, case_sensitive=True, extra_types=None)`

Find all matches of a pattern in a string.

- **pattern**: Format specification pattern
- **string**: String to search
- **case_sensitive**: Whether matching should be case sensitive (default: `True`)
- **extra_types**: Dictionary of custom type converters

Returns: List of `ParseResult` objects

### `with_pattern(pattern, regex_group_count=0)`

Decorator to create a custom type converter with a regex pattern.

- **pattern**: The regex pattern to match
- **regex_group_count**: Number of regex groups in the pattern (for parentheses)

### `ParseResult`

Result object containing parsed data.

- **fixed**: List of positional values
- **named**: Dictionary of named values
- **span**: Tuple of (start, end) positions
- **start**: Start position
- **end**: End position

Supports indexing with `result[0]` or `result['name']`.

## Type Specifiers

- `s` - String (default)
- `d` or `i` - Integer
- `f`, `F`, `e`, `E`, `g`, `G` - Float
- `b` - Boolean

## Differences from Original parse Library

This library aims to be a drop-in replacement for the original `parse` library, but there may be subtle differences in edge cases. The core functionality and API are compatible.

## Performance

This Rust-backed implementation provides significant performance improvements over the pure Python implementation, especially for:
- Complex patterns with many fields
- Large input strings
- Repeated parsing operations
- Search operations in long strings

### Benchmark Results

#### Standard Benchmarks

| Test Case | Result | Speedup |
|-----------|--------|---------|
| Simple named fields | ✅ Faster | 1.45x |
| Multiple named fields | ⚠️ Slower | 1.11x |
| Positional fields | ⚠️ Slower | 1.01x |
| Complex pattern with types | ⚠️ Slower | 1.12x |
| No match (fail fast) | ✅ Faster | 1.32x |
| Long string with match at end | ✅ Faster | 1.23x |

#### Optimization-Focused Benchmarks

These benchmarks highlight scenarios where Rust optimizations provide the most benefit:

| Optimization Test | Result | Speedup |
|-------------------|--------|---------|
| **Long String Search** | ✅ Faster | **23.80x** |
| **Cache Warmup** | ✅ Faster | **4.47x** |
| **Pre-compiled Search Regex** | ✅ Faster | **3.29x** |
| **Fast Type Conversion Paths** | ✅ Faster | **1.58x** |
| **Pre-allocation (Many Fields)** | ✅ Faster | **1.35x** |
| **Mixed Patterns (Cache Management)** | ✅ Faster | **1.44x** |
| **Pattern Caching (LRU Cache)** | ✅ Faster | **1.06x** |
| Case-Insensitive Matching | ⚠️ Slower | 1.23x |
| Findall (Multiple Matches) | ⚠️ Slower | 3.78x |

**Key Optimizations:**
- **Pattern Caching**: LRU cache (1000 patterns) eliminates regex compilation overhead
- **Regex Pre-compilation**: Pre-compiled search regex variants for faster search operations
- **Fast Type Conversion**: Optimized paths for common type conversions (int, float, bool)
- **Pre-allocation**: Pre-allocated vectors and HashMaps reduce memory allocations
- **Reduced GIL Overhead**: Batched Python operations minimize interpreter overhead

Run the benchmarks yourself:
```bash
# Standard comparison benchmarks
python scripts/benchmark.py

# Optimization-focused benchmarks
python scripts/benchmark_optimizations.py
```

## License

MIT License - see LICENSE file for details

## Credits

Based on the [parse](https://github.com/r1chardj0n3s/parse) library by Richard Jones.

