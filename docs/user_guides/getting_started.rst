Getting Started
================

Installation
------------

Install formatparse from PyPI:

.. code-block:: bash

   pip install formatparse

Basic Usage
-----------

The main functions in formatparse are `parse()`, `search()`, and `findall()`.
These functions allow you to extract structured data from strings using
Python's format() syntax.

Parsing with Named Fields
-------------------------

The most common use case is parsing strings with named fields:

.. doctest::

   >>> from formatparse import parse
   >>> result = parse("{name}: {age:d}", "Alice: 30")
   >>> result.named['name']
   'Alice'
   >>> result.named['age']
   30

The ``:d`` in ``{age:d}`` tells formatparse to convert the matched value to an integer.

Searching in Text
-----------------

Use `search()` to find a pattern anywhere within a string:

.. doctest::

   >>> from formatparse import search
   >>> result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
   >>> result.named['age']
   30
   >>> result = search("age: {age:d}", "No age here")
   >>> result is None
   True

Finding All Matches
-------------------

Use `findall()` to find all non-overlapping occurrences of a pattern:

.. doctest::

   >>> from formatparse import findall
   >>> results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
   >>> len(results)
   3
   >>> results[0].named['id']
   1
   >>> results[1].named['id']
   2
   >>> results[2].named['id']
   3
   >>> for result in results:
   ...     print(result.named['id'])
   1
   2
   3

Understanding ParseResult
-------------------------

Both `parse()` and `search()` return a `ParseResult` object (or `None` if no match is found).
`findall()` usually returns a `Results` object (list-like) containing `ParseResult` objects; with ``extra_types``, ``evaluate_result=False``, or nested dict field names it returns a plain Python ``list`` of the same element types.

ParseResult has two main attributes:

- ``named``: A dictionary of named fields (read-only)
- ``fixed``: A tuple of positional fields (read-only)

.. doctest::

   >>> result = parse("{}, {}", "Hello, World")
   >>> result.fixed
   ('Hello', 'World')
   >>> result = parse("{greeting}, {name}", "Hello, World")
   >>> result.named['greeting']
   'Hello'
   >>> result.named['name']
   'World'

You can also access fields using dictionary-like syntax:

.. doctest::

   >>> result = parse("{name}: {age:d}", "Alice: 30")
   >>> result['name']
   'Alice'
   >>> result['age']
   30

Next Steps
----------

- Learn about :doc:`patterns` for detailed pattern syntax
- Explore :doc:`datetime_parsing` for parsing dates and times
- Check out :doc:`custom_types` for creating custom converters
- See :doc:`bidirectional_patterns` for round-trip parsing and formatting

