Nesting and composition
=======================

Nested field patterns
---------------------

When the substring after ``:`` in a field is a **balanced** nested brace pattern (for
example ``{outer:{inner:d}}``), the inner pattern is compiled and matched as part of the
outer capture, then parsed again. Nested groups appear as :class:`~formatparse.ParseResult`
objects under ``ParseResult.named``. Maximum nesting depth when compiling is **10**.

.. code-block:: python

   from formatparse import parse

   r = parse("{outer:{inner:d}}", "42")
   assert r.named["outer"].named["inner"] == 42

Brace-balanced scanning applies in the spec after ``:``. Literal braces in the **pattern**
string still use ``{{`` and ``}}`` for a single literal brace in pattern text.

Composition with ``composed_type``
----------------------------------

Use :func:`~formatparse.composed_type` to wrap a compiled :class:`~formatparse.FormatParser`
and pass it in ``extra_types`` so one field is parsed by the child parser and returns a
nested :class:`~formatparse.ParseResult`.

.. code-block:: python

   from formatparse import compile, composed_type

   ts = compile("{year:d}-{month:02d}-{day:02d}")
   log = compile(
       "{ts:Timestamp} [{level}] {msg}",
       extra_types={"Timestamp": composed_type(ts)},
   )
   r = log.parse("2024-01-15 [ERROR] oops")
   inner = r.named["ts"]
   assert inner.named["year"] == 2024

**Pickling:** a pickled :class:`~formatparse.FormatParser` stores only the pattern string.
After ``pickle.loads``, pass ``extra_types`` again when calling ``parse`` / ``search`` /
``findall`` if your pattern uses custom types—including any composed child parsers.

See also :doc:`patterns` (nested patterns section) and issues
`#7 <https://github.com/eddiethedean/formatparse/issues/7>`_,
`#12 <https://github.com/eddiethedean/formatparse/issues/12>`_.
