Matching behavior
=================

This guide covers flags and defaults that affect how patterns match text.

Choosing an API
---------------

- :func:`~formatparse.parse` — the entire string must match the pattern (anchored).
- :func:`~formatparse.search` — find the first match anywhere in a longer string.
- :func:`~formatparse.findall` — collect all non-overlapping matches.
- :func:`~formatparse.compile` — reuse one pattern many times (best performance).
- :func:`~formatparse.findall_iter` — stream matches without building a full list.
- :func:`~formatparse.parse_batch` — parse many strings with the same pattern.

``case_sensitive``
------------------

Defaults:

- :func:`~formatparse.parse` — ``False``
- :func:`~formatparse.search` — ``True``
- :func:`~formatparse.findall` — ``False``
- :meth:`~formatparse.FormatParser.parse` — ``False``
- :meth:`~formatparse.FormatParser.search` — ``True``
- :meth:`~formatparse.FormatParser.findall_iter` — ``False``

When migrating from the original ``parse`` package, verify case sensitivity if
matches differ unexpectedly.

``evaluate_result``
-------------------

When ``True`` (default), captured text is converted to typed values (integers,
datetimes, etc.). When ``False``, :func:`~formatparse.findall` / :func:`~formatparse.findall_iter`
may yield raw :class:`Match` objects instead of :class:`~formatparse.ParseResult`.

``findall`` return type
-----------------------

- **Default fast path** — :class:`~formatparse.Results` (list-like)
- **`extra_types` provided** — plain ``list``
- **`evaluate_result=False`** — ``list`` of :class:`Match` / results
- **Nested dict field names** — plain ``list``

``search`` slice: ``pos`` and ``endpos``
----------------------------------------

:func:`~formatparse.search` searches only within ``string[pos:endpos]``. Both arguments
are **Unicode character indices** (like Python slicing), not UTF-8 byte offsets.

.. doctest::

   >>> from formatparse import search
   >>> search("x:{v:d}", "aaa x:1 bbb", pos=4).named['v']
   1
   >>> search("🚀{v:d}", "a🚀42", pos=1).named['v']
   42

``max_matches``
---------------

:func:`~formatparse.findall` and :func:`~formatparse.findall_iter` accept ``max_matches``
to cap how many non-overlapping matches are returned. Use this for untrusted input;
see :doc:`../security`.

Match spans
-----------

:class:`~formatparse.ParseResult` exposes:

- **span** — ``(start, end)`` of the full match
- **spans** — per-field spans keyed by name or index
- **field_spans** — named fields only

See :doc:`../api/native_reference`.
