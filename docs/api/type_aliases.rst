Type aliases
============

.. py:module:: formatparse.types

These names are re-exported from the top-level ``formatparse`` package.

ConverterProtocol
-----------------

.. autoclass:: formatparse.ConverterProtocol
   :members:

Protocol for custom type converters used in ``extra_types``. Implementations must
expose ``pattern`` (regex fragment), ``regex_group_count``, and be callable with
the matched text.

ExtraTypes
----------

Type alias: ``Dict[str, ConverterProtocol]``.

Mapping from custom type name (the token after ``:`` in a field) to a converter,
usually from :func:`~formatparse.with_pattern`.

FieldConstraint
---------------

.. autoclass:: formatparse.FieldConstraint
   :members:

Typed dict describing a field in a compiled pattern (name, type specifier, width,
precision). Exposed on :attr:`~formatparse.FormatParser.field_constraints`.

ValidationMode
--------------

.. py:data:: ValidationMode
   :module: formatparse

   Type alias: ``Literal["strict", "collect", "lenient"]`` — how :class:`~formatparse.ValidationPipeline`
   and post-parse validation report failures.

ValidatorMap
------------

.. py:data:: ValidatorMap
   :module: formatparse

   Type alias: ``Dict[Union[str, int], Callable[..., Any]]``.

   Keys are field names (:class:`str`) or positional indices (:class:`int`);
   values are callables invoked during post-parse validation.
