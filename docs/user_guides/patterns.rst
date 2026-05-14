Pattern Syntax
==============

formatparse uses Python's format() syntax for patterns. This guide explains the
various pattern elements and how to use them.

Field Syntax
------------

Basic Fields
~~~~~~~~~~~~

The simplest pattern is a named field:

.. doctest::

   >>> from formatparse import parse
   >>> result = parse("{name}", "Alice")
   >>> result.named['name']
   'Alice'

Positional fields use empty braces:

.. doctest::

   >>> result = parse("{}, {}", "Hello, World")
   >>> result.fixed
   ('Hello', 'World')

Type Specifiers
---------------

Type specifiers control how the matched text is converted. Common types include:

- ``:d`` - Integer (decimal)
- ``:f`` - Float
- ``:s`` - String (default)
- ``:b`` - Boolean

.. doctest::

   >>> result = parse("{age:d}", "30")
   >>> result.named['age']
   30
   >>> type(result.named['age'])
   <class 'int'>
   
   >>> result = parse("{price:f}", "3.14")
   >>> result.named['price']
   3.14
   >>> type(result.named['price'])
   <class 'float'>
   
   >>> result = parse("{active:b}", "1")
   >>> result.named['active']
   1
   >>> result = parse("{active:b}", "0")
   >>> result.named['active']
   0

Brace-delimited text in the input (``:brace``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use ``{name:brace}`` when the **source text** contains a literal ``{`` … ``}``
pair and you want the **inner** text as a string—including when it is empty
(``{}``). The inner match is **non-greedy**; if literal text in the pattern
follows the closing ``}``, the regex engine may match a **later** ``}`` so
the remainder of the string still matches (same rules as a normal regular
expression). Deeply nested ``{`` … ``}`` inside the payload is not supported
as a separate MVP (see `#15`).

.. doctest::

   >>> from formatparse import parse
   >>> line = "v:1 t:CON c:PUT i:cdcb {} [ Observe:0 ]"
   >>> pat = "v:1 t:CON c:PUT i:cdcb {payload:brace} [ Observe:0 ]"
   >>> r = parse(pat, line)
   >>> r.named["payload"]
   ''
   >>> line2 = "v:1 t:CON c:PUT i:cdcb {telemetry} [ Observe:0 ]"
   >>> pat2 = "v:1 t:CON c:PUT i:cdcb {payload:brace} [ Observe:0 ]"
   >>> parse(pat2, line2).named["payload"]
   'telemetry'

Pattern literals still use doubled braces ``{{`` and ``}}`` for a single
literal brace in the *pattern* string; ``:brace`` is only for matching
brace-wrapped **payload** in the **input**.

Nested format patterns in a field spec (``#12``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When the text after ``:`` is itself a balanced brace pattern that looks like a
mini format field (for example ``{inner:d}``), formatparse compiles it as an
inner pattern, matches it as part of the outer field, then parses the captured
substring again. Nested values show up as :class:`ParseResult` instances under
the outer field name (``result.named["outer"].named["inner"]``). Nesting depth
when compiling is capped at 10; ``{{`` / ``}}`` in the **pattern** string still
denote literal braces in pattern text—inside the nested spec substring, a ``}``
that closes an inner field is **not** merged with the following ``}`` that
closes the outer field.

.. doctest::

   >>> from formatparse import parse
   >>> r = parse("{outer:{inner:d}}", "42")
   >>> r.named["outer"].named["inner"]
   42

Format Specifiers
-----------------

Alignment
~~~~~~~~~

You can specify alignment using ``<`` (left), ``>`` (right), or ``^`` (center):

.. doctest::

   >>> result = parse("{name:>10}", "     Alice")
   >>> result.named['name']
   'Alice'
   
   >>> result = parse("{name:<10}", "Alice     ")
   >>> result.named['name']
   'Alice'
   
   >>> result = parse("{name:^10}", "  Alice   ")
   >>> result.named['name']
   'Alice'

Width
~~~~~

Specify minimum width with a number:

.. doctest::

   >>> result = parse("{value:05d}", "00042")
   >>> result.named['value']
   42

Precision
~~~~~~~~~

For floats, specify precision with ``.N``:

.. doctest::

   >>> result = parse("{value:.2f}", "3.14")
   >>> result.named['value']
   3.14

Combining Width and Precision
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

You can combine alignment, width, and precision:

.. doctest::

   >>> result = parse("{value:>10.5s}", "     Hello")
   >>> result.named['value']
   'Hello'

Integer width and precision (``#82`` / ``parse#107``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Python’s ``str.format`` does not allow a ``.precision`` on integer ``d`` presentation types.
formatparse still accepts ``width.precision`` in patterns: **width** is treated as a **minimum**
number of significant digits and **precision** as a **maximum** (inclusive), matching the
`parse <https://github.com/r1chardj0n3s/parse>`_ library documentation. That yields bounded
digit runs so several adjacent integer fields do not consume the string greedily.

.. code-block:: python

   from formatparse import parse

   assert parse("#{:2.2x}{:2.2x}{:2.2x}", "#FFFFFF").fixed == (255, 255, 255)
   assert parse("{:2.2d}{:2.2d}{:2.2d}", "123456").fixed == (12, 34, 56)
   assert parse("{:2.2d}{:2.2d}{:2.2d}", "9999") is None

Positional vs Named Fields
--------------------------

Named fields extract values into the ``named`` dictionary:

.. doctest::

   >>> result = parse("{greeting}, {name}", "Hello, World")
   >>> result.named['greeting']
   'Hello'
   >>> result.named['name']
   'World'

Positional fields extract values into the ``fixed`` tuple:

.. doctest::

   >>> result = parse("{}, {}", "Hello, World")
   >>> result.fixed[0]
   'Hello'
   >>> result.fixed[1]
   'World'

You can mix named and positional fields:

.. doctest::

   >>> result = parse("{name}, {} years old", "Alice, 30 years old")
   >>> result.named['name']
   'Alice'
   >>> result.fixed[0]
   '30'

Escaping Braces
---------------

Escaping braces in formatparse patterns requires special handling. In most cases,
you can include literal braces in the text you're parsing without special escaping
in the pattern itself. For complex cases involving literal braces, consider using
custom patterns or regex-based matching.

Advanced Examples
-----------------

Complex Format Specifiers
~~~~~~~~~~~~~~~~~~~~~~~~~

.. doctest::

   >>> result = parse("{name:>10}: {value:05d}", "     Alice: 00042")
   >>> result.named['name']
   'Alice'
   >>> result.named['value']
   42

Zero-Padding
~~~~~~~~~~~~

Zero-padding is useful for numeric IDs:

.. doctest::

   >>> result = parse("ID:{id:05d}", "ID:00042")
   >>> result.named['id']
   42

Scientific Notation
~~~~~~~~~~~~~~~~~~~

Use ``:e`` for scientific notation:

.. doctest::

   >>> result = parse("{value:e}", "1.5e10")
   >>> result.named['value'] == 15000000000.0
   True

