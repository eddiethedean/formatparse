"""Post-parse validation."""

from __future__ import annotations

import re
import warnings
from typing import Any, Callable, Dict, Iterable, List, Optional, Tuple, Union
from urllib.parse import urlparse

from ._native import ParseResult
from .exceptions import MultipleValidationErrors, ValidationError, ValidationWarning
from .types import ValidationMode, ValidatorMap

_VALIDATION_MODES = frozenset({"strict", "collect", "lenient"})


def _check_validation_mode(mode: str) -> None:
    if mode not in _VALIDATION_MODES:
        raise ValueError(f"invalid validation_mode: {mode!r}")


def validator(func: Callable[..., Any]) -> Callable[..., Any]:
    """Mark a function as a post-parse validator (metadata for tooling / docs).

    Validators are invoked with the parsed value for one field; they should raise
    :exc:`ValidationError` on failure or return normally on success.

    :param func: Callable taking the parsed value (and optional extra args if you
        wrap it yourself before passing to :func:`apply_validators`).
    """
    setattr(func, "_formatparse_validator", True)
    return func


def _sorted_validator_keys(keys: Iterable[Union[str, int]]) -> List[Union[str, int]]:
    key_list = list(keys)
    ints = sorted(k for k in key_list if isinstance(k, int))
    strs = sorted(k for k in key_list if isinstance(k, str))
    if len(ints) + len(strs) != len(key_list):
        raise TypeError("validator keys must be str or int")
    return ints + strs


def _warn_validation_failure(err: ValidationError, *, kind: str) -> None:
    """Emit :exc:`ValidationWarning` for lenient mode (``kind`` is e.g. ``\"field\"`` or ``\"hook\"``)."""
    field = err.field
    base = str(err)
    if field is not None:
        msg = f"{kind} validation failed (field={field!r}): {base}"
    else:
        msg = f"{kind} validation failed: {base}"
    warnings.warn(msg, ValidationWarning, stacklevel=3)


def _validator_field_value(result: ParseResult, key: Union[str, int]) -> Any:
    if isinstance(key, str):
        if key not in result.named:
            raise ValidationError(
                f"no named field {key!r} in parse result",
                field=key,
            )
        return result.named[key]
    fixed = result.fixed
    if not isinstance(key, int) or key < 0 or key >= len(fixed):
        raise ValidationError(
            f"fixed field index {key!r} out of range (len(fixed)={len(fixed)})",
            field=key,
        )
    return fixed[key]


def _collect_field_validator_errors(
    result: ParseResult,
    validators: ValidatorMap,
) -> List[ValidationError]:
    """Run every per-field validator and return all failures (never raises)."""
    errors: List[ValidationError] = []
    for key in _sorted_validator_keys(validators.keys()):
        fn = validators[key]
        try:
            value = _validator_field_value(result, key)
        except ValidationError as e:
            errors.append(e)
            continue
        try:
            fn(value)
        except ValidationError as e:
            err = ValidationError(str(e), field=key)
            err.__cause__ = e.__cause__ if e.__cause__ is not None else e
            errors.append(err)
        except Exception as e:
            err = ValidationError(f"validator failed: {e}", field=key)
            err.__cause__ = e
            errors.append(err)
    return errors


def apply_validators(
    result: Optional[ParseResult],
    validators: Optional[ValidatorMap],
    *,
    mode: ValidationMode = "strict",
) -> Optional[ParseResult]:
    """Run post-parse validators on ``named`` / ``fixed`` values.

    Validators are **raise-based**: on success the callable returns (typically
    ``None``). On failure raise :exc:`ValidationError` (recommended) or any
    exception (wrapped in :exc:`ValidationError`). Replacing values in ``fixed``
    slots is not supported; use named fields or mutate ``result.named`` yourself
    after validation if you need coercion.

    :param result: Output of :func:`parse` / :meth:`FormatParser.parse`, or ``None``.
    :param validators: Map from **field key** to validator. Keys are ``str`` field
        names for :attr:`ParseResult.named` or ``int`` indices for :attr:`ParseResult.fixed`.
    :param mode: ``\"strict\"`` — stop on first error. ``\"collect\"`` — run all
        validators, then raise :exc:`MultipleValidationErrors` if any failed.
        ``\"lenient\"`` — run all validators, emit :exc:`ValidationWarning` for each
        failure, and always return ``result``.
    :returns: The same ``result`` reference after validation (including lenient runs
        with failures).
    :raises ValidationError: In ``strict`` mode when a validator fails.
    :raises MultipleValidationErrors: In ``collect`` mode when any validator fails.
    """
    if result is None or not validators:
        return result

    _check_validation_mode(mode)

    if mode == "lenient":
        for key in _sorted_validator_keys(validators.keys()):
            fn = validators[key]
            try:
                value = _validator_field_value(result, key)
            except ValidationError as e:
                _warn_validation_failure(e, kind="field")
                continue
            try:
                fn(value)
            except ValidationError as e:
                err = ValidationError(str(e), field=key)
                err.__cause__ = e.__cause__ if e.__cause__ is not None else e
                _warn_validation_failure(err, kind="field")
            except Exception as e:
                err = ValidationError(f"validator failed: {e}", field=key)
                err.__cause__ = e
                _warn_validation_failure(err, kind="field")
        return result

    if mode == "strict":
        for key in _sorted_validator_keys(validators.keys()):
            fn = validators[key]
            try:
                value = _validator_field_value(result, key)
            except ValidationError:
                raise
            try:
                fn(value)
            except ValidationError as e:
                err = ValidationError(str(e), field=key)
                err.__cause__ = e.__cause__ if e.__cause__ is not None else e
                raise err from err.__cause__
            except Exception as e:
                err = ValidationError(f"validator failed: {e}", field=key)
                err.__cause__ = e
                raise err from e
        return result

    errors = _collect_field_validator_errors(result, validators)
    if errors:
        raise MultipleValidationErrors(errors)
    return result


class ValidationPipeline:
    """Ordered registry of per-field validators and whole-result hooks (issue #11).

    Build with :meth:`add_validator` and/or :meth:`add_hook`, then :meth:`apply` on a
    :class:`ParseResult`. Per-field keys follow :func:`apply_validators` (``str`` for
    named fields, ``int`` for ``fixed`` indices). If the same field is registered twice,
    the **last** registration wins. Hooks run in registration order **after** the
    per-field validator pass completes (on success, or in ``lenient`` mode after every
    field validator has been attempted).

    Async validators and inline ``{...:validator(...)}`` syntax remain deferred
    (see issue #11).
    """

    __slots__ = ("_steps", "_hooks")

    def __init__(self) -> None:
        self._steps: List[Tuple[Union[str, int], Callable[..., Any]]] = []
        self._hooks: List[Callable[[ParseResult], None]] = []

    def add_validator(
        self,
        field: Union[str, int],
        fn: Callable[..., Any],
    ) -> ValidationPipeline:
        """Register ``fn`` for ``field``; returns ``self`` for chaining.

        :param field: Field name (``str``) or positional index (``int``).
        :type field: str or int
        :param fn: Callable invoked with the field value; raise :exc:`ValidationError` on failure.
        :type fn: callable
        :returns: ``self`` for method chaining.
        :rtype: ValidationPipeline
        """
        self._steps.append((field, fn))
        return self

    def add_hook(self, fn: Callable[[ParseResult], None]) -> ValidationPipeline:
        """Register a whole-result hook; runs after per-field validators. Chainable.

        :param fn: Callable taking the full :class:`~formatparse.ParseResult`.
        :type fn: callable
        :returns: ``self`` for method chaining.
        :rtype: ValidationPipeline
        """
        self._hooks.append(fn)
        return self

    def as_mapping(self) -> Dict[Union[str, int], Callable[..., Any]]:
        """Last registration per field wins (dict built in ``add_validator`` order)."""
        m: Dict[Union[str, int], Callable[..., Any]] = {}
        for k, fn in self._steps:
            m[k] = fn
        return m

    def apply(
        self,
        result: Optional[ParseResult],
        *,
        mode: ValidationMode = "strict",
    ) -> Optional[ParseResult]:
        """Run per-field validators, then registered hooks.

        If ``result`` is ``None``, returns ``None`` immediately (no validators or hooks).

        Hooks receive the full :class:`ParseResult` and use the same raise-based
        contract as :func:`apply_validators`. Failures are :exc:`ValidationError`
        (``field`` preserved when the raised error had one; otherwise ``None`` for
        generic hook failures). Other exceptions become :exc:`ValidationError` with
        ``field=None``.

        ``mode`` matches :func:`apply_validators`: ``strict`` stops on the first
        failure (field or hook). ``collect`` runs **all** per-field validators and
        **all** hooks, then raises a single :exc:`MultipleValidationErrors` listing field
        failures first (same key order as :func:`apply_validators`), then hook failures
        in hook registration order. ``lenient`` runs all field validators (warning on
        each failure), then all hooks (warning on each failure), and returns
        ``result`` without raising validation exceptions.
        """
        if result is None:
            return result
        _check_validation_mode(mode)
        if mode == "collect":
            errors = _collect_field_validator_errors(result, self.as_mapping())
            for h in self._hooks:
                try:
                    h(result)
                except ValidationError as e:
                    err = ValidationError(str(e), field=e.field)
                    err.__cause__ = e.__cause__ if e.__cause__ is not None else e
                    errors.append(err)
                except Exception as e:
                    err = ValidationError(f"hook failed: {e}", field=None)
                    err.__cause__ = e
                    errors.append(err)
            if errors:
                raise MultipleValidationErrors(errors)
            return result
        apply_validators(result, self.as_mapping(), mode=mode)
        if not self._hooks:
            return result
        if mode == "lenient":
            for h in self._hooks:
                try:
                    h(result)
                except ValidationError as e:
                    err = ValidationError(str(e), field=e.field)
                    err.__cause__ = e.__cause__ if e.__cause__ is not None else e
                    _warn_validation_failure(err, kind="hook")
                except Exception as e:
                    err = ValidationError(f"hook failed: {e}", field=None)
                    err.__cause__ = e
                    _warn_validation_failure(err, kind="hook")
            return result
        if mode == "strict":
            for h in self._hooks:
                try:
                    h(result)
                except ValidationError as e:
                    err = ValidationError(str(e), field=e.field)
                    err.__cause__ = e.__cause__ if e.__cause__ is not None else e
                    raise err from err.__cause__
                except Exception as e:
                    err = ValidationError(f"hook failed: {e}", field=None)
                    err.__cause__ = e
                    raise err from e
            return result


def in_range(
    min_value: Optional[Union[int, float]] = None,
    max_value: Optional[Union[int, float]] = None,
) -> Callable[[Union[int, float]], None]:
    """Return a validator that accepts numeric ``value`` within ``[min_value, max_value]``.

    :param min_value: Minimum allowed value (inclusive), or ``None`` for no lower bound.
    :type min_value: int or float, optional
    :param max_value: Maximum allowed value (inclusive), or ``None`` for no upper bound.
    :type max_value: int or float, optional
    :returns: Validator callable for use in :func:`~formatparse.apply_validators` or pipelines.
    """

    def check(value: Union[int, float]) -> None:
        if type(value) is bool:
            raise ValidationError("expected numeric value, got bool")
        if min_value is not None and value < min_value:
            raise ValidationError(
                f"expected value >= {min_value!r}, got {value!r}",
            )
        if max_value is not None and value > max_value:
            raise ValidationError(
                f"expected value <= {max_value!r}, got {value!r}",
            )

    return check


def non_empty_str(value: Any) -> None:
    """Reject ``None``, non-strings, or blank/whitespace-only strings.

    :param value: Field value after parsing.
    :type value: Any
    :raises ValidationError: If ``value`` is not a non-empty stripped string.
    """
    if not isinstance(value, str) or not value.strip():
        raise ValidationError("expected non-empty string")


# Practical ``user@host`` check (not full RFC 5322 / internationalized email).
_EMAIL_RE = re.compile(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")


def is_valid_email(value: Any) -> None:
    """Reject values that are not plausible ``user@domain`` mailbox strings.

    Uses a simple ASCII pattern suitable for post-parse validation only. It does
    not implement full RFC 5322 (no quoted-string local parts, no comments) and is
    not a deliverability or security check.

    :raises ValidationError: If ``value`` is not a non-empty string matching the pattern.
    """
    if not isinstance(value, str) or not value.strip():
        raise ValidationError("expected a non-empty string for email")
    if not _EMAIL_RE.fullmatch(value.strip()):
        raise ValidationError("invalid email address")


def is_valid_url(value: Any) -> None:
    """Reject values that are not ``http`` or ``https`` URLs with a non-empty host.

    Uses :func:`urllib.parse.urlparse`. This does not verify reachability, TLS, or
    that the resource exists.

    :raises ValidationError: If the string is empty, the scheme is not ``http``/``https``,
        or the parsed URL has no network location (host).
    """
    if not isinstance(value, str) or not value.strip():
        raise ValidationError("expected a non-empty string for URL")
    parsed = urlparse(value.strip())
    if parsed.scheme not in ("http", "https"):
        raise ValidationError("expected URL with http or https scheme")
    if not parsed.netloc:
        raise ValidationError("invalid URL: missing host")


def _validation_source_exclusive(
    *,
    validators: Optional[ValidatorMap],
    pipeline: Optional[ValidationPipeline],
) -> None:
    if validators is not None and pipeline is not None:
        raise ValueError("pass only one of validators= or pipeline=")


def validation_source_exclusive(
    *,
    validators: Optional[ValidatorMap],
    pipeline: Optional["ValidationPipeline"],
) -> None:
    if validators is not None and pipeline is not None:
        raise ValueError("pass only one of validators= or pipeline=")


def post_parse_validate(
    result: Optional[ParseResult],
    *,
    validators: Optional[ValidatorMap] = None,
    pipeline: Optional["ValidationPipeline"] = None,
    validation_mode: ValidationMode = "strict",
) -> Optional[ParseResult]:
    """Run optional validators or pipeline after parse (shared by parse / ValidatedParser)."""
    validation_source_exclusive(validators=validators, pipeline=pipeline)
    if pipeline is not None:
        return pipeline.apply(result, mode=validation_mode)
    return apply_validators(result, validators, mode=validation_mode)
