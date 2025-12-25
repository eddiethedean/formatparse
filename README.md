# structparse

A Rust-backed implementation of the [parse](https://github.com/r1chardj0n3s/parse) library for Python. This library provides the same API as the original `parse` library but with improved performance and reliability thanks to Rust.

## Installation

```bash
pip install structparse
```

Or build from source:

```bash
pip install maturin
maturin develop
```

## Usage

Parse strings using a specification based on the Python `format()` syntax:

```python
from structparse import parse

result = parse("Hello, {}!", "Hello, World!")
print(result.fixed)  # ['World!']

result = parse("{name}: {age:d}", "Alice: 30")
print(result.named)  # {'name': 'Alice', 'age': 30}
```

### Basic Parsing

```python
from structparse import parse

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
from structparse import search

result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
if result:
    print(result.named['age'])  # 30
```

### Finding All Matches

```python
from structparse import findall

results = findall("{word}", "hello world python")
for result in results:
    print(result.fixed[0])
# hello
# world
# python
```

### Custom Types

```python
from structparse import parse, with_pattern

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

This Rust-backed implementation should provide significant performance improvements over the pure Python implementation, especially for:
- Complex patterns with many fields
- Large input strings
- Repeated parsing operations

## License

MIT License - see LICENSE file for details

## Credits

Based on the [parse](https://github.com/r1chardj0n3s/parse) library by Richard Jones.

