# formatparse-pyo3

PyO3 bindings for [formatparse](https://github.com/eddiethedean/formatparse): the `_formatparse` native extension used by the Python `formatparse` package.

## crates.io vs PyPI

- **End users** should install from **PyPI** (`pip install formatparse`), which ships prebuilt wheels built with [maturin](https://github.com/PyO3/maturin).
- This crate is published to **crates.io** for transparency and for tooling that indexes Rust dependencies. The library target is a **`cdylib`** (`_formatparse`), not a normal Rust library artifact.

## Core engine

Pattern compilation and regex generation live in [`formatparse-core`](https://crates.io/crates/formatparse-core) on crates.io. This crate adds Python types, conversion, matching, and the `FormatParser` API surface.

## Building locally

From the repository root (with a Python venv):

```bash
cd formatparse-pyo3
maturin develop --release --features extension-module
```

See the repository `README.md` and `CONTRIBUTING.md` for full development setup.
