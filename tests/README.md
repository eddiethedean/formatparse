# Test layout

Pytest collects `tests/test_*.py`. Use the same environment as CI: build the native extension (`maturin develop` from `formatparse-pyo3/`), install optional test deps (`pip install -e ".[test]"`), and set `PYTEST_DISABLE_PLUGIN_AUTOLOAD=1` (see [CONTRIBUTING.md](../CONTRIBUTING.md)).

## Inventory (by theme, rough line count)

| Theme | Files (approx. lines) |
|--------|-------------------------|
| Parse API / upstream parity | `test_parse.py` (~670), `parse_compat/test_parse_numbers.py`, `test_basic.py`, `test_result.py`, `test_parsetype.py` |
| Search / findall / iter | `test_search.py`, `test_findall.py`, `test_findall_iter.py`, `test_results.py` |
| Compile / cache / formatparser | `test_formatparser.py`, `test_compile_cache.py`, `test_pattern.py` |
| Types / numbers / alignment | `test_types.py`, `test_alignment_precision.py`, `test_integer_*.py`, `test_float_zero_precision.py`, `test_fill_character.py` |
| Datetime | `test_datetime.py` |
| Multiline / brace / pattern | `test_multiline.py`, `test_brace_delimited.py`, `test_pattern_continuations.py`, `test_indent_block.py` (native `:blk` probe in `conftest.py`) |
| Validation / pipeline | `test_validation.py`, `test_validation_pipeline.py`, `test_parse_with_validation.py`, `test_builtin_validators.py` |
| Composition / nested format | `test_composition.py`, `test_nested_format.py` |
| Bidirectional | `test_bidirectional.py` |
| Integration | `test_integration.py`, `test_integration_comprehensive.py` |
| Security / stress / perf / fuzz / memory | `test_security.py`, `test_stress.py`, `test_performance.py`, `test_fuzz.py`, `test_memory.py` |
| Property-based | `test_property.py` (~825) |
| Misc bugs / unicode / errors | `test_bugs.py`, `test_unicode.py`, `test_errors.py`, `test_lookaround.py`, `test_parse_batch.py` |

## Large files (split candidates)

- `test_parse.py` — upstream-style suite; numeric cases live in `parse_compat/test_parse_numbers.py`.
- `test_property.py` — Hypothesis-heavy.
- `test_integration_comprehensive.py` — broad scenarios.

## CI vs local Rust

GitHub Actions separates **compile** steps from **test** steps: `cargo build -p formatparse-core --all-targets` runs before the Python venv (fails fast on core compile errors). On **Ubuntu + CPython 3.11**, after `maturin develop` it runs `cargo build --workspace --all-targets` and `cargo test -p formatparse-pyo3 --features python-tests --no-run`, then installs pytest dependencies, then **runs** `cargo test` (full workspace plus `python-tests` execution), then **pytest**. Other matrix cells run `cargo test -p formatparse-core` only after the shared core compile step.

**PyPy 3.11** on Ubuntu runs the same pytest path with a `maturin develop` build; it does not run the full-workspace `cargo test` job (see **PyO3 `python-tests`** below).

## PyO3 `python-tests`

Opt-in Rust tests that embed a Python interpreter (`RawValue::to_py_object`, etc.): `cargo test -p formatparse-pyo3 --features python-tests` with `PYO3_PYTHON` set to your venv interpreter. On **Ubuntu + CPython 3.11**, CI first compiles those targets with `cargo test … --no-run`, then runs the same `cargo test` without `--no-run` (same environment as `maturin develop` in that job).
