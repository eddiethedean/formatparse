# formatparse

[![PyPI version](https://badge.fury.io/py/formatparse.svg)](https://badge.fury.io/py/formatparse)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.83+-orange.svg)](https://www.rust-lang.org/)
[![Documentation](https://readthedocs.org/projects/formatparse/badge/?version=latest)](https://formatparse.readthedocs.io/)

A Rust-backed reimplementation of the [parse](https://github.com/r1chardj0n3s/parse) library for Python. It targets the same `parse` / `search` / `findall` workflow with substantially lower overhead on hot paths ([performance guide](https://formatparse.readthedocs.io/en/latest/user_guides/performance.html)). Use it as a drop-in replacement for common `parse` patterns; see the [migration guide](https://formatparse.readthedocs.io/en/latest/user_guides/migration_from_parse.html) for edge-case differences.

## Documentation

**Full documentation:** [https://formatparse.readthedocs.io/](https://formatparse.readthedocs.io/)

- [Getting Started](https://formatparse.readthedocs.io/en/latest/user_guides/getting_started.html)
- [User Guides](https://formatparse.readthedocs.io/en/latest/user_guides/index.html) — patterns, datetime, custom types, validation, security
- [API Reference](https://formatparse.readthedocs.io/en/latest/api/index.html)
- [FAQ / Troubleshooting](https://formatparse.readthedocs.io/en/latest/user_guides/faq_troubleshooting.html)
- [Security](https://formatparse.readthedocs.io/en/latest/security.html) — read before parsing untrusted input
- [Changelog](https://formatparse.readthedocs.io/en/latest/changelog.html)

## Features

- **Fast**: Rust backend with pattern caching and batch APIs ([benchmarks](https://formatparse.readthedocs.io/en/latest/user_guides/performance.html))
- **Drop-in replacement**: Compatible with common `parse` usage ([migration guide](https://formatparse.readthedocs.io/en/latest/user_guides/migration_from_parse.html))
- **Pattern matching**: Named and positional fields, custom types, nested specs
- **DateTime parsing**: ISO 8601, log timestamps, strftime, and more
- **Case-sensitive and case-insensitive matching** ([matching behavior](https://formatparse.readthedocs.io/en/latest/user_guides/matching_behavior.html))
- **Validation and bidirectional patterns** for round-trip format/parse workflows

## Installation

```bash
pip install formatparse
```

From source, see [Installation](https://formatparse.readthedocs.io/en/latest/installation.html) in the docs.

## Quick Start

```python
from formatparse import parse, search, findall

result = parse("{name}: {age:d}", "Alice: 30")
print(result.named["name"], result.named["age"])  # Alice 30
```

More examples: [Getting Started](https://formatparse.readthedocs.io/en/latest/user_guides/getting_started.html).

### Malformed patterns: `parse` vs `compile`

For some invalid patterns (for example a missing `}` after a field), [`parse`](https://formatparse.readthedocs.io/en/latest/api/core_functions.html#formatparse.parse) returns `None` while [`compile`](https://formatparse.readthedocs.io/en/latest/api/core_functions.html#formatparse.compile) raises [`PatternParseMismatch`](https://formatparse.readthedocs.io/en/latest/api/exceptions.html#formatparse.PatternParseMismatch). See the [migration guide](https://formatparse.readthedocs.io/en/latest/user_guides/migration_from_parse.html).

### Custom types (`extra_types`)

Use `@with_pattern` and an `extra_types` dict — see the [Custom types guide](https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html). **Caching** and **pickling** notes are documented there and in [FAQ](https://formatparse.readthedocs.io/en/latest/user_guides/faq_troubleshooting.html).

## Contributing

See [CONTRIBUTING.md](https://github.com/eddiethedean/formatparse/blob/main/CONTRIBUTING.md) and the [Contributing docs page](https://formatparse.readthedocs.io/en/latest/contributing.html).

## Testing

- **Python**: 740+ tests (`pytest tests/`), coverage enforced at 86%+ in CI
- **Rust**: `formatparse-core` and `formatparse-pyo3` tests (`cargo test -p formatparse-core --all-targets`)

```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/
```

## License

MIT License — see [LICENSE](https://github.com/eddiethedean/formatparse/blob/main/LICENSE).

## Credits

Based on the [parse](https://github.com/r1chardj0n3s/parse) library by Richard Jones.
