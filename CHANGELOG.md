# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Inline ``{...:validator(...)}`` syntax and **async** validation pipelines (currently deferred in API documentation).
- ``composed_type`` extensions: pattern ``+``, inheritance, and **flattening** nested parse results into the parent (see `#7 <https://github.com/eddiethedean/formatparse/issues/7>`_).

## [0.8.1] - 2026-05-15

### Changed

- **Python package layout**: split the monolithic ``formatparse`` module into focused submodules (``api``, ``validation``, ``bidirectional``, etc.) with unchanged ``from formatparse import …`` surface.
- **Rust internals**: phased SOLID refactor—``CompiledFields`` on ``FormatParser``, split ``types/regex`` and ``parser/matching`` modules, pattern compilation in ``formatparse-core``, unified builtin conversion via ``builtin_convert``, and ``format_parser`` / ``findall_iter`` module splits. No intended public API or match-semantics changes.

### Maintenance

- **Quality**: ``ruff``, ``mypy`` (check target 3.9+), ``cargo clippy -D warnings``, and ``rustfmt`` clean across the workspace.

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
- **Regex lookaround assertions** after the type token for **integer** (``:d`` and related) and **float** (``:f`` / ``:F``) fields: append ``(?=…)``, ``(?!…)``, ``(?<=…)``, or ``(?<!…)`` groups so the numeric capture stays zero-width outside the field span (issues `#9 <https://github.com/eddiethedean/formatparse/issues/9>`_, upstream `parse#209 <https://github.com/r1chardj0n3s/parse/issues/209>`_). Compiled format patterns use the **`fancy-regex`** engine; **simple literal** positive lookarounds may be rewritten as non-capturing groups for correct anchoring. Lookarounds are rejected inside strftime ``%…`` type tails for this release.
- **Nested format patterns** in a field's format specification (e.g. ``{outer:{inner:d}}``): the inner ``{…}`` is compiled as its own pattern, the outer capture is parsed again, and nested values appear as ``ParseResult`` objects under ``named`` (issue `#12 <https://github.com/eddiethedean/formatparse/issues/12>`_; upstream `parse#206 <https://github.com/r1chardj0n3s/parse/issues/206>`_). Brace-balanced scanning applies in the spec after ``:``; nesting depth is capped at 10; ``findall`` and ``FindallIter`` use the Python match path when nested fields are present (same rule as nested dict field names).
- **Input line continuations** for ``:ml`` and ``:blk`` captures: a backslash before end-of-line in the matched text joins lines (same odd/even backslash rules as pattern continuations in #68); leading spaces and tabs on the continued line are stripped (#80).

### Fixed

- **Empty string input** can match patterns whose fields use the default string conversion (`#`) (#16).
- **Post-parse validation**: `_validator_field_value` no longer returned `None` for valid `fixed` indices (incorrect `return` indentation under the bounds check).
- **`ValidationPipeline` `collect` mode**: all hooks still run when field validators fail; field and hook failures are merged into a single **`MultipleValidationErrors`** (field errors first in validator key order, then hook errors in registration order) (#11).
- **Float `f` / `F` with `.0` precision** (e.g. ``{:02.0f}``): accept integer-shaped captures such as ``20``, matching ``str.format`` output for those specs (#84; upstream `parse#159 <https://github.com/r1chardj0n3s/parse/issues/159>`_).
- **CI:** Skip pympler-based memory tests on **PyPy** (`SummaryTracker` / `asizeof` raises `KeyError` on PyPy 3.11).
- **CI / ReDoS guard:** Raise the post-regex-compile wall-clock cap from **200ms to 500ms** so slow shared runners (e.g. macOS Intel) do not spuriously reject valid patterns; documentation updated accordingly.
- **Pattern LRU cache:** after a hash hit, the entry is checked against the full normalized pattern and ``extra_types`` fingerprint so a colliding key cannot return the wrong compiled parser.
- **Fixed-width string fields with literals beside the field** (e.g. ``"{s:<5.5} "`` vs ``"abc   "``): when width and precision are equal, the compiled fragment is exactly ``prec`` characters so trailing (or leading) pattern space is not pulled into the capture (#97). **Right-aligned** fields with that same equal width and precision also accept an opaque fixed-width capture (including trailing spaces) so post-capture validation matches **parse** for those cells (#97).
- **String width/precision with literal newline after the field** (e.g. ``" {s:<4.4}\n"`` vs ``"     \n"``): bounded string fragments no longer use DOTALL ``.`` for content, so a trailing ``\n`` in the input is not consumed as part of the field (#95). The same bounded runs also exclude VT, FF, NEL (U+0085), U+2028 LINE SEPARATOR, and U+2029 PARAGRAPH SEPARATOR so those line boundaries cannot be absorbed under ``(?s)`` when they appear as literals after the field.
- **Integer and radix fields with both width and precision** (e.g. ``"{:2.2d}{:2.2d}"``, ``"#{:2.2x}{:2.2x}"``): digit runs use inclusive min/max bounds (parse semantics: width = minimum digits, precision = maximum) so adjacent fields no longer steal digits with a greedy ``+`` (#82; upstream `parse#107 <https://github.com/r1chardj0n3s/parse/issues/107>`_). Patterns that previously matched too loosely on short inputs may now return no match.
- **String fields with alignment + precision before a fixed-width integer** (e.g. ``"{n:>10.10}{x:02d}"``): leading fill in the regex is non-greedy so the slice for the width/precision content is left for the following digit field (#88; related `parse#218 <https://github.com/r1chardj0n3s/parse/issues/218>`_).
- **Default string fields next to literals** (e.g. ``"/{name}"``): an empty capture is allowed when it matches ``str.format`` output such as ``"/"`` for ``name=""`` (#83; upstream `parse#136 <https://github.com/r1chardj0n3s/parse/issues/136>`_). Applies to full-string **parse** / **compile().parse**; **search** / **findall** still use ``.+?`` for those segments so unanchored matching does not stop early.
- **Integer `d`**: leading spaces and tabs before decimal digits are accepted (e.g. ``parse("{a:d}", "    0")``) for parity with padded ``str.format`` output (#81; upstream `parse#133 <https://github.com/r1chardj0n3s/parse/issues/133>`_).

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
- **CI:** Standalone **Rust · formatparse-core (Linux)** job (separate row in Actions) plus matrix **Summary** table; matrix splits **Rust compile** / **Rust test** / **Python build** (`maturin build` + `pip install` wheel, no pytest until after `cargo test`) / **Python test** (`pytest`).
- **CI:** Ubuntu **PyPy 3.11** job; **``python-tests``** for ``formatparse-pyo3`` is a blocking step on Ubuntu CPython 3.11 (no ``continue-on-error``).
- **Rust:** clearer ``expect`` messages for UTF-8 invariants in line-continuation helpers and ``ResultsIterator``.

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

[0.8.1]: https://github.com/eddiethedean/formatparse/releases/tag/v0.8.1
[0.8.0]: https://github.com/eddiethedean/formatparse/releases/tag/v0.8.0
[0.7.0]: https://github.com/eddiethedean/formatparse/releases/tag/v0.7.0
