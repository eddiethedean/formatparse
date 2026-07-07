Architecture
============

formatparse is a three-layer library: a Rust parsing core, PyO3 bindings, and a thin
Python API.

Layers
------

.. code-block:: text

   formatparse/          Python package (validation, bidirectional, compat)
        │
        ▼
   _formatparse          PyO3 extension (formatparse-pyo3)
        │
        ▼
   formatparse-core      Rust library (pattern → regex, match, convert)

**formatparse-core** — Pattern parsing, regex compilation, matching, type conversion,
and limits (pattern length, field count, compilation timing checks). Crate:
``formatparse-core``.

**formatparse-pyo3** — Exposes ``_formatparse`` to CPython/PyPy: ``parse``, ``search``,
``findall``, ``ParseResult``, ``FormatParser``, etc. Built with maturin as a
``cdylib`` with the ``extension-module`` feature for wheels.

**formatparse (Python)** — Wraps native calls in :mod:`formatparse.api`, adds
:class:`~formatparse.ValidationPipeline`, :class:`~formatparse.BidirectionalPattern`,
``@with_pattern`` / :func:`~formatparse.composed_type`, and compatibility aliases.

Data flow
---------

1. User passes a format pattern and input string to :func:`~formatparse.parse` or
   :func:`~formatparse.compile`.
2. The pattern is translated to a regex (with caching keyed by pattern and ``extra_types``).
3. The regex matches; captures are converted to Python values.
4. Optional Python-layer validators run on :class:`~formatparse.ParseResult`.

Where logic lives
-----------------

- **Pattern / regex engine** — ``formatparse-core``
- **ParseResult, matching** — ``formatparse-pyo3``
- **Validators, pipeline** — ``formatparse/validation.py``
- **Bidirectional format/parse** — ``formatparse/bidirectional.py``
- **Custom types decorator** — ``formatparse/custom.py``
- **parse compat aliases** — ``formatparse/compat.py``

Building and packaging
----------------------

- Version is defined in workspace [`Cargo.toml`](https://github.com/eddiethedean/formatparse/blob/main/Cargo.toml).
- Maturin reads ``pyproject.toml`` and builds the extension from ``formatparse-pyo3/``.
- Wheels include ``formatparse/**/*.py`` and the compiled ``_formatparse`` module.

Further reading
---------------

- :doc:`installation` — install from PyPI or source
- :doc:`contributing` — development setup
- :doc:`security` — production limits and ReDoS notes
- :doc:`user_guides/performance` — when Rust speedups apply
