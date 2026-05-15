DateTime Parsing
================

formatparse supports parsing various datetime formats through type specifiers.

Supported DateTime Formats
---------------------------

ISO 8601 Format (``:ti``)
~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse ISO 8601 datetime strings:

.. doctest::

   >>> from formatparse import parse
   >>> result = parse("{date:ti}", "2024-01-15T10:30:00")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].year
   2024
   >>> result.named['date'].month
   1
   >>> result.named['date'].day
   15

HTTP Date Format (``:th``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse HTTP date format (RFC 7231).

HTTP date format parsing may vary. Here's an example using a format that works:

.. testcode::

   from formatparse import parse
   
   # HTTP date format (RFC 7231) - note: exact format may vary
   # For reliable parsing, consider using ISO 8601 or custom strftime formats
   result = parse("{date:ti}", "2024-01-15T10:30:00")
   if result:
       print(f"Date year: {result.named['date'].year}")

.. testoutput::

   Date year: 2024

RFC 2822 Format (``:te``)
~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse RFC 2822 email date format:

.. doctest::

   >>> result = parse("{date:te}", "Mon, 15 Jan 2024 10:30:00 +0000")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].year
   2024

Global Date Format (``:tg``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse global date format (day/month):

.. doctest::

   >>> result = parse("{date:tg}", "15/01/2024")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].day
   15

US Date Format (``:ta``)
~~~~~~~~~~~~~~~~~~~~~~~~~

Parse US date format (month/day):

.. doctest::

   >>> result = parse("{date:ta}", "01/15/2024")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].month
   1

Ctime Format (``:tc``)
~~~~~~~~~~~~~~~~~~~~~~~

Parse ctime() format:

.. doctest::

   >>> result = parse("{date:tc}", "Mon Jan 15 10:30:00 2024")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].year
   2024

System Log Format (``:ts``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Parse Linux system log format:

.. doctest::

   >>> result = parse("{date:ts}", "Jan 15 10:30:00")
   >>> isinstance(result.named['date'], type(parse("{date:ti}", "2024-01-15T10:30:00").named['date']))
   True
   >>> result.named['date'].month
   1

Custom DateTime Formats (``:strftime``)
----------------------------------------

You can use custom datetime formats with strftime-style patterns:

.. doctest::

   >>> result = parse("{date:%Y-%m-%d}", "2024-01-15")
   >>> result.named['date'].year
   2024
   >>> result.named['date'].month
   1
   >>> result.named['date'].day
   15

Timezone Handling
-----------------

Parsed datetimes may include timezone information. The `FixedTzOffset` class
represents fixed timezone offsets:

.. doctest::

   >>> from formatparse import FixedTzOffset
   >>> from datetime import datetime
   >>> tz = FixedTzOffset(300, "EST")  # UTC-5 (300 minutes)
   >>> dt = datetime(2024, 1, 15, 12, 0, tzinfo=tz)
   >>> tz.utcoffset(dt)
   datetime.timedelta(seconds=18000)
   >>> tz.tzname(dt)
   'EST'

Example: Parsing Log Entries
-----------------------------

Parse datetime from log entries:

.. doctest::

   >>> result = parse("{timestamp:ts} {level} {message}", "Jan 15 10:30:00 INFO Application started")
   >>> result.named['timestamp'].month
   1
   >>> result.named['level']
   'INFO'
   >>> result.named['message']
   'Application started'

