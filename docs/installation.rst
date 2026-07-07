Installation
============

From PyPI
---------

The easiest way to install formatparse is using pip:

.. code-block:: bash

   pip install formatparse

From Source
-----------

To install from source, you'll need Rust and maturin:

.. code-block:: bash

   # Clone the repository
   git clone https://github.com/eddiethedean/formatparse.git
   cd formatparse

   # Install maturin (build tool)
   pip install "maturin>=1.13.3,<2.0"

   # Build and install in development mode
   maturin develop --manifest-path formatparse-pyo3/Cargo.toml --release

Requirements
------------

- Python 3.8+
- Rust 1.83+ (for building from source)
- maturin (for building from source)

