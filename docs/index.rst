formatparse Documentation
==========================

formatparse is a Rust-backed reimplementation of Python's `parse
<https://github.com/r1chardj0n3s/parse>`_ library. It targets the same
``parse`` / ``search`` / ``findall`` workflow with lower overhead on hot paths
(see :doc:`user_guides/performance` for benchmarks). Use it as a drop-in
replacement for common ``parse`` patterns; see :doc:`user_guides/migration_from_parse`
for edge-case differences.

**New here?** Read :doc:`user_guides/getting_started`, then :doc:`security` if you
parse untrusted input.

Quick Start
-----------

.. testcode::

    from formatparse import parse, search, findall

    result = parse("{name}: {age:d}", "Alice: 30")
    print(f"Name: {result.named['name']}")
    print(f"Age: {result.named['age']}")

.. testoutput::

    Name: Alice
    Age: 30

.. toctree::
   :maxdepth: 2
   :caption: Contents

   installation
   user_guides/index
   api/index
   examples/index
   security
   architecture
   contributing
   changelog

Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
