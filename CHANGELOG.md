# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-05-14

### Added

- `formatparse_core::count_capturing_groups` for validating custom `with_pattern` regexes, including correct handling of `(?P<name>...)` named capture groups.
- Richer `ParseResult.__repr__` for REPL debugging.

### Fixed

- Pattern cache keys for `extra_types` now incorporate converter `pattern` and `regex_group_count` so cached parsers are not incorrectly reused.
- `Results` negative indexing aligned with Python list semantics.
- Zero-fill and right-align `width.precision` parsing parity with the reference `parse` library (#40).
- Trailing fill preserved for left-aligned string fields with width (#39).
- `formatparse.__version__` stable when installed from sdist/wheel (PEP 440) and fallback from `Cargo.toml` in source checkouts (#38).
- `FormatParser` pickling behavior documented and clarified for `extra_types`.

### Changed

- `findall` return type documentation: fast path returns `Results`; with `extra_types`, `evaluate_result=False`, or nested dict fields, a plain `list` is returned (matches runtime).
- Security documentation: clarify post-compile timing check vs match-time behavior; document uncapped `findall` match count within input limits.

### Maintenance

- CI: load `pytest-cov` when `PYTEST_DISABLE_PLUGIN_AUTOLOAD=1`; bump `cargo-audit` in the main CI Ubuntu step for advisory DB compatibility.
- Dependency updates (e.g. `lru` for RustSec advisories), formatting, and Clippy cleanups.

[0.7.0]: https://github.com/eddiethedean/formatparse/releases/tag/v0.7.0
