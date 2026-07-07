Getting Started
================

Installation
------------

Install from PyPI (see :doc:`../installation` for source builds and requirements):

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
`findall()` usually returns a `Results` object (list-like) containing `ParseResult` objects; with ``extra_types``, ``evaluate_result=False``, or nested dict field names it returns a plain Python ``list`` of the same element types. See :doc:`matching_behavior`.

ParseResult has two main attributes:

- ``named``: A dictionary of named fields (read-only)
- ``fixed``: A tuple of positional fields (read-only)

Match positions:

- ``span``: ``(start, end)`` character indices of the full match
- ``spans`` / ``field_spans``: per-field spans (see :doc:`../api/native_reference`)

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

Recommended reading order:

1. :doc:`patterns` — field syntax and type specifiers
2. :doc:`type_specifiers` — quick reference for ``:d``, ``:ti``, ``:ml``, etc.
3. :doc:`matching_behavior` — ``case_sensitive``, ``findall`` return types, ``pos`` / ``endpos``
4. :doc:`migration_from_parse` — if replacing the ``parse`` package
5. :doc:`../security` — before parsing untrusted patterns or inputs
6. :doc:`performance` — if parsing at scale
7. :doc:`../api/index` — full function reference
8. :doc:`faq_troubleshooting` — common questions

Also explore:

- :doc:`datetime_parsing` for dates and times
- :doc:`custom_types` for custom converters
- :doc:`bidirectional_patterns` for round-trip parsing and formatting

