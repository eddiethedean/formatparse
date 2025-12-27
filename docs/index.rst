formatparse Documentation
==========================

Welcome to the formatparse documentation!

formatparse is a high-performance, Rust-backed implementation of the `parse <https://github.com/r1chardj0n3s/parse>`_ library for Python. It provides the same API as the original parse library but with **significant performance improvements** (up to **80x faster**) thanks to Rust's zero-cost abstractions and optimized regex engine.

Quick Start
-----------

.. testcode::

    from formatparse import parse, search, findall

    # Basic parsing with named fields
    result = parse("{name}: {age:d}", "Alice: 30")
    print(f"Name: {result.named['name']}")
    print(f"Age: {result.named['age']}")

.. testoutput::

    Name: Alice
    Age: 30

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   installation
   user_guides/index
   api/index
   examples/index
   security

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`

