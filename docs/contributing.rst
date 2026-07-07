Contributing
============

Thank you for contributing to formatparse. This page summarizes how to get started;
the full guide is in the repository:

`CONTRIBUTING.md <https://github.com/eddiethedean/formatparse/blob/main/CONTRIBUTING.md>`_

Quick setup
-----------

1. Fork and clone the repository.
2. Create a virtual environment (Python 3.8+).
3. Install Rust 1.83+ from https://rustup.rs/
4. Build the extension::

      pip install "maturin>=1.13.3,<2.0"
      cd formatparse-pyo3 && maturin develop --release --features extension-module

5. Install test dependencies::

      pip install -e ".[test]"

Running tests
-------------

Set ``PYTEST_DISABLE_PLUGIN_AUTOLOAD=1`` (see CONTRIBUTING.md), then::

   pytest tests/ -v
   cargo test -p formatparse-core --all-targets

Documentation
-------------

Build docs locally (requires a built extension for full API pages)::

   pip install -r docs/requirements.txt
   maturin develop --release --features extension-module
   cd docs && python -m sphinx -W -b html . _build/html

Pull requests
-------------

- Run Python and Rust tests relevant to your change.
- Update user guides or API docs when behavior or public API changes.
- Follow existing code style (ruff, ``cargo fmt``, clippy).

Releases
--------

Maintainers: see `RELEASING.md <https://github.com/eddiethedean/formatparse/blob/main/RELEASING.md>`_.
