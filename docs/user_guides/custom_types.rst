Custom Types
============

formatparse allows you to define custom type converters using the `with_pattern()`
decorator. This enables parsing of custom formats and types.

Basic Custom Types
------------------

Use the `with_pattern()` decorator to create a custom type converter:

.. doctest::

   >>> from formatparse import parse, with_pattern
   >>> @with_pattern(r'\d+')
   ... def parse_number(text):
   ...     return int(text)
   >>> result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
   >>> result.fixed[0]
   42
   >>> type(result.fixed[0])
   <class 'int'>

The decorator adds a ``pattern`` attribute to your function, which formatparse
uses to match the text before calling your converter function.

Compile-time ``extra_types``
----------------------------

Custom-type regex fragments are fixed when the pattern is **compiled**. Pass
``extra_types`` to :func:`compile` (or use :func:`parse` / :func:`search` / :func:`findall`,
which compile through the shared cache with your mapping) so fields like ``{:Number}``
use your ``@with_pattern`` regex. If you :func:`compile` without ``extra_types`` and only
supply them on :meth:`FormatParser.parse`, formatparse still merges converters at call
time and re-resolves the cached parser so matching stays consistent with top-level
:func:`parse` (strict regex, ``None`` on mismatch—not a loose ``\S+`` capture followed by
a converter error).

Regex Patterns
--------------

The pattern parameter is a regular expression that defines what text should match:

.. doctest::

   >>> @with_pattern(r'[A-Z]{2,3}')
   ... def parse_code(text):
   ...     return text.upper()
   >>> result = parse("Code: {:Code}", "Code: abc", {"Code": parse_code})
   >>> result.fixed[0]
   'ABC'

Regex Groups
------------

If your regex pattern contains capturing groups (parentheses), specify the number
of groups using the ``regex_group_count`` parameter:

.. doctest::

   >>> @with_pattern(r'(\d+)-(\d+)', regex_group_count=2)
   ... def parse_range(text):
   ...     # text will contain the full match, groups are available separately
   ...     return tuple(map(int, text.split('-')))
   >>> result = parse("Range: {:Range}", "Range: 10-20", {"Range": parse_range})
   >>> result.fixed[0]
   (10, 20)

Integration with parse/search/findall
--------------------------------------

Custom types work with all parsing functions:

.. doctest::

   >>> @with_pattern(r'\d+\.\d+')
   ... def parse_version(text):
   ...     return tuple(map(int, text.split('.')))
   >>> result = parse("Version: {:Version}", "Version: 1.2", extra_types={"Version": parse_version})
   >>> result.fixed[0]
   (1, 2)
   
   >>> from formatparse import search
   >>> result = search("v{:Version}", "Current version v2.5 installed", extra_types={"Version": parse_version})
   >>> result.fixed[0]
   (2, 5)
   
   Note: search() finds the first match in the string.
   
   >>> from formatparse import findall
   >>> results = findall("v{:Version}", "v1.0 v2.0 v3.0", {"Version": parse_version})
   >>> len(results)
   3
   >>> results[0].fixed[0]
   (1, 0)

Advanced Examples
-----------------

Parsing IP Addresses
~~~~~~~~~~~~~~~~~~~~

.. doctest::

   >>> @with_pattern(r'\d+\.\d+\.\d+\.\d+')
   ... def parse_ip(text):
   ...     return tuple(map(int, text.split('.')))
   >>> result = parse("IP: {:IP}", "IP: 192.168.1.1", {"IP": parse_ip})
   >>> result.fixed[0]
   (192, 168, 1, 1)

Parsing Enumerations
~~~~~~~~~~~~~~~~~~~~

.. doctest::

   >>> STATUS_MAP = {'active': True, 'inactive': False, 'pending': None}
   >>> @with_pattern(r'active|inactive|pending')
   ... def parse_status(text):
   ...     return STATUS_MAP.get(text.lower())
   >>> result = parse("Status: {:Status}", "Status: active", {"Status": parse_status})
   >>> result.fixed[0]
   True

Complex Parsing
~~~~~~~~~~~~~~~

You can combine multiple custom types in a single pattern:

.. doctest::

   >>> @with_pattern(r'\d+')
   ... def parse_id(text):
   ...     return int(text)
   >>> @with_pattern(r'[A-Z]+')
   ... def parse_category(text):
   ...     return text
   >>> result = parse("Item {:ID} in category {:Cat}", "Item 42 in category TOOLS", 
   ...                extra_types={"ID": parse_id, "Cat": parse_category})
   >>> result.fixed[0]
   42
   >>> result.fixed[1]
   'TOOLS'
   
   Note: Positional fields (without names) are stored in ``fixed``, not ``named``.

