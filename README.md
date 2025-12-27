# formatparse

[![PyPI version](https://badge.fury.io/py/formatparse.svg)](https://badge.fury.io/py/formatparse)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://readthedocs.org/projects/formatparse/badge/?version=latest)](https://formatparse.readthedocs.io/)

A high-performance, Rust-backed implementation of the [parse](https://github.com/r1chardj0n3s/parse) library for Python. `formatparse` provides the same API as the original `parse` library but with **significant performance improvements** (up to **80x faster**) thanks to Rust's zero-cost abstractions and optimized regex engine.

## 📖 Documentation

**Full documentation is available at [https://formatparse.readthedocs.io/](https://formatparse.readthedocs.io/)**

The documentation includes:
- **Getting Started Guide** - Quick introduction and basic usage
- **User Guides** - Comprehensive guides on patterns, datetime parsing, custom types, and bidirectional patterns
- **API Reference** - Complete API documentation for all functions and classes
- **Examples & Cookbook** - Practical examples and common use cases

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
maturin develop --manifest-path formatparse-pyo3/Cargo.toml --release
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

For more examples and detailed usage, see the [documentation](https://formatparse.readthedocs.io/).

## Performance

formatparse is significantly faster than the original Python `parse` library, with speedups ranging from **3x to 80x** depending on the use case. The Rust backend provides:

- Pattern caching to eliminate regex compilation overhead
- Optimized type conversion paths for common types
- Efficient memory management with pre-allocated data structures
- Reduced Python GIL overhead through batched operations

For detailed benchmark results and performance analysis, see the [documentation](https://formatparse.readthedocs.io/).

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
