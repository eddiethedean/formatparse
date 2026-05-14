# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0] - 2026-05-15

### Fixed

- Post-parse validation: `_validator_field_value` returned `None` for valid `fixed` indices (indentation bug under the bounds check).
- `ValidationPipeline` in `collect` mode now runs all hooks even when field validators fail and merges field then hook errors in one `MultipleValidationErrors`.

### Changed

- PyO3: `extension-module` is an explicit Cargo feature (enabled by Maturin for wheels / `maturin develop`); `cargo test` / `cargo clippy` on `formatparse-pyo3` link libPython without extra linker configuration.

### Maintenance

- Pytest: skip `test_indent_block` when the installed `_formatparse` does not implement `:blk` / multiline validation (avoids false failures before `maturin develop`).
- CI, Makefile, wheel publish: pass `--features extension-module` when Maturin is run from `formatparse-pyo3/` so builds match root `pip install` behavior.

## [0.7.0] - 2026-05-14

### PyPI publish (post-tag workflow fix)

- Publish workflow: build sdist from repo root with `--manifest-path` (fixes “pyproject.toml not found” when run from `formatparse-pyo3/` only).
- Remove `[tool.maturin] python-source = "."` so `maturin sdist` does not require a repo-root `_formatparse` Python package (that check failed for this PyO3 cdylib layout on 1.12.x and 1.13.x). Publish and `[build-system].requires` use **`maturin>=1.12.6,<2.0`**.
- Linux wheel for CPython 3.8: use `manylinux2014` + `--manylinux 2014` when `manylinux_2_28` no longer ships that interpreter (avoids exit 127); pin `manylinux2014_x86_64:2026.05.01-2` because `latest` removed `/opt/python/cp38-cp38` after [manylinux#1882](https://github.com/pypa/manylinux/issues/1882).

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

[0.8.0]: https://github.com/eddiethedean/formatparse/releases/tag/v0.8.0
[0.7.0]: https://github.com/eddiethedean/formatparse/releases/tag/v0.7.0
