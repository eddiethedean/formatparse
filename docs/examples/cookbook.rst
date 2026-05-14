Cookbook
========

Common Patterns and Solutions
------------------------------

Parsing Log Files
~~~~~~~~~~~~~~~~~

Parse structured log entries:

.. doctest::

   >>> from formatparse import parse
   >>> log_line = "2024-01-15 10:30:00 [INFO] User logged in: alice"
   >>> result = parse("{timestamp} [{level}] {message}", log_line)
   >>> result.named['timestamp']
   '2024-01-15 10:30:00'
   >>> result.named['level']
   'INFO'
   >>> result.named['message']
   'User logged in: alice'

Extracting URLs
~~~~~~~~~~~~~~~

Extract components from URLs:

.. doctest::

   >>> result = parse("https://{host}/{path}", "https://example.com/api/users")
   >>> result.named['host']
   'example.com'
   >>> result.named['path']
   'api/users'

Parsing Configuration Files
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse key-value pairs:

.. doctest::

   >>> config_line = "database_host=localhost"
   >>> result = parse("{key}={value}", config_line)
   >>> result.named['key']
   'database_host'
   >>> result.named['value']
   'localhost'

Processing CSV-like Data
~~~~~~~~~~~~~~~~~~~~~~~~~

Extract data from delimited strings:

.. doctest::

   >>> result = parse("{name},{age:d},{city}", "Alice,30,NYC")
   >>> result.named['name']
   'Alice'
   >>> result.named['age']
   30
   >>> result.named['city']
   'NYC'

Custom types (``extra_types``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use :func:`with_pattern` and pass converters in ``extra_types`` (see the
:doc:`../user_guides/custom_types` guide):

.. doctest::

   >>> from formatparse import parse, with_pattern
   >>> @with_pattern(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}")
   ... def parse_email(text):
   ...     return text.lower()
   >>> r = parse("Contact {:Email}", "Contact User@Example.COM",
   ...           extra_types={"Email": parse_email})
   >>> r.fixed[0]
   'user@example.com'

Performance Tips
----------------

Use compile() for Repeated Patterns
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

If you're parsing the same pattern multiple times, compile it first:

.. testcode::

   from formatparse import compile
   
   parser = compile("{name}: {age:d}")
   results = []
   for text in ["Alice: 30", "Bob: 25", "Charlie: 35"]:
       result = parser.parse(text)
       if result:
           results.append(result)
   
   print(f"Parsed {len(results)} entries")

.. testoutput::

   Parsed 3 entries

Use findall() for Multiple Matches
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When searching for multiple occurrences, use `findall()`:

.. doctest::

   >>> from formatparse import findall
   >>> text = "ID:1 Name:Alice ID:2 Name:Bob ID:3 Name:Charlie"
   >>> results = findall("ID:{id:d} Name:{name}", text)
   >>> len(results)
   3
   >>> results[0].named['id']
   1
   >>> len(results[0].named['name']) > 0
   True

Best Practices
--------------

Handle None Results
~~~~~~~~~~~~~~~~~~~

Always check for None when using `parse()` or `search()`:

.. testcode::

   from formatparse import parse
   
   result = parse("{name}: {age:d}", "Invalid input")
   if result:
       print(f"Name: {result.named['name']}, Age: {result.named['age']}")
   else:
       print("Pattern did not match")

.. testoutput::

   Pattern did not match

Use Type Specifiers
~~~~~~~~~~~~~~~~~~~

Always use type specifiers when you need typed values:

.. doctest::

   >>> result = parse("{count:d}", "42")
   >>> result.named['count']
   42
   >>> type(result.named['count'])
   <class 'int'>

Validate Input with BidirectionalPattern
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Use `BidirectionalPattern` for input validation:

.. doctest::

   >>> from formatparse import BidirectionalPattern
   >>> formatter = BidirectionalPattern("{username:>10}: {score:05d}")
   >>> user_input = "     alice: 00100"
   >>> result = formatter.parse(user_input)
   >>> is_valid, errors = result.validate()
   >>> is_valid
   True
   >>> errors
   []

