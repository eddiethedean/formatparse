# Contributing to formatparse

Thank you for your interest in contributing to formatparse! This document provides guidelines and instructions for contributing.

## Development Setup

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/your-username/formatparse.git
   cd formatparse
   ```

2. **Set up Python environment**
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   pip install --upgrade pip
   pip install maturin pytest
   pip install -e ".[test]"
   ```

3. **Set up Rust**
   - Install Rust from https://rustup.rs/
   - The project uses Rust stable

4. **Build the extension module** (from repo root; if `maturin develop` with `--manifest-path` errors about `_formatparse`, build from the crate directory instead):
   ```bash
   cd formatparse-pyo3 && maturin develop --release --features extension-module && cd ..
   ```

## Running Tests

Pytest loads only the plugins listed in `pyproject.toml` (`benchmark`, `hypothesispytest`, `pytest_cov`) when **`PYTEST_DISABLE_PLUGIN_AUTOLOAD=1`** is set. That avoids broken third-party `pytest11` plugins (for example an incompatible **pytest-asyncio**) crashing pytest before tests run. CI and `make test` set this automatically.

### Python Tests

Run all Python tests (uses `.venv/bin/python` when that interpreter exists):
```bash
make test
```

Faster feedback (skips `slow`, `stress`, and `benchmark` markers):
```bash
make test-fast
```

Or manually:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1   # omit on a clean venv with no conflicting plugins
pytest tests/ -v
```

Quick local run without slow tiers:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/ -v -m "not slow and not stress and not benchmark"
```

Run specific test files:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/test_basic.py -v
```

Run with coverage:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/ --cov=formatparse --cov-report=html --cov-report=term
```

Run benchmarks:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/test_performance.py --benchmark-only
```

Run stress tests:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/test_stress.py -v
```

Run property-based tests:
```bash
export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
pytest tests/test_property.py -v
```

### Rust Tests

Run Rust unit tests for the core crate only (fast; used on most CI matrix cells):
```bash
cargo test -p formatparse-core
```

Run the full Rust workspace (matches CI on **Ubuntu + Python 3.11**):
```bash
cargo test --workspace
```

Interpreter-linked PyO3 tests (opt-in feature; requires a working Python on `PATH` / `PYO3_PYTHON`). On macOS, linking can require the same interpreter PyO3 was configured against—use your venv’s `python`:
```bash
export PYO3_PYTHON="$(python -c 'import sys; print(sys.executable)')"
cargo test -p formatparse-pyo3 --features python-tests
```

GitHub Actions runs this on **Ubuntu + Python 3.11** as a **blocking** step (same venv as `maturin develop` in that job); failures fail the workflow.

Run tests for a specific module:
```bash
cargo test -p formatparse-core --lib types::regex
```

## Testing Guidelines

### Test Coverage Expectations

- **Enforced minimum**: `tool.coverage.report.fail_under` in `pyproject.toml` is checked on the **Ubuntu, Python 3.11** coverage job in CI (no `continue-on-error`). Raise it incrementally toward **90%+** as gaps close.
- All new code must include tests
- Critical paths should have high coverage
- Check coverage before submitting PRs:
  ```bash
  export PYTEST_DISABLE_PLUGIN_AUTOLOAD=1
  pytest tests/ --cov=formatparse --cov-report=term-missing
  ```

### Writing Tests

1. **Unit Tests**: Test individual functions and classes in isolation
2. **Integration Tests**: Test components working together
3. **Property-Based Tests**: Use Hypothesis for comprehensive input testing
4. **Performance Tests**: Use pytest-benchmark for performance regression testing
5. **Stress Tests**: Test with large inputs and many operations

### Test Organization

- Unit tests: `tests/test_*.py` (organized by module/feature)
- Upstream-style parse splits: `tests/parse_compat/` (smaller modules collected with the rest of `tests/`)
- Integration tests: `tests/test_integration*.py`
- Property-based tests: `tests/test_property.py`
- Performance tests: `tests/test_performance.py`
- Stress tests: `tests/test_stress.py`
- Fuzz tests: `tests/test_fuzz.py`

### Markers

Use pytest markers to categorize tests:

- `@pytest.mark.benchmark` - Performance benchmarks
- `@pytest.mark.slow` - Slow-running tests
- `@pytest.mark.stress` - Stress/load tests

Run tests by marker:
```bash
pytest -m "not slow and not stress and not benchmark"   # recommended default for local iteration
pytest -m "not slow"  # Skip slow tests only
pytest -m benchmark   # Run only benchmarks
```

## Performance Testing

### Running Benchmarks

```bash
pytest tests/test_performance.py --benchmark-only
```

### Performance Requirements

- New code should not significantly degrade performance
- Benchmarks are run in CI and compared against baselines
- Performance regressions >10% will fail CI

### Adding Benchmarks

Add benchmarks to `tests/test_performance.py`:

```python
@pytest.mark.benchmark
def test_my_feature_benchmark(benchmark):
    result = benchmark(my_function, arg1, arg2)
    assert result is not None
```

## Mutation Testing

Mutation testing validates that tests actually catch bugs. Run periodically:

```bash
pip install -e ".[dev]"
mutmut run
mutmut show
```

Mutation testing runs automatically in CI on the main branch (weekly).

## Code Style

### Python

- Follow PEP 8
- Use `ruff` for linting and formatting:
  ```bash
  ruff format .
  ruff check .
  ```

### Rust

- Follow Rust style guidelines
- Use `cargo fmt` to format:
  ```bash
  cargo fmt
  ```
- Use `cargo clippy` for linting:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

## Documentation

### Code Documentation

- Add docstrings to all public functions and classes
- Use Google-style docstrings
- Include examples in docstrings when helpful

### Documentation Testing

- All examples in documentation should be executable
- Run doctests: `cd docs && python -m sphinx -b doctest . _build/doctest`
- Doctests run automatically in CI

## Submitting Changes

1. **Create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Write code
   - Add tests
   - Update documentation
   - Ensure all tests pass

3. **Commit your changes**
   ```bash
   git add .
   git commit -m "Description of your changes"
   ```
   - Use clear, descriptive commit messages
   - Reference issues if applicable: "Fix #123: Description"

4. **Push and create a Pull Request**
   ```bash
   git push origin feature/your-feature-name
   ```
   - Create PR on GitHub
   - Fill out the PR template
   - Link related issues

## Pull Request Checklist

Before submitting a PR, ensure:

- [ ] All tests pass (`make test` or `PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 pytest tests/`)
- [ ] Rust tests pass (`cargo test --workspace` on the Ubuntu + Python 3.11 model; other setups at minimum `cargo test -p formatparse-core`). On Ubuntu 3.11-equivalent setups also run `cargo test -p formatparse-pyo3 --features python-tests` with `PYO3_PYTHON` set (see **Rust Tests**); CI enforces this on Ubuntu 3.11.
- [ ] Code coverage is maintained or improved
- [ ] Code is formatted (`ruff format .`, `cargo fmt`)
- [ ] No linting errors (`ruff check .`, `cargo clippy`)
- [ ] Documentation is updated
- [ ] Doctests pass
- [ ] Performance benchmarks pass (if applicable)
- [ ] Changelog is updated (if applicable)

## CI/CD

The project uses GitHub Actions for CI/CD:

- **Tests**: Run on all PRs across multiple Python versions and platforms
- **Coverage**: Generated on Python 3.11, Ubuntu; `fail_under` from `pyproject.toml` is enforced on that job
- **Rust**: `cargo test --workspace` on Ubuntu + Python 3.11; other cells run `cargo test -p formatparse-core` only. The `python-tests` step on Ubuntu 3.11 runs `cargo test -p formatparse-pyo3 --features python-tests` and **must pass** (see **Rust Tests** below). Ubuntu **PyPy 3.11** runs pytest with a built extension like the CPython matrix.
- **Benchmarks**: Run on PRs and main branch
- **Doctests**: Run on Python 3.11, Ubuntu
- **Mutation Testing**: Runs weekly on main branch

## Getting Help

- Open an issue for questions or bug reports
- Check existing issues before creating new ones
- Be respectful and follow the code of conduct

Thank you for contributing to formatparse!

