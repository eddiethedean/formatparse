Core Functions
==============

parse
-----

.. autofunction:: formatparse.parse

.. note::

   For some malformed patterns (for example a missing ``}`` after a field), :func:`parse`
   returns ``None`` while :func:`compile` raises :exc:`formatparse.PatternParseMismatch`
   (a subclass of :exc:`ValueError`). Other invalid patterns may still raise plain
   :exc:`ValueError` from both APIs. This mirrors the original ``parse`` library.

search
------

.. autofunction:: formatparse.search

findall
-------

.. autofunction:: formatparse.findall

findall_iter
------------

.. autofunction:: formatparse.findall_iter

parse_batch
-----------

.. autofunction:: formatparse.parse_batch

parse_with_validation
----------------------

.. autofunction:: formatparse.parse_with_validation

compile
-------

.. autofunction:: formatparse.compile

with_pattern
------------

.. autofunction:: formatparse.with_pattern

