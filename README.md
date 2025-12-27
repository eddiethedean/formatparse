# formatparse

[![PyPI version](https://badge.fury.io/py/formatparse.svg)](https://badge.fury.io/py/formatparse)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

A high-performance, Rust-backed implementation of the [parse](https://github.com/r1chardj0n3s/parse) library for Python. `formatparse` provides the same API as the original `parse` library but with **significant performance improvements** (up to **80x faster**) thanks to Rust's zero-cost abstractions and optimized regex engine.

## Features

- 🚀 **Blazing Fast**: Up to 80x faster than the original Python implementation
- 🔄 **Drop-in Replacement**: Compatible API with the original `parse` library
- 🎯 **Type-Safe**: Rust backend ensures reliability and correctness
- 🔍 **Advanced Pattern Matching**: Support for named fields, positional fields, and custom types
- 📅 **DateTime Parsing**: Built-in support for various datetime formats (ISO 8601, RFC 2822, HTTP dates, etc.)
- 🎨 **Flexible**: Case-sensitive and case-insensitive matching options
- 💾 **Optimized**: Pattern caching, lazy evaluation, and batch operations for maximum performance

## Installation

### From PyPI

```bash
pip install formatparse
```

### From Source

```bash
# Clone the repository
git clone https://github.com/eddiethedean/formatparse.git
cd formatparse

# Install maturin (build tool)
pip install maturin

# Build and install in development mode
maturin develop --release
```

## Quick Start

```python
from formatparse import parse, search, findall

# Basic parsing with named fields
result = parse("{name}: {age:d}", "Alice: 30")
print(result.named['name'])  # 'Alice'
print(result.named['age'])   # 30

# Search for patterns in text
result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
if result:
    print(result.named['age'])  # 30

# Find all matches
results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
for result in results:
    print(result.named['id'])
# Output: 1, 2, 3
```

## Usage Examples

### Basic Parsing

```python
from formatparse import parse

# Named fields
result = parse("{name} is {age:d} years old", "Alice is 30 years old")
print(result.named['name'])  # 'Alice'
print(result.named['age'])   # 30

# Positional fields
result = parse("{}, {}", "Hello, World")
print(result.fixed)  # ('Hello', 'World')

# Type conversion
result = parse("Value: {value:f}", "Value: 3.14")
print(result.named['value'])  # 3.14

# Mixed named and positional
result = parse("{name}, {} years old", "Alice, 30 years old")
print(result.named['name'])  # 'Alice'
print(result.fixed[0])       # '30'
```

### Searching

```python
from formatparse import search

# Search for a pattern anywhere in the string
text = "Name: Alice, age: 30, City: NYC, phone: 555-1234"
result = search("age: {age:d}", text)
if result:
    print(result.named['age'])  # 30

# Search with position constraints
result = search("age: {age:d}", text, pos=20, endpos=40)
```

### Finding All Matches

```python
from formatparse import findall

# Find all matches
results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
print(len(results))  # 3

# Iterate over results
for result in results:
    print(result.named['id'])

# Access by index
print(results[0].named['id'])  # 1
print(results[-1].named['id'])  # 3

# Slice results
first_two = results[:2]
```

### Custom Types

```python
from formatparse import parse, with_pattern

# Define a custom type converter
@with_pattern(r'\d+')
def parse_number(text):
    return int(text)

# Use the custom type
result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
print(result.fixed[0])  # 42

# Custom type with regex groups
@with_pattern(r'(\d+)-(\d+)', regex_group_count=2)
def parse_range(text, start, end):
    return (int(start), int(end))

result = parse("Range: {:Range}", "Range: 10-20", {"Range": parse_range})
print(result.fixed[0])  # (10, 20)
```

### DateTime Parsing

```python
from formatparse import parse

# ISO 8601 format
result = parse("{timestamp:ti}", "2023-12-25T10:30:00Z")
print(result.named['timestamp'])  # datetime object

# RFC 2822 format
result = parse("{date:tr}", "Mon, 25 Dec 2023 10:30:00 +0000")
print(result.named['date'])  # datetime object

# HTTP date format
result = parse("{date:th}", "Mon, 25 Dec 2023 10:30:00 GMT")
print(result.named['date'])  # datetime object
```

### Case Sensitivity

```python
from formatparse import parse, findall

# Case-sensitive (default)
result = parse("Hello, {name}!", "Hello, World!")
print(result)  # Match found

result = parse("Hello, {name}!", "HELLO, World!")
print(result)  # None (no match)

# Case-insensitive
result = parse("Hello, {name}!", "HELLO, World!", case_sensitive=False)
print(result)  # Match found

# Case-insensitive findall
results = findall("x({})x", "X(hi)X", case_sensitive=False)
print(len(results))  # 1
```

### Advanced Pattern Matching

```python
from formatparse import parse

# Width and precision
result = parse("{value:5.2f}", " 3.14")
print(result.named['value'])  # 3.14

# Alignment
result = parse("{name:>10}", "     Alice")
print(result.named['name'])  # 'Alice'

# Nested field access
result = parse("{person[name]}: {person[age]:d}", "Alice: 30")
print(result.named['person']['name'])  # 'Alice'
print(result.named['person']['age'])   # 30
```

## API Reference

### Core Functions

#### `parse(pattern, string, extra_types=None, case_sensitive=False, evaluate_result=True)`

Parse a string using a format specification.

**Parameters:**
- `pattern` (str): Format specification pattern (e.g., `"{name}: {age:d}"`)
- `string` (str): String to parse
- `extra_types` (dict, optional): Dictionary of custom type converters
- `case_sensitive` (bool, optional): Whether matching should be case sensitive (default: `False`)
- `evaluate_result` (bool, optional): Whether to evaluate the result immediately (default: `True`)

**Returns:** `ParseResult` if match found, `None` otherwise

**Example:**
```python
result = parse("{name}: {age:d}", "Alice: 30")
if result:
    print(result.named['name'], result.named['age'])
```

#### `search(pattern, string, pos=0, endpos=None, extra_types=None, case_sensitive=True, evaluate_result=True)`

Search for a pattern in a string.

**Parameters:**
- `pattern` (str): Format specification pattern
- `string` (str): String to search
- `pos` (int, optional): Start position (default: 0)
- `endpos` (int, optional): End position (default: `None` for end of string)
- `extra_types` (dict, optional): Dictionary of custom type converters
- `case_sensitive` (bool, optional): Whether matching should be case sensitive (default: `True`)
- `evaluate_result` (bool, optional): Whether to evaluate the result immediately (default: `True`)

**Returns:** `ParseResult` if match found, `None` otherwise

**Example:**
```python
result = search("age: {age:d}", "Name: Alice, age: 30")
if result:
    print(result.named['age'])
```

#### `findall(pattern, string, extra_types=None, case_sensitive=False, evaluate_result=True)`

Find all matches of a pattern in a string.

**Parameters:**
- `pattern` (str): Format specification pattern
- `string` (str): String to search
- `extra_types` (dict, optional): Dictionary of custom type converters
- `case_sensitive` (bool, optional): Whether matching should be case sensitive (default: `False`)
- `evaluate_result` (bool, optional): Whether to evaluate the result immediately (default: `True`)

**Returns:** `Results` object (list-like) containing `ParseResult` objects

**Example:**
```python
results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
for result in results:
    print(result.named['id'])
```

#### `compile(pattern)`

Compile a pattern into a `FormatParser` for repeated use.

**Parameters:**
- `pattern` (str): Format specification pattern

**Returns:** `FormatParser` object

**Example:**
```python
parser = compile("{name}: {age:d}")
result1 = parser.parse("Alice: 30")
result2 = parser.parse("Bob: 25")
```

#### `with_pattern(pattern, regex_group_count=0)`

Decorator to create a custom type converter with a regex pattern.

**Parameters:**
- `pattern` (str): The regex pattern to match
- `regex_group_count` (int, optional): Number of regex groups in the pattern (for parentheses)

**Returns:** Decorator function

**Example:**
```python
@with_pattern(r'\d+')
def parse_number(text):
    return int(text)
```

### Classes

#### `ParseResult`

Result object containing parsed data.

**Properties:**
- `fixed` (tuple): Tuple of positional values
- `named` (dict): Dictionary of named values
- `span` (tuple): Tuple of (start, end) positions in the original string
- `start` (int): Start position in the original string
- `end` (int): End position in the original string

**Methods:**
- `__getitem__(key)`: Access values by index or name
  - `result[0]` - Access positional value by index
  - `result['name']` - Access named value by name

**Example:**
```python
result = parse("{name}: {age:d}", "Alice: 30")
print(result.named['name'])  # 'Alice'
print(result.named['age'])   # 30
print(result[0])             # 'Alice' (first positional/named value)
print(result['name'])        # 'Alice'
print(result.span)           # (0, 10)
```

#### `FormatParser`

Compiled pattern parser for repeated use.

**Methods:**
- `parse(string, extra_types=None, case_sensitive=False, evaluate_result=True)`: Parse a string
- `search(string, pos=0, endpos=None, extra_types=None, case_sensitive=True, evaluate_result=True)`: Search for a pattern
- `findall(string, extra_types=None, case_sensitive=False, evaluate_result=True)`: Find all matches

**Example:**
```python
parser = compile("{name}: {age:d}")
result = parser.parse("Alice: 30")
```

## Type Specifiers

| Specifier | Description | Example |
|-----------|-------------|---------|
| `s` | String (default) | `"{name}"` matches any string |
| `d` or `i` | Integer | `"{age:d}"` matches `"30"` → `30` |
| `f`, `F` | Fixed-point float | `"{pi:f}"` matches `"3.14"` → `3.14` |
| `e`, `E` | Scientific notation | `"{val:e}"` matches `"1.5e2"` → `150.0` |
| `g`, `G` | General format | `"{val:g}"` matches `"3.14"` or `"1.5e2"` |
| `b` | Boolean | `"{flag:b}"` matches `"True"` → `True` |
| `ti` | ISO 8601 datetime | `"{date:ti}"` matches `"2023-12-25T10:30:00Z"` |
| `tr` | RFC 2822 datetime | `"{date:tr}"` matches `"Mon, 25 Dec 2023 10:30:00 +0000"` |
| `th` | HTTP date | `"{date:th}"` matches `"Mon, 25 Dec 2023 10:30:00 GMT"` |
| `ts` | System log format | `"{date:ts}"` matches `"Dec 25 10:30:00"` |
| `tu` | US format | `"{date:tu}"` matches `"12/25/2023"` |

## Performance

`formatparse` provides significant performance improvements over the original Python `parse` library, especially for:

- Complex patterns with many fields
- Large input strings
- Repeated parsing operations
- Search operations in long strings
- Finding multiple matches (`findall`)

### Benchmark Results

#### Standard Benchmarks

| Test Case | Result | Speedup |
|-----------|--------|---------|
| Simple named fields | ✅ Faster | **8.61x** |
| Multiple named fields | ✅ Faster | **7.59x** |
| Positional fields | ✅ Faster | **5.99x** |
| Complex pattern with types | ✅ Faster | **42.24x** |
| No match (fail fast) | ✅ Faster | **27.64x** |
| Long string with match at end | ✅ Faster | **25.03x** |

#### Optimization-Focused Benchmarks

These benchmarks highlight scenarios where Rust optimizations provide the most benefit:

| Optimization Test | Result | Speedup |
|-------------------|--------|---------|
| **Long String Search** | ✅ Faster | **79.94x** |
| **Fast Type Conversion Paths** | ✅ Faster | **58.07x** |
| **Pre-compiled Search Regex** | ✅ Faster | **22.41x** |
| **Cache Warmup** | ✅ Faster | **23.86x** |
| **Pattern Caching (LRU Cache)** | ✅ Faster | **7.76x** |
| **Pre-allocation (Many Fields)** | ✅ Faster | **7.81x** |
| **Mixed Patterns (Cache Management)** | ✅ Faster | **7.60x** |
| **Case-Insensitive Matching** | ✅ Faster | **6.04x** |
| **Findall (Multiple Matches)** | ✅ Faster | **2.88x** |

### Key Optimizations

- **Pattern Caching**: LRU cache (1000 patterns) eliminates regex compilation overhead
- **Regex Pre-compilation**: Pre-compiled search regex variants for faster search operations
- **Fast Type Conversion**: Optimized paths for common type conversions (int, float, bool)
- **Pre-allocation**: Pre-allocated vectors and HashMaps reduce memory allocations
- **Reduced GIL Overhead**: Batched Python operations minimize interpreter overhead
- **Custom Type Validation Caching**: Pre-computed validation results eliminate repeated Python attribute lookups
- **Reference-based Matching**: Eliminated HashMap cloning in hot paths for better performance
- **Lazy Results Conversion**: Custom `Results` object stores raw data and converts to Python objects only when accessed, with batch conversion on first iteration. This makes `findall` **2.88x faster** than the original library, including both the call and iteration overhead

### Running Benchmarks

```bash
# Standard comparison benchmarks
python scripts/benchmark.py

# Optimization-focused benchmarks
python scripts/benchmark_optimizations.py
```

## Compatibility

This library aims to be a **drop-in replacement** for the original `parse` library. The core functionality and API are compatible, but there may be subtle differences in edge cases.

### Known Differences

- Default `case_sensitive` parameter: `formatparse` uses `False` for `parse()` and `findall()` (matching original behavior), but `True` for `search()` (for consistency)
- Some edge cases in pattern matching may behave slightly differently due to Rust's regex engine

## Development

### Prerequisites

- Python 3.8+
- Rust 1.70+
- maturin (for building)

### Building

```bash
# Install dependencies
pip install maturin

# Build in development mode
maturin develop

# Build in release mode (optimized)
maturin develop --release
```

### Testing

```bash
# Run tests (requires pytest)
pytest tests/

# Or run tests manually
python -m pytest tests/ -v
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) file for details

## Credits

Based on the [parse](https://github.com/r1chardj0n3s/parse) library by Richard Jones.

## Links

- **GitHub**: [https://github.com/eddiethedean/formatparse](https://github.com/eddiethedean/formatparse)
- **Original parse library**: [https://github.com/r1chardj0n3s/parse](https://github.com/r1chardj0n3s/parse)
