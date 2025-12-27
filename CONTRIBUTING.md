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

4. **Build the extension module**
   ```bash
   maturin develop --manifest-path formatparse-pyo3/Cargo.toml --release
   ```

## Running Tests

### Python Tests

Run all Python tests:
```bash
pytest tests/ -v
```

Run specific test files:
```bash
pytest tests/test_basic.py -v
```

Run with coverage:
```bash
pytest tests/ --cov=formatparse --cov-report=html --cov-report=term
```

Run benchmarks:
```bash
pytest tests/test_performance.py --benchmark-only
```

Run stress tests:
```bash
pytest tests/test_stress.py -v
```

Run property-based tests:
```bash
pytest tests/test_property.py -v
```

### Rust Tests

Run Rust unit tests:
```bash
cargo test --package formatparse-core
```

Run tests for a specific module:
```bash
cargo test --package formatparse-core --lib types::regex
```

## Testing Guidelines

### Test Coverage Expectations

- **Target coverage**: >90% (aim for 95%+)
- All new code must include tests
- Critical paths should have 100% coverage
- Check coverage before submitting PRs:
  ```bash
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
pytest -m "not slow"  # Skip slow tests
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

- [ ] All tests pass (`pytest tests/`)
- [ ] Rust tests pass (`cargo test --package formatparse-core`)
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
- **Coverage**: Generated on Python 3.11, Ubuntu
- **Benchmarks**: Run on PRs and main branch
- **Doctests**: Run on Python 3.11, Ubuntu
- **Mutation Testing**: Runs weekly on main branch

## Getting Help

- Open an issue for questions or bug reports
- Check existing issues before creating new ones
- Be respectful and follow the code of conduct

Thank you for contributing to formatparse!

