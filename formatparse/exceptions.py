"""Public exception types."""

from __future__ import annotations

from typing import Optional, Sequence, Union


class RepeatedNameError(ValueError):
    """Exception raised when a repeated field name has mismatched types.

    This exception is raised when a format pattern contains the same field name
    multiple times with different type specifications (e.g., ``"{age:d}"`` and
    ``"{age:f}"`` in the same pattern).

    :raises RepeatedNameError: When a repeated field name has mismatched types

    Example::

        >>> from formatparse import compile, RepeatedNameError
        >>> try:
        ...     compile("{age:d} years and {age:f} months")
        ... except RepeatedNameError as e:
        ...     print(f"Error: {e}")
    """

    pass


class ValidationWarning(UserWarning):
    """Issued when ``validation_mode='lenient'`` and a validator or hook fails."""

    pass


class ValidationError(ValueError):
    """Raised when a post-parse validator rejects a field.

    Subclass of :exc:`ValueError` for compatibility with ``except ValueError``.
    Validators should raise this (or let :func:`apply_validators` wrap other
    exceptions) so callers can inspect :attr:`field`.

    See `GitHub issue #10 <https://github.com/eddiethedean/formatparse/issues/10>`_.

    :param message: Human-readable reason the value was rejected.
    :param field: Parsed field identifier — ``str`` for a named field or ``int``
        for a positional ``fixed`` index (same convention as ``validators`` keys).
    """

    def __init__(
        self,
        message: str,
        *,
        field: Optional[Union[str, int]] = None,
    ) -> None:
        super().__init__(message)
        self.field = field


class MultipleValidationErrors(ValueError):
    """Raised when ``validation_mode='collect'`` and at least one validator fails.

    Not used for ``validation_mode='lenient'`` (failures are reported via
    :exc:`ValidationWarning` instead).

    For :func:`apply_validators`, :attr:`errors` lists each :exc:`ValidationError` in
    key order (all ``int`` keys ascending, then ``str`` keys alphabetically). For
    :meth:`ValidationPipeline.apply` in ``collect`` mode, field failures are listed
    first in that same order, followed by hook failures in hook registration order.
    """

    def __init__(self, errors: Sequence[ValidationError]) -> None:
        self.errors = tuple(errors)
        if not self.errors:
            super().__init__("validation failed")
        else:
            super().__init__("; ".join(str(e) for e in self.errors))
