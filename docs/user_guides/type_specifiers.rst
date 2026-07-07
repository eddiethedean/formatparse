Type specifiers cheat sheet
============================

Quick reference for common field type codes (token after ``:`` in ``{name:spec}``).
See :doc:`patterns` for full syntax.

- ``d`` — integer (decimal)
- ``b`` — binary integer
- ``o`` — octal integer
- ``x`` / ``X`` — hex integer
- ``f`` / ``F`` — float
- ``e`` / ``E`` — scientific notation
- ``g`` / ``G`` — general numeric
- ``s`` — string (default for unnamed fields)
- ``ti`` — ISO 8601 datetime
- ``th`` — HTTP / log-style datetime
- ``te`` — RFC 2822 email date
- ``tg`` — global date (day/month)
- ``ta`` — US date (month/day)
- ``tc`` — ``ctime`` format
- ``ts`` — system log (``Jan 15 10:30:00``)
- ``%...`` — ``strftime``-style custom datetime
- ``ml`` — multiline string field
- ``blk`` — indented block (dedented after capture)
- ``brace`` — literal ``{`` / ``}`` in captures
- **Custom name** — key in ``extra_types`` (``@with_pattern``)

Width, alignment (``<``, ``>``, ``^``), fill, and precision follow Python
``format()``-style rules where supported. See :doc:`patterns` and
:doc:`datetime_parsing`.
