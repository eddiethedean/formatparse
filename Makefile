.PHONY: test develop

PYTHON := $(if $(wildcard .venv/bin/python),.venv/bin/python,python3)

# Build the Rust extension (run from repo root; activate .venv first if you use it).
develop:
	cd formatparse-pyo3 && maturin develop --release --features extension-module

# Run the full test suite the same way CI does (explicit pytest plugins only).
test:
	PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 $(PYTHON) -m pytest tests/ $(PYTEST_ARGS)
