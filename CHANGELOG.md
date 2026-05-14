# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0] - 2026-05-15

### Added

- **Multiline fields (`:ml`)** for captures that may span newlines, with greedy/non-greedy boundaries like plain string fields (#8). **Width, precision, alignment, and fill** are supported for `:ml` the same way as for string fields (#70, #75).
- **Indent-block fields (`:blk`)**: same boundary rules as `:ml`, then **dedent** by removing the largest common prefix of spaces and tabs on each line (blank lines do not set the margin; tabs count as single characters) (#69).
- **Pattern line continuations**: a backslash immediately before end-of-line continues the **format pattern** on the next line (`\r\n` or `\n`); doubled backslashes keep a literal newline; leading spaces and tabs on the continued line are stripped (#68).
- **`findall_iter`** for incremental iteration over `findall`-style matches (#13).
- **`parse_batch`** to parse many strings with one compiled parser (#14).
- **`:brace` field type** for capturing a literal `{...}` payload in the input (#15).
- **Datetime strftime**: merge adjacent strftime fragments that share a field name into a single datetime conversion (#4).
- **Composition**: embed a compiled `FormatParser` as a custom type so a field is parsed by a child parser and returns a nested `ParseResult`, via `composed_type` / `extra_types` (#7).
- **Post-parse validators**: `ValidationError`, `MultipleValidationErrors`, `apply_validators`, `validator` decorator, `parse` / `compile` / `ValidatedParser` with `validators=`, and built-ins **`in_range`** / **`non_empty_str`** (#10).
- **Validation pipeline** (`ValidationPipeline`): ordered per-field validators, `validation_mode` **`strict`** / **`collect`** / **`lenient`**, `parse(..., pipeline=...)`, whole-result **hooks** for cross-field checks, **`parse_with_validation`**, and built-ins **`is_valid_email`** / **`is_valid_url`** (heuristic checks, not full RFC compliance or security audits) (#11, #74–#78).

### Fixed

- **Empty string input** can match patterns whose fields use the default string conversion (`#`) (#16).
- **Post-parse validation**: `_validator_field_value` no longer returned `None` for valid `fixed` indices (incorrect `return` indentation under the bounds check).
- **`ValidationPipeline` `collect` mode**: all hooks still run when field validators fail; field and hook failures are merged into a single **`MultipleValidationErrors`** (field errors first in validator key order, then hook errors in registration order) (#11).
- **Float `f` / `F` with `.0` precision** (e.g. ``{:02.0f}``): accept integer-shaped captures such as ``20``, matching ``str.format`` output for those specs (#84; upstream `parse#159 <https://github.com/r1chardj0n3s/parse/issues/159>`_).

### Changed

- **`compile()`** uses the same pattern LRU cache as `parse` / `search` / `findall` when cache keys match (#29).
- **PyO3**: **`extension-module`** is an explicit Cargo feature enabled by Maturin for wheels and `maturin develop`; plain **`cargo test`** / **`cargo clippy`** on `formatparse-pyo3` link libPython without extra linker configuration.

### Documentation

- Expanded the **custom types / `extra_types`** user guide (#17).

### Maintenance

- **CI, Makefile, and publish workflow**: pass **`--features extension-module`** when Maturin is run from **`formatparse-pyo3/`** so builds match root **`pip install`** / **`maturin develop --manifest-path`** (Maturin does not read the repo-root `pyproject` from that working directory).
- **Pytest**: skip **`tests/test_indent_block.py`** when the installed **`_formatparse`** lacks **`:blk`** / multiline validation (skip message points to **`maturin develop`** / editable install).
- **Rust**: migrate off deprecated **`ToPyObject::to_object`** (#45); remove several crate-level Clippy allows (**`manual_strip`**, **`dead_code`**, **`wrong_self_convention`**, **`if_same_then_else`**, **`too_many_arguments`**, **`type_complexity`**) (#44, #46–#50).
- **CI**: Codecov Action **`files`** input (replaces deprecated **`file`**); Dependabot bumps for Rust crates and GitHub Actions.
- **Formatting**: Ruff-format touched Python tests and package sources.

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
