Multiline and indent-block fields
==================================

Use ``{name:ml}`` when a single field may span **newlines**. The capture is non-greedy
up to the next literal or the next field in the pattern. Width, precision, alignment,
and fill behave like plain string fields; sign, zero-padding, and ``=`` alignment are
not supported with ``:ml``.

Use ``{name:blk}`` for **indented blocks**: matching follows the same boundary rules as
``:ml``, then the captured text is **dedented** by removing the largest common prefix of
spaces and tabs from each non-blank line (blank lines do not set the margin; tabs count
as single characters).

Line continuations in **matched** text (for both ``:ml`` and ``:blk``): a single
backslash immediately before end-of-line joins the next line; doubled backslashes keep a
literal newline. Leading spaces and tabs on the continued line are stripped (same idea
as long **patterns** continued across lines).

**Pattern** line continuations: a backslash immediately before end-of-line continues the
format pattern on the next line (``\r\n`` or ``\n``); doubled backslashes keep a literal
newline in the pattern; leading spaces and tabs on the continued line are stripped.

**Caveat for ``:blk``:** if a pattern literal ends with trailing whitespace immediately
before ``{...:blk}``, it may compile as ``\s+`` and consume the block's leading margin.
Prefer keeping the newline **outside** that literal when you need a stable margin, for
example ``key:{body:blk}\nEND`` rather than ``key:\n{body:blk}``.

Examples
--------

.. code-block:: python

   from formatparse import compile, parse

   # Multiline capture between literals
   r = parse("BEGIN\n{body:ml}\nEND", "BEGIN\nalpha\nbeta\nEND")
   assert r.named["body"] == "alpha\nbeta"

   # Indented block under a header
   text = "summary:\n  line one\n  line two\nfooter"
   r = parse("summary:\n{body:blk}\nfooter", text)
   assert "line one" in r.named["body"]

Further reading: issues `#8 <https://github.com/eddiethedean/formatparse/issues/8>`_,
`#69 <https://github.com/eddiethedean/formatparse/issues/69>`_,
`#70 <https://github.com/eddiethedean/formatparse/issues/70>`_,
`#68 <https://github.com/eddiethedean/formatparse/issues/68>`_,
`#80 <https://github.com/eddiethedean/formatparse/issues/80>`_.
