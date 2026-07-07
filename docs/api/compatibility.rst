Compatibility with ``parse``
==============================

formatparse mirrors the original `parse <https://github.com/r1chardj0n3s/parse>`_ package
for common imports. See :doc:`../user_guides/migration_from_parse` for behavioral
differences.

Result
------

.. py:data:: Result
   :module: formatparse

   Alias for :class:`~formatparse.ParseResult`.

Parser
------

.. py:data:: Parser
   :module: formatparse

   Alias for :class:`~formatparse.FormatParser`.

dt_format_to_regex
------------------

.. py:data:: dt_format_to_regex
   :module: formatparse

   ``dict[str, str]`` mapping ``strftime`` format codes to regex fragments (compatibility
   with the original ``parse`` module).

__version__
-----------

.. py:data:: __version__
   :module: formatparse

   Package version string (read from workspace ``Cargo.toml`` at build time).
