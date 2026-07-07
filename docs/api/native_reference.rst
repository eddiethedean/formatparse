Native types reference
======================

``ParseResult``, ``FormatParser``, ``FindallIter``, and ``Results`` are implemented in
Rust (``_formatparse``). When the extension is built, :doc:`classes` and
:doc:`types_and_iterators` autoclass pages include live members. This page documents
the public surface for readers and for stubbed doc builds.

See also :repo:`_formatparse.pyi` for type-checker stubs.

ParseResult
-----------

- **named** — ``dict[str, Any]``: named field values (read-only mapping).
- **fixed** — ``tuple[Any, ...]``: positional field values (read-only).
- **span** — ``tuple[int, int]``: ``(start, end)`` character indices of the full match.
- **start** / **end** — ``int``: match bounds (same as ``span[0]`` / ``span[1]``).
- **spans** — ``dict[Hashable, tuple[int, int]]``: per-field spans keyed by name or
  positional index.
- **field_spans** — ``dict[str, tuple[int, int]]``: spans for named fields only.
- **\_\_getitem\_\_** — access by field name (``str``) or positional index (``int``).
- **\_\_contains\_\_** — test whether a field name is present.

FormatParser
------------

Compiled pattern object returned by :func:`~formatparse.compile`.

- **parse(string, case_sensitive=False, extra_types=None, evaluate_result=True)** —
  match the full string; returns :class:`~formatparse.ParseResult` or ``None``.
- **search(string, case_sensitive=True, extra_types=None, evaluate_result=True)** —
  find first match anywhere in ``string``.
- **findall_iter(string, case_sensitive=False, extra_types=None, evaluate_result=True, max_matches=None)** —
  iterator over matches.
- **named_fields** — list of named field names in the pattern.
- **field_constraints** — list of field constraint dicts from the pattern.
- **regex_subpattern** — regex subpattern string used internally.
- **regex_capturing_group_count** — number of capturing groups.
- **format** — object with **format(\*args)** for bidirectional formatting when the
  pattern supports it.

Pickling stores only the pattern string; pass ``extra_types`` again after
``pickle.loads`` when the pattern uses custom types.

FindallIter
-----------

Iterator returned by :func:`~formatparse.findall_iter` and
:meth:`~formatparse.FormatParser.findall_iter`. Implements ``__iter__`` and
``__next__`` yielding :class:`~formatparse.ParseResult` instances (or ``None`` at end).

Results
-------

List-like container returned by :func:`~formatparse.findall` on the default fast path.

- **\_\_len\_\_** — number of matches.
- **\_\_getitem\_\_** — index or slice of :class:`~formatparse.ParseResult` items.
- **\_\_iter\_\_** — iterate matches.

With ``extra_types``, ``evaluate_result=False``, or nested dict field names,
:func:`~formatparse.findall` returns a plain Python ``list`` instead.
