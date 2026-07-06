Validation
==========

ValidationPipeline
------------------

.. autoclass:: formatparse.ValidationPipeline
   :members:
   :undoc-members:

apply_validators
----------------

.. autofunction:: formatparse.apply_validators

validator
---------

.. autofunction:: formatparse.validator

Built-in validators
-------------------

.. autofunction:: formatparse.in_range

.. autofunction:: formatparse.non_empty_str

.. autofunction:: formatparse.is_valid_email

.. autofunction:: formatparse.is_valid_url

Type aliases
------------

.. autodata:: formatparse.ValidationMode
   :annotation:

``ValidatorMap``
~~~~~~~~~~~~~~~~

.. py:data:: ValidatorMap
   :module: formatparse

   Type alias: ``Dict[Union[str, int], Callable[..., Any]]``.

   Keys are field names (:class:`str`) or positional indices (:class:`int`);
   values are callables invoked during post-parse validation.
