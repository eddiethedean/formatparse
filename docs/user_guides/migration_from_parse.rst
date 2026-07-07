Replacing ``parse`` (migration)
===============================

formatparse aims to be a **drop-in** replacement for the original `parse
<https://github.com/r1chardj0n3s/parse>`_ package for common uses, with a Rust-backed
implementation for speed.

API surface
-----------

Import :func:`~formatparse.parse`, :func:`~formatparse.search`, :func:`~formatparse.findall`,
:func:`~formatparse.compile`, :func:`~formatparse.with_pattern`, and result types from the
``formatparse`` package the same way you would from ``parse``. formatparse adds optional
features (validators, ``:ml`` / ``:blk``, nested specs, ``composed_type``, ``findall_iter``,
``parse_batch``, and more) documented elsewhere in this guide.

Malformed patterns
------------------

For some invalid patterns (for example a missing ``}`` after a field), :func:`~formatparse.parse`
returns ``None`` while :func:`~formatparse.compile` raises :exc:`~formatparse.PatternParseMismatch`
(a subclass of :exc:`ValueError`). Other syntax errors may still raise plain :exc:`ValueError`
from both APIs—this mirrors the original ``parse`` library.

Pattern cache and ``extra_types``
---------------------------------

``parse``, ``search``, ``findall``, and ``compile`` share an internal LRU cache keyed by the
pattern string and a fingerprint of ``extra_types`` (each converter's ``pattern`` and
``regex_group_count``). If you change a converter's ``pattern`` at runtime without changing
the dict object identity, you can still hit a stale cache entry—use a fresh ``extra_types``
dict or restart the process. See `issue #29 <https://github.com/eddiethedean/formatparse/issues/29>`_.

Pickling
--------

Pickling a :class:`~formatparse.FormatParser` stores only the pattern string. After
``pickle.loads``, pass ``extra_types`` again when parsing if your pattern uses custom types.

Compatibility aliases
---------------------

The original ``parse`` module exports some names that formatparse provides as aliases:

- :data:`~formatparse.Result` → :class:`~formatparse.ParseResult`
- :data:`~formatparse.Parser` → :class:`~formatparse.FormatParser`
- :data:`~formatparse.dt_format_to_regex` — strftime code to regex fragment map

See :doc:`../api/compatibility`.

Case sensitivity
----------------

:func:`~formatparse.search` defaults to ``case_sensitive=True``; :func:`~formatparse.parse`
and :func:`~formatparse.findall` default to ``False``. See :doc:`matching_behavior`.
