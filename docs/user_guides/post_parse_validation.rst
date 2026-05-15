Post-parse validation
=====================

After :func:`~formatparse.parse` or :meth:`formatparse.FormatParser.parse`, you can run
validators in several ways:

- :func:`~formatparse.apply_validators` on an existing :class:`~formatparse.ParseResult`.
- ``validators=`` or ``pipeline=`` on :func:`~formatparse.parse` (and related APIs).
- :func:`~formatparse.parse_with_validation` with a compiled parser and a
  :class:`~formatparse.ValidationPipeline`.
- :class:`~formatparse.ValidatedParser` as a thin wrapper around a compiled parser with
  default ``validators`` / ``pipeline`` on each :meth:`~formatparse.ValidatedParser.parse`.

:class:`~formatparse.ValidationPipeline` supports ordered per-field validators
(:meth:`~formatparse.ValidationPipeline.add_validator`), whole-result hooks
(:meth:`~formatparse.ValidationPipeline.add_hook`), and ``validation_mode`` of
``"strict"``, ``"collect"``, or ``"lenient"``. Field keys are field names (``str``) or
``fixed`` indices (``int``). In ``lenient`` mode, failures are reported with
:func:`warnings.warn` and the parse result is still returned.

Built-ins :func:`~formatparse.in_range`, :func:`~formatparse.non_empty_str`,
:func:`~formatparse.is_valid_email`, and :func:`~formatparse.is_valid_url` cover common
checks. Email and URL checks are **heuristic**, not full RFC compliance or a security audit.

.. code-block:: python

   from formatparse import ValidationPipeline, parse, in_range

   pipe = ValidationPipeline().add_validator("age", in_range(0, 150))
   r = parse("{name} {age:d}", "Ada 42", pipeline=pipe)
   assert r.named["age"] == 42

See :doc:`../api/validation` for the full API and issues
`#10 <https://github.com/eddiethedean/formatparse/issues/10>`_,
`#11 <https://github.com/eddiethedean/formatparse/issues/11>`_.
