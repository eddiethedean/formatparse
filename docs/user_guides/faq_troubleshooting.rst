FAQ and troubleshooting
=======================

Why does ``parse`` return ``None``?
------------------------------------

No match was found, or the pattern is invalid in a way that ``parse`` treats as
non-matching (for example a missing ``}`` after a field). Use :func:`~formatparse.compile`
on the same pattern to see :exc:`~formatparse.PatternParseMismatch` for some syntax errors.

``parse`` vs ``compile`` on bad patterns
----------------------------------------

For some malformed patterns, :func:`~formatparse.parse` returns ``None`` while
:func:`~formatparse.compile` raises :exc:`~formatparse.PatternParseMismatch`. Other
errors may raise :exc:`ValueError` from both. See :doc:`migration_from_parse`.

Why does ``findall`` return a ``list`` sometimes?
-------------------------------------------------

By default, :func:`~formatparse.findall` returns a :class:`~formatparse.Results`
object (list-like). It returns a plain ``list`` when you pass ``extra_types``,
set ``evaluate_result=False``, or use nested dict field names. See
:doc:`matching_behavior`.

``search`` does not match but ``parse`` works
---------------------------------------------

:func:`~formatparse.search` defaults to ``case_sensitive=True``; :func:`~formatparse.parse`
and :func:`~formatparse.findall` default to ``case_sensitive=False``. Pass
``case_sensitive=False`` to ``search`` if you expect case-insensitive matching.

Custom types stopped working after I changed the regex
------------------------------------------------------

``parse``, ``search``, ``findall``, and ``compile`` cache compiled parsers by pattern
and a fingerprint of each converter's ``pattern`` and ``regex_group_count``. Mutating
a live converter without changing the ``extra_types`` dict identity can reuse a stale
entry. Use a fresh dict or new converter objects. See `issue #29
<https://github.com/eddiethedean/formatparse/issues/29>`_ and :doc:`custom_types`.

Pickled ``FormatParser`` loses custom types
-------------------------------------------

Pickling stores only the pattern string. After ``pickle.loads``, pass ``extra_types``
again when parsing if the pattern uses custom types.

Unicode ``pos`` / ``endpos`` in ``search``
------------------------------------------

:func:`~formatparse.search` treats ``pos`` and ``endpos`` as **Unicode character
indices**, not UTF-8 byte offsets. This matches Python string indexing.

Spans and field locations
-------------------------

Use :attr:`~formatparse.ParseResult.span`, :attr:`~formatparse.ParseResult.spans`, and
:attr:`~formatparse.ParseResult.field_spans` for match positions. See
:doc:`../api/native_reference`.

Where is the security guidance?
-------------------------------

See :doc:`../security` before parsing untrusted patterns or large inputs.

Getting more help
-----------------

- :doc:`getting_started` — first steps
- :doc:`migration_from_parse` — replacing the ``parse`` package
- :doc:`../api/index` — API reference
- `GitHub issues <https://github.com/eddiethedean/formatparse/issues>`_
