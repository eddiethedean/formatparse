"""
Parse strings using a specification based on the Python format() syntax.

This is a Rust-backed implementation of the parse library for better performance.

**Custom types:** pass a mapping as ``extra_types`` from each type name used in the
pattern (e.g. ``{:Number}`` → key ``\"Number\"``) to a callable, usually decorated
with :func:`with_pattern`. See :func:`compile` and the `Custom types user guide
<https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html>`_.

**Multiline text:** use ``{name:ml}`` when a single field may span newlines; the
capture is non-greedy up to the next literal or field. Width, precision, alignment,
and fill are supported like plain string fields (see `GitHub issue #70
<https://github.com/eddiethedean/formatparse/issues/70>`_); sign, zero-padding, and
``=`` alignment are not supported with ``:ml``.

**Indented blocks:** use ``{name:blk}`` when content is written as an indented block
under a header: matching uses the same rules as ``:ml`` (non-greedy up to the next
literal or field), then the captured text is **dedented** by removing the largest
common prefix of spaces and tabs from each line (blank lines do not contribute to
that margin; tabs count as single characters). See `GitHub issue #69
<https://github.com/eddiethedean/formatparse/issues/69>`_. If a pattern literal ends
with trailing whitespace before ``{...:blk}``, it is compiled as ``\\s+`` and may
consume the block's leading spaces—keep the newline outside that literal chunk when
you need a margin (e.g. ``key:{body:blk}\\nEND`` rather than ``key:\\n{body:blk}``).

**Long patterns:** a backslash immediately before the end of a line continues the
pattern on the next line (``\\r\\n`` or ``\\n``); doubled backslashes keep a literal
newline. Leading spaces and tabs on the continued line are stripped (see `GitHub
issue #68 <https://github.com/eddiethedean/formatparse/issues/68>`_).

**Float zero precision:** for ``f`` / ``F`` fields with ``.0`` precision (e.g. ``{:02.0f}``),
``str.format`` often emits digits only (no decimal point). Parsing accepts that same
integer-like text as well as forms with a trailing dot or trailing fractional zeros
(see `GitHub issue #84 <https://github.com/eddiethedean/formatparse/issues/84>`_).

**Composition (sub-parsers):** use :func:`composed_type` to wrap a compiled
:class:`FormatParser` and pass it in ``extra_types`` so one field is parsed by the
child parser and returns a nested :class:`ParseResult`. The parent pickle story is
unchanged (only the parent pattern string is restored); re-supply ``extra_types``
after unpickling, including any composed child parsers (see `GitHub issue #7
<https://github.com/eddiethedean/formatparse/issues/7>`_).

**Post-parse validators:** after :func:`parse` or :meth:`FormatParser.parse`, run
callables with :func:`apply_validators`, or pass ``validators=`` / ``pipeline=`` to
:func:`parse`, or use :func:`parse_with_validation` with a compiled parser and a
:class:`ValidationPipeline`. See `GitHub issue #10 <https://github.com/eddiethedean/formatparse/issues/10>`_
and `GitHub issue #11 <https://github.com/eddiethedean/formatparse/issues/11>`_.
Use :class:`ValidationPipeline` for per-field steps (:meth:`~ValidationPipeline.add_validator`)
and whole-result hooks (:meth:`~ValidationPipeline.add_hook`), then
:meth:`~ValidationPipeline.apply`. Field keys are names (``str``) or ``fixed`` indices
(``int``). ``validation_mode='lenient'`` logs validator and hook failures with
:func:`warnings.warn` and still returns the :class:`ParseResult`. Built-ins
:func:`in_range`, :func:`non_empty_str`, :func:`is_valid_email`, and :func:`is_valid_url`
help with common checks (email/URL are heuristic, not full RFC or security audits).
Inline ``{...:validator(...)}`` syntax and async pipelines are deferred.
"""

from __future__ import annotations

from datetime import datetime, timedelta, tzinfo
from typing import (
    Any,
    Callable,
    Dict,
    Iterable,
    Iterator,
    List,
    Literal,
    Mapping,
    Optional,
    Protocol,
    Sequence,
    Tuple,
    TypedDict,
    Union,
)
import re
import warnings
from importlib.metadata import PackageNotFoundError, version as _package_version
from urllib.parse import urlparse

# PEP 440 string for the installed distribution; falls back to workspace Cargo.toml
# when running from a source checkout without package metadata (see issue #38).
try:
    __version__ = _package_version("formatparse")
except PackageNotFoundError:
    from pathlib import Path

    _cargo = Path(__file__).resolve().parent.parent / "Cargo.toml"
    try:
        _m = re.search(r'version\s*=\s*"([^"]+)"', _cargo.read_text(encoding="utf-8"))
        __version__ = _m.group(1) if _m else "0.0.0"
    except OSError:
        __version__ = "0.0.0"

# Import from the Rust extension module
from _formatparse import (  # type: ignore[import-not-found, import-untyped]
    parse as _parse,
    parse_batch as _parse_batch,
    search as _search,
    findall as _findall,
    findall_iter as _findall_iter,
    compile as _compile,
    ParseResult,
    FormatParser,
    FindallIter,
    FixedTzOffset as _FixedTzOffset,
    Results,
)


# Type definitions for custom converters
class ConverterProtocol(Protocol):
    """Protocol for custom type converter functions."""

    pattern: str
    regex_group_count: int

    def __call__(self, text: str) -> Any:
        """Convert a text string to a value."""
        ...


# Type alias: mapping from custom type name (e.g. ``Number`` in ``{:Number}``) to a
# callable with ``pattern`` and optional ``regex_group_count`` (see :func:`with_pattern`).
ExtraTypes = Dict[str, ConverterProtocol]


# TypedDict for field constraints
class FieldConstraint(TypedDict, total=False):
    """Type for field constraint dictionaries."""

    name: Optional[str]
    type: str
    width: Optional[int]
    precision: Optional[int]


# Define RepeatedNameError exception (matches original parse library)
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


ValidationMode = Literal["strict", "collect", "lenient"]
ValidatorMap = Mapping[Union[str, int], Callable[..., Any]]


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
    ints = sorted(k for k in keys if isinstance(k, int))
    strs = sorted(k for k in keys if isinstance(k, str))
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
        """Register ``fn`` for ``field``; returns ``self`` for chaining."""
        self._steps.append((field, fn))
        return self

    def add_hook(self, fn: Callable[[ParseResult], None]) -> ValidationPipeline:
        """Register a whole-result hook; runs after per-field validators. Chainable."""
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
    """Return a validator that accepts numeric ``value`` within ``[min_value, max_value]``."""

    def check(value: Union[int, float]) -> None:
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
    """Reject ``None``, non-strings, or blank/whitespace-only strings."""
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


# Wrap compile to catch RepeatedNameError
def compile(pattern: str, extra_types: Optional[ExtraTypes] = None) -> FormatParser:
    """Compile a pattern into a FormatParser for repeated use.

    Compiling a pattern allows you to reuse the same pattern multiple times
    without recompiling the regex, which improves performance for repeated
    parsing operations.

    Repeated ``compile`` calls with the same *pattern* and equivalent
    ``extra_types`` (same converter ``pattern`` and ``regex_group_count`` per
    name) share the same internal compiled-regex cache as :func:`parse`,
    :func:`search`, and :func:`findall`, so hot loops that call ``compile`` do
    not pay full pattern-to-regex compilation on every iteration (see
    `issue #29 <https://github.com/eddiethedean/formatparse/issues/29>`_).

    **Custom types:** keys are the *type names* used after ``:`` in fields (for example
    ``Number`` in ``{:Number}`` or ``{x:Number}``). Values are callables, usually from
    :func:`with_pattern`, which attach a ``pattern`` regex fragment and optional
    ``regex_group_count`` when the regex contains capturing parentheses. See the
    `Custom types guide <https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html>`_
    for examples with :func:`search` / :func:`findall` and for ``regex_group_count``.

    The cache fingerprints each name's ``pattern`` and ``regex_group_count``. If you
    mutate those attributes on a live converter object, reuse the same ``extra_types``
    dict, and the fingerprint stays unchanged, you can see a stale compiled parser until
    the process restarts—prefer a fresh dict or new function objects when changing
    patterns at runtime.

    :param pattern: Format specification pattern (e.g., ``"{name}: {age:d}"``)
    :type pattern: str
    :param extra_types: Optional mapping of custom type names to converters (see above)
    :type extra_types: dict, optional
    :returns: FormatParser object that can be used to parse strings
    :rtype: FormatParser
    :raises RepeatedNameError: If a repeated field name has mismatched types
    :raises ValueError: If pattern is invalid

    **Pickling:** A :class:`FormatParser` only round-trips the pattern string.
    If you compiled with ``extra_types``, unpickling yields a parser **without**
    those converters; call :func:`compile` again with the same ``extra_types``
    if you need them after ``pickle.loads``.

    Example::

        >>> parser = compile("{name}: {age:d}")
        >>> result = parser.parse("Alice: 30")
        >>> result.named['name']
        'Alice'
        >>> result.named['age']
        30
        >>> result2 = parser.parse("Bob: 25")
        >>> result2.named['name']
        'Bob'
        >>> result2.named['age']
        25
    """
    try:
        return _compile(pattern, extra_types)
    except ValueError as e:
        if "Repeated name" in str(e) and "mismatched types" in str(e):
            raise RepeatedNameError(str(e)) from e
        raise


# Wrap parse, search, findall to match original API
def parse(
    pattern: str,
    string: str,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
    *,
    validators: Optional[ValidatorMap] = None,
    pipeline: Optional[ValidationPipeline] = None,
    validation_mode: ValidationMode = "strict",
) -> Optional[ParseResult]:
    """Parse a string using a format specification.

    This function parses a string according to a format pattern and extracts
    named or positional fields from it. The pattern syntax is based on Python's
    format() function syntax.

    :param pattern: Format specification pattern (e.g., ``"{name}: {age:d}"``)
    :type pattern: str
    :param string: String to parse
    :type string: str
    :param extra_types: Optional mapping of custom type names (after ``:`` in the field)
        to callables, typically from :func:`with_pattern`. Uses the same compiled-parser
        cache as :func:`compile` (pattern plus per-name ``pattern`` /
        ``regex_group_count``). See the
        `Custom types guide <https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html>`_.
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: False)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :param validators: Optional map of field key to validator; see :func:`apply_validators`.
    :param pipeline: Optional :class:`ValidationPipeline` (mutually exclusive with ``validators``).
    :param validation_mode: ``\"strict\"``, ``\"collect\"``, or ``\"lenient\"`` for validation.
    :returns: ParseResult object if match found, None otherwise
    :rtype: ParseResult or None
    :raises ValueError: If pattern is invalid, or both ``validators`` and ``pipeline`` are set
    :raises ValidationError: If validation fails in strict mode
    :raises MultipleValidationErrors: If ``validation_mode='collect'`` and any validator fails

    Example::

        >>> result = parse("{name}: {age:d}", "Alice: 30")
        >>> result.named['name']
        'Alice'
        >>> result.named['age']
        30
        >>> result = parse("{}, {}", "Hello, World")
        >>> result.fixed
        ('Hello', 'World')
    """
    _validation_source_exclusive(validators=validators, pipeline=pipeline)
    r = _parse(pattern, string, extra_types, case_sensitive, evaluate_result)
    if pipeline is not None:
        return pipeline.apply(r, mode=validation_mode)
    return apply_validators(r, validators, mode=validation_mode)


def parse_with_validation(
    parser: FormatParser,
    string: str,
    pipeline: ValidationPipeline,
    *,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
    validation_mode: ValidationMode = "strict",
) -> Optional[ParseResult]:
    """Parse ``string`` with a compiled ``parser``, then run ``pipeline``.

    Equivalent to applying ``pipeline`` to the result of ``parser.parse(...)`` with
    the same ``case_sensitive``, ``extra_types``, and ``evaluate_result`` defaults as
    :func:`parse`. Use :func:`parse` or :meth:`ValidatedParser.parse` when you pass a
    ``validators`` map instead of a :class:`ValidationPipeline`.

    :param parser: Output of :func:`compile`.
    :param string: Text to parse.
    :param pipeline: Validation pipeline (required).
    :param validation_mode: Passed to :meth:`ValidationPipeline.apply`.
    :returns: Same as :meth:`FormatParser.parse` after validation, or ``None`` if parse failed.
    In ``lenient`` mode, validation failures emit :exc:`ValidationWarning` and do not raise.
    :raises ValidationError: In ``strict`` mode when validation fails.
    :raises MultipleValidationErrors: In ``collect`` mode when validation fails.
    """
    r = parser.parse(string, case_sensitive, extra_types, evaluate_result)
    return pipeline.apply(r, mode=validation_mode)


class ValidatedParser:
    """Thin wrapper around :class:`FormatParser` with optional ``validators`` / ``pipeline`` on :meth:`parse`.

    Also provides :meth:`parse_with_validation` for the compile-once + pipeline case.

    Other attributes and methods are forwarded to the inner parser (e.g. ``search``,
    ``pattern``). Use when you compile once and want the same validation ergonomics
    as :func:`parse` keyword arguments.
    """

    __slots__ = ("_parser",)

    def __init__(self, parser: FormatParser) -> None:
        object.__setattr__(self, "_parser", parser)

    def __getattr__(self, name: str) -> Any:
        return getattr(self._parser, name)

    def parse(
        self,
        string: str,
        case_sensitive: bool = False,
        extra_types: Optional[ExtraTypes] = None,
        evaluate_result: bool = True,
        *,
        validators: Optional[ValidatorMap] = None,
        pipeline: Optional[ValidationPipeline] = None,
        validation_mode: ValidationMode = "strict",
    ) -> Optional[ParseResult]:
        """Parse ``string`` with optional ``validators`` or ``pipeline`` (same rules as :func:`parse`).

        :param validation_mode: ``\"strict\"``, ``\"collect\"``, or ``\"lenient\"`` (see :func:`parse`).
        """
        _validation_source_exclusive(validators=validators, pipeline=pipeline)
        r = self._parser.parse(string, case_sensitive, extra_types, evaluate_result)
        if pipeline is not None:
            return pipeline.apply(r, mode=validation_mode)
        return apply_validators(r, validators, mode=validation_mode)

    def parse_with_validation(
        self,
        string: str,
        pipeline: ValidationPipeline,
        *,
        extra_types: Optional[ExtraTypes] = None,
        case_sensitive: bool = False,
        evaluate_result: bool = True,
        validation_mode: ValidationMode = "strict",
    ) -> Optional[ParseResult]:
        """Parse ``string`` with the inner parser, then ``pipeline`` (see :func:`parse_with_validation`)."""
        return parse_with_validation(
            self._parser,
            string,
            pipeline,
            extra_types=extra_types,
            case_sensitive=case_sensitive,
            evaluate_result=evaluate_result,
            validation_mode=validation_mode,
        )


def parse_batch(
    pattern: str,
    strings: Sequence[str],
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> List[Optional[ParseResult]]:
    """Parse many strings with the same pattern (compile once, sequential apply).

    This is intended for workloads that apply one pattern to many strings: the
    compiled regex is resolved once (same LRU cache as :func:`parse` /
    :func:`compile`) and each string is parsed in order. Non-matches appear as
    ``None`` at the corresponding index.

    ``strings`` is copied to a list of ``str`` on the Rust side (pass a
    ``list`` or ``tuple`` of strings; a bare ``str`` is treated as an iterable
    of characters, which is usually not what you want).

    :param pattern: Format specification pattern
    :param strings: Sequence of strings to parse (e.g. list or tuple)
    :param extra_types: Same as :func:`parse`
    :param case_sensitive: Same as :func:`parse`
    :param evaluate_result: Same as :func:`parse`
    :returns: List of :class:`ParseResult` or ``None`` per input string

    Example::

        >>> out = parse_batch("{:d}", ["1", "2", "x"])
        >>> out[0].fixed[0]
        1
        >>> out[2] is None
        True
    """
    return _parse_batch(
        pattern, list(strings), extra_types, case_sensitive, evaluate_result
    )


def search(
    pattern: str,
    string: str,
    pos: int = 0,
    endpos: Optional[int] = None,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = True,
    evaluate_result: bool = True,
) -> Optional[ParseResult]:
    """Search for a pattern anywhere in a string.

    Unlike parse(), which matches the entire string, search() finds the first
    occurrence of the pattern anywhere within the string.

    :param pattern: Format specification pattern
    :type pattern: str
    :param string: String to search
    :type string: str
    :param pos: Start position for search (default: 0)
    :type pos: int
    :param endpos: End position for search (default: None for end of string)
    :type endpos: int, optional
    :param extra_types: Same semantics as :func:`parse` (custom types / cache); see
        `Custom types guide <https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html>`_.
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: True)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :returns: ParseResult object if match found, None otherwise
    :rtype: ParseResult or None
    :raises ValueError: If pattern is invalid

    Example::

        >>> result = search("age: {age:d}", "Name: Alice, age: 30, City: NYC")
        >>> result.named['age']
        30
        >>> result = search("age: {age:d}", "No age here")
        >>> result is None
        True
    """
    # Validate pos parameter - handle negative values
    if pos < 0:
        pos = 0
    if pos > len(string):
        return None

    # Validate endpos parameter
    if endpos is not None:
        if endpos < 0:
            endpos = 0
        if endpos > len(string):
            endpos = len(string)
        if endpos < pos:
            return None

    return _search(
        pattern, string, pos, endpos, extra_types, case_sensitive, evaluate_result
    )


def findall(
    pattern: str,
    string: str,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> Union[Results, List[Any]]:
    """Find all matches of a pattern in a string.

    Searches for all non-overlapping occurrences of the pattern in the string.
    Returns a list-like :class:`Results` when the fast Rust path applies (no
    ``extra_types``, ``evaluate_result`` is True, and no nested dict field names).
    Otherwise returns a plain Python ``list`` of :class:`ParseResult` or
    :class:`Match` objects (same values as the original ``parse`` library).

    :param pattern: Format specification pattern
    :type pattern: str
    :param string: String to search
    :type string: str
    :param extra_types: Same semantics as :func:`parse`. When provided, the Rust fast
        path that returns :class:`Results` is disabled and a Python ``list`` is built
        instead (see returns below). See the
        `Custom types guide <https://formatparse.readthedocs.io/en/latest/user_guides/custom_types.html>`_.
    :type extra_types: dict, optional
    :param case_sensitive: Whether matching should be case sensitive (default: False)
    :type case_sensitive: bool
    :param evaluate_result: Whether to evaluate and convert result types (default: True)
    :type evaluate_result: bool
    :returns: ``Results`` (preferred) or ``list`` of matches, depending on options
    :rtype: Results | list

    Example::

        >>> results = findall("ID:{id:d}", "ID:1 ID:2 ID:3")
        >>> len(results)
        3
        >>> results[0].named['id']
        1
        >>> results[1].named['id']
        2
        >>> results[2].named['id']
        3
        >>> for result in results:
        ...     print(result.named['id'])
        1
        2
        3
    """
    return _findall(pattern, string, extra_types, case_sensitive, evaluate_result)


def findall_iter(
    pattern: str,
    string: str,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> Iterator[Any]:
    """Yield non-overlapping matches for ``pattern`` in ``string``, one at a time.

    Semantics match :func:`findall` (same ``extra_types``, ``case_sensitive``, and
    ``evaluate_result``), but each step converts at most one match. This lowers peak
    memory when you stream results instead of building a full :class:`Results` or list.

    This is a **partial** answer to `issue #13 <https://github.com/eddiethedean/formatparse/issues/13>`_:
    it does **not** implement arbitrary chunked file reads with backtracking across
    chunk boundaries. For logs, a common pattern is line-sized strings (matches must not
    span lines)::

        parser = compile("ID:{id:d}")
        with open("log.txt") as f:
            for line in f:
                for m in parser.findall_iter(line.strip()):
                    process(m.named["id"])

    :returns: Iterator of :class:`ParseResult` or :class:`Match` (same as ``findall``)
    """
    return _findall_iter(pattern, string, extra_types, case_sensitive, evaluate_result)


# Create a tzinfo-compatible wrapper for FixedTzOffset
class FixedTzOffset(tzinfo):
    """Fixed timezone offset compatible with datetime.tzinfo.

    This class provides a fixed timezone offset implementation that is compatible
    with Python's datetime.tzinfo interface. It's used internally for datetime
    parsing when timezone information is present.

    :param offset_minutes: Timezone offset in minutes from UTC
    :type offset_minutes: int
    :param name: Timezone name (e.g., "EST", "PST")
    :type name: str

    Example::

        >>> from formatparse import FixedTzOffset
        >>> from datetime import datetime
        >>> tz = FixedTzOffset(300, "EST")  # UTC-5
        >>> dt = datetime(2024, 1, 1, 12, 0, tzinfo=tz)
        >>> tz.utcoffset(dt)
        datetime.timedelta(seconds=18000)
        >>> tz.dst(dt) is None
        True
        >>> tz.tzname(dt)
        'EST'
    """

    def __init__(self, offset_minutes: int, name: str) -> None:
        """Initialize a fixed timezone offset.

        :param offset_minutes: Timezone offset in minutes from UTC
        :type offset_minutes: int
        :param name: Timezone name (e.g., "EST", "PST")
        :type name: str
        """
        self._rust_tz: _FixedTzOffset = _FixedTzOffset(offset_minutes, name)
        self._offset_minutes: int = offset_minutes
        self._name: str = name

    def __repr__(self) -> str:
        return repr(self._rust_tz)

    def __str__(self) -> str:
        return str(self._rust_tz)

    def __eq__(self, other: object) -> bool:
        if isinstance(other, FixedTzOffset):
            return self._rust_tz == other._rust_tz
        elif (
            hasattr(other, "__class__") and other.__class__.__name__ == "FixedTzOffset"
        ):
            # Handle comparison with Rust FixedTzOffset
            return self._rust_tz == other
        return False

    def __ne__(self, other: object) -> bool:
        return not self.__eq__(other)

    def utcoffset(self, dt: Optional[datetime]) -> timedelta:
        """Return the timezone offset from UTC.

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone offset as timedelta
        :rtype: datetime.timedelta
        """
        return timedelta(minutes=self._offset_minutes)

    def dst(self, dt: Optional[datetime]) -> None:
        """Return daylight saving time adjustment (always None for fixed offsets).

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Always None for fixed timezone offsets
        :rtype: None
        """
        return None

    def tzname(self, dt: Optional[datetime]) -> str:
        """Return the timezone name.

        :param dt: Datetime object (unused, kept for compatibility)
        :type dt: datetime.datetime
        :returns: Timezone name
        :rtype: str
        """
        return self._name


# Export with names matching original parse library API
Result = ParseResult
Parser = FormatParser

# Module attribute for compatibility with original parse library
# Maps strftime format codes to their regex patterns
dt_format_to_regex: Dict[str, str] = {
    "%Y": r"\d{4}",  # Year with century
    "%y": r"\d{2}",  # Year without century
    "%m": r"\d{1,2}",  # Month (1-12 or 01-12) - flexible
    "%d": r"\d{1,2}",  # Day (1-31 or 01-31) - flexible
    "%H": r"\d{1,2}",  # Hour (0-23 or 00-23) - flexible
    "%M": r"\d{1,2}",  # Minute (0-59 or 00-59) - flexible
    "%S": r"\d{1,2}",  # Second (0-59 or 00-59) - flexible
    "%f": r"\d{1,6}",  # Microseconds
    "%b": r"[A-Za-z]{3}",  # Abbreviated month name
    "%B": r"[A-Za-z]+",  # Full month name
    "%a": r"[A-Za-z]{3}",  # Abbreviated weekday
    "%A": r"[A-Za-z]+",  # Full weekday
    "%w": r"\d",  # Weekday as decimal (0=Sunday)
    "%j": r"\d{1,3}",  # Day of year (1-366, flexible padding)
    "%U": r"\d{2}",  # Week number (Sunday as first day)
    "%W": r"\d{2}",  # Week number (Monday as first day)
    "%c": r".+",  # Date and time representation (locale dependent)
    "%x": r".+",  # Date representation (locale dependent)
    "%X": r".+",  # Time representation (locale dependent)
    "%%": "%",  # Literal %
}


def with_pattern(
    pattern: str, regex_group_count: int = 0
) -> Callable[[Callable[[str], Any]], Callable[[str], Any]]:
    """Decorator to create a custom type converter with a regex pattern.

    This decorator adds a ``pattern`` attribute to the converter function,
    which is used by the parse functions when matching custom types.

    :param pattern: The regex pattern to match
    :type pattern: str
    :param regex_group_count: Number of regex groups in the pattern (for parentheses) (default: 0)
    :type regex_group_count: int
    :returns: Decorator function that adds the pattern attribute
    :rtype: Callable

    Example::

        >>> @with_pattern(r'\\d+')
        ... def parse_number(text):
        ...     return int(text)
        >>> result = parse("Answer: {:Number}", "Answer: 42", {"Number": parse_number})
        >>> result.fixed[0]
        42
        >>> type(result.fixed[0])
        <class 'int'>

        >>> @with_pattern(r'[A-Z]{2,3}')
        ... def parse_code(text):
        ...     return text.upper()
        >>> result = parse("Code: {:Code}", "Code: abc", {"Code": parse_code})
        >>> result.fixed[0]
        'ABC'
    """

    def decorator(func: Callable[[str], Any]) -> Callable[[str], Any]:
        func.pattern = pattern  # type: ignore[attr-defined]
        func.regex_group_count = regex_group_count  # type: ignore[attr-defined]
        return func

    return decorator


class ComposedType:
    """Wrap a compiled :class:`FormatParser` for use as one ``extra_types`` converter.

    Instances expose ``pattern`` and ``regex_group_count`` for the parent's regex
    builder (GitHub issue #7). Prefer constructing via :func:`composed_type`.

    Pickling a parent :class:`FormatParser` still does not restore ``extra_types``;
    rebuild composed mappings after unpickling.
    """

    __slots__ = ("_parser", "pattern", "regex_group_count")

    def __init__(self, parser: FormatParser) -> None:
        self._parser = parser
        self.pattern = parser.regex_subpattern
        self.regex_group_count = parser.regex_capturing_group_count

    def __call__(self, text: str) -> ParseResult:
        result = self._parser.parse(text)
        if result is None:
            raise ValueError(
                "Composed sub-parser did not match the captured text; the child "
                "pattern must accept the substring matched by the parent field."
            )
        return result


def composed_type(parser: FormatParser) -> ComposedType:
    """Wrap a compiled parser for embedding in another pattern's ``extra_types``.

    The parent pattern refers to a custom type name; the value is this wrapper,
    which delegates parsing of the captured substring to ``parser``.

    Example::

        >>> from formatparse import compile, composed_type
        >>> ts = compile("{year:d}-{month:02d}-{day:02d}")
        >>> log = compile(
        ...     "{ts:Timestamp} [{level}] {msg}",
        ...     extra_types={"Timestamp": composed_type(ts)},
        ... )
        >>> r = log.parse("2024-01-15 [ERROR] oops")
        >>> r.named["level"]
        'ERROR'
        >>> r.named["msg"]
        'oops'
        >>> inner = r.named["ts"]
        >>> inner.named["year"], inner.named["month"], inner.named["day"]
        (2024, 1, 15)

    :param parser: Child parser produced by :func:`compile`.
    :returns: Callable with ``pattern`` / ``regex_group_count`` set for composition.

    .. note::
        Pattern ``+``, inheritance, and flattening nested fields into the parent
        result are not implemented yet (see issue #7).
    """
    return ComposedType(parser)


class BidirectionalPattern:
    """A bidirectional pattern that can parse and format strings.

    Enables round-trip parsing: parse → modify → format back, with built-in validation.
    This class combines parsing and formatting capabilities, allowing you to parse
    a string, modify the extracted values, and format them back while maintaining
    the original format constraints.

    :param pattern: Format string pattern (e.g., ``"{name:>10}: {value:05d}"``)
    :type pattern: str
    :param extra_types: Optional dictionary of custom type converters
    :type extra_types: dict, optional

    Example::

        >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
        >>> result = formatter.parse("      John: 00042")
        >>> result.named['name']
        'John'
        >>> result.named['value']
        42
        >>> result.format()
        '      John: 00042'
        >>> result.named['value'] = 100
        >>> result.format()
        '      John: 00100'
    """

    def __init__(self, pattern: str, extra_types: Optional[ExtraTypes] = None) -> None:
        """Initialize a bidirectional pattern.

        :param pattern: Format string pattern (e.g., ``"{name:>10}: {value:05d}"``)
        :type pattern: str
        :param extra_types: Optional dictionary of custom type converters
        :type extra_types: dict, optional
        """
        self._parser: FormatParser = compile(pattern)
        self._pattern: str = pattern
        self._extra_types: Optional[ExtraTypes] = extra_types
        # Parse pattern to extract field constraints for validation
        self._field_constraints: List[FieldConstraint] = self._parse_constraints(
            pattern
        )

    def _parse_constraints(self, pattern: str) -> List[FieldConstraint]:
        """Parse pattern string to extract field constraints for validation"""
        constraints = []
        # Match field patterns: {name:format} or {name} or {}
        field_pattern = r"\{([^}]*)\}"

        for match in re.finditer(field_pattern, pattern):
            field_spec = match.group(1)
            constraint: FieldConstraint
            if not field_spec:
                # Positional field with no spec
                constraint = {
                    "name": None,
                    "type": "s",
                    "width": None,
                    "precision": None,
                }
                constraints.append(constraint)
                continue

            # Parse field name and format spec
            parts = field_spec.split(":", 1)
            name = parts[0] if parts[0] else None
            format_spec = parts[1] if len(parts) > 1 else ""

            # Parse format spec (e.g., ">10", "05d", ".2f", ">10.5s")
            constraint = {
                "name": name,
                "type": "s",
                "width": None,
                "precision": None,
            }

            # Extract type character (last letter if present)
            type_match = re.search(r"([a-zA-Z%])$", format_spec)
            if type_match:
                constraint["type"] = type_match.group(1)
                format_spec = format_spec[:-1]

            # Extract width and precision
            # Format: [fill][align][sign][width][.precision]
            # Handle formats like: "05d" (width=5), ">10" (width=10), ".5s" (precision=5), ">10.5s" (width=10, precision=5)

            # Check for precision first (after dot)
            dot_pos = format_spec.find(".")
            if dot_pos >= 0:
                # Has precision
                precision_str = format_spec[dot_pos + 1 :]
                # Remove type char from precision if present
                precision_str = re.sub(r"[a-zA-Z%]$", "", precision_str)
                if precision_str:
                    precision_match = re.search(r"(\d+)", precision_str)
                    if precision_match:
                        constraint["precision"] = int(precision_match.group(1))
                # Width is before the dot
                width_str = format_spec[:dot_pos]
            else:
                width_str = format_spec

            # Extract width from width_str (remove type char, fill, align, sign)
            # Remove type char if still present
            width_str = re.sub(r"[a-zA-Z%]$", "", width_str)
            # Remove fill, align, sign characters
            width_str = re.sub(r"[<>=^+\- ]", "", width_str)
            if width_str:
                width_match = re.search(r"(\d+)", width_str)
                if width_match:
                    constraint["width"] = int(width_match.group(1))

            constraints.append(constraint)

        return constraints

    def parse(
        self, string: str, case_sensitive: bool = False, evaluate_result: bool = True
    ) -> Optional["BidirectionalResult"]:
        """Parse a string and return BidirectionalResult.

        :param string: String to parse
        :type string: str
        :param case_sensitive: Whether matching is case-sensitive (default: False)
        :type case_sensitive: bool
        :param evaluate_result: Whether to evaluate result (convert types) (default: True)
        :type evaluate_result: bool
        :returns: BidirectionalResult if match found, None otherwise
        :rtype: BidirectionalResult or None

        Example::

            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.named['name']
            'John'
            >>> result.named['value']
            42
        """
        result = self._parser.parse(
            string,
            extra_types=self._extra_types,
            case_sensitive=case_sensitive,
            evaluate_result=evaluate_result,
        )
        if result:
            return BidirectionalResult(self, result)
        return None

    def format(self, values: Union[dict, tuple, ParseResult]) -> str:
        """Format values back into the pattern.

        Formats the provided values according to the pattern specification,
        maintaining format constraints like width, precision, and alignment.

        :param values: Dictionary (for named fields), tuple (for positional), or ParseResult
        :type values: dict, tuple, or ParseResult
        :returns: Formatted string matching the pattern
        :rtype: str

        Example::

            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> formatter.format({"name": "John", "value": 42})
            '      John: 00042'
            >>> formatter.format(("John", 42))  # Positional fields
            '      John: 00042'
        """
        # Format.format() expects args or kwargs, not a dict directly
        # For named fields, we need to unpack the dict as kwargs
        if isinstance(values, dict):
            # Use Python's format() method directly with **kwargs
            return self._pattern.format(**values)
        elif isinstance(values, tuple):
            return self._pattern.format(*values)
        elif isinstance(values, ParseResult):
            # Convert ParseResult to dict or tuple
            if values.named:
                return self._pattern.format(**dict(values.named))
            else:
                return self._pattern.format(*values.fixed)
        else:
            return self._pattern.format(values)

    def validate(
        self, values: Union[dict, tuple, ParseResult]
    ) -> Tuple[bool, List[str]]:
        """
        Validate values against format constraints.

        Args:
            values: Dict (for named fields), tuple (for positional), or ParseResult

        Returns:
            Tuple of (is_valid, list_of_errors)
        """
        errors = []

        # Convert values to dict/list format
        if isinstance(values, ParseResult):
            named_values = dict(values.named) if values.named else {}
            fixed_values = list(values.fixed) if values.fixed else []
        elif isinstance(values, dict):
            named_values = values
            fixed_values = []
        elif isinstance(values, tuple):
            named_values = {}
            fixed_values = list(values)
        else:
            return False, ["Invalid values type: expected dict, tuple, or ParseResult"]

        # Validate each field
        for i, constraint in enumerate(self._field_constraints):
            field_name = constraint["name"]
            field_type = constraint["type"]
            width = constraint["width"]
            precision = constraint["precision"]

            # Get value
            if field_name:
                if field_name not in named_values:
                    continue  # Field not present, skip validation
                value = named_values[field_name]
            else:
                if i >= len(fixed_values):
                    continue  # Positional field not present
                value = fixed_values[i]

            # Type validation
            if field_type == "d" and not isinstance(value, int):
                errors.append(
                    f"Field '{field_name or i}': expected int, got {type(value).__name__}"
                )
            elif field_type == "f" and not isinstance(value, (int, float)):
                errors.append(
                    f"Field '{field_name or i}': expected float, got {type(value).__name__}"
                )

            # Width/precision validation for strings
            if isinstance(value, str):
                if precision is not None and len(value) > precision:
                    errors.append(
                        f"Field '{field_name or i}': string length {len(value)} exceeds precision {precision}"
                    )
                if width is not None and len(value) > width:
                    errors.append(
                        f"Field '{field_name or i}': string length {len(value)} exceeds width {width}"
                    )

            # Width validation for integers (zero-padded)
            if isinstance(value, int) and width is not None:
                # Check if value fits in width with zero-padding
                # Need to account for sign if negative
                value_str = str(abs(value))
                sign_len = 1 if value < 0 else 0
                if len(value_str) + sign_len > width:
                    errors.append(
                        f"Field '{field_name or i}': integer {value} exceeds width {width} (with zero-padding)"
                    )

        return len(errors) == 0, errors


class BidirectionalResult:
    """Result from BidirectionalPattern.parse() that allows modification and formatting.

    Stores parsed values in a mutable format and provides methods to format back
    and validate against the original pattern constraints. Unlike ParseResult, this
    class allows you to modify the extracted values and format them back while
    maintaining the original format constraints.

    Example::

        >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
        >>> result = formatter.parse("      John: 00042")
        >>> result.named['value'] = 100
        >>> result.format()
        '      John: 00100'
        >>> result.validate()
        (True, [])
    """

    def __init__(self, pattern: BidirectionalPattern, result: ParseResult) -> None:
        """Initialize a bidirectional result.

        :param pattern: The BidirectionalPattern that created this result
        :type pattern: BidirectionalPattern
        :param result: The ParseResult from parsing
        :type result: ParseResult
        """
        self._pattern: BidirectionalPattern = pattern
        self._result: ParseResult = result
        # Store values in mutable dict/list
        self._values: Dict[str, Union[Dict[str, Any], List[Any]]] = {
            "named": dict(result.named) if result.named else {},
            "fixed": list(result.fixed) if result.fixed else [],
        }

    @property
    def named(self) -> Dict[str, Any]:
        """Mutable named fields dictionary.

        :returns: Dictionary of named fields (can be modified)
        :rtype: Dict[str, Any]

        Example::

            >>> formatter = BidirectionalPattern("{name}: {age:d}")
            >>> result = formatter.parse("Alice: 30")
            >>> result.named['age'] = 31
            >>> result.format()
            'Alice: 31'
        """
        return self._values["named"]  # type: ignore[return-value]

    @property
    def fixed(self) -> List[Any]:
        """Mutable fixed (positional) fields list.

        :returns: List of positional fields (can be modified)
        :rtype: List[Any]

        Example::

            >>> formatter = BidirectionalPattern("{}, {}")
            >>> result = formatter.parse("Hello, World")
            >>> result.fixed[1] = "Python"
            >>> result.format()
            'Hello, Python'
        """
        return self._values["fixed"]  # type: ignore[return-value]

    def format(self) -> str:
        """Format values back using the pattern.

        Formats the current (potentially modified) values according to the
        original pattern specification.

        :returns: Formatted string matching the original pattern
        :rtype: str

        Example::

            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.named['value'] = 100
            >>> result.format()
            '      John: 00100'
        """
        if self._values["named"]:
            return self._pattern.format(self._values["named"])
        else:
            return self._pattern.format(tuple(self._values["fixed"]))

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current values against format constraints.

        Checks if the current (potentially modified) values conform to the
        pattern's constraints (type, width, precision).

        :returns: Tuple of (is_valid, list_of_errors)
        :rtype: Tuple[bool, List[str]]

        Example::

            >>> formatter = BidirectionalPattern("{name:>10}: {value:05d}")
            >>> result = formatter.parse("      John: 00042")
            >>> result.validate()
            (True, [])
            >>> result.named['value'] = "not a number"
            >>> is_valid, errors = result.validate()
            >>> is_valid
            False
            >>> len(errors) > 0
            True
        """
        # Pass the actual values dict/list, not the wrapper structure
        if self._values["named"]:
            return self._pattern.validate(self._values["named"])
        else:
            return self._pattern.validate(tuple(self._values["fixed"]))

    def __repr__(self) -> str:
        """String representation"""
        if self._values["named"]:
            return f"<BidirectionalResult {self._values['named']}>"
        else:
            return f"<BidirectionalResult {self._values['fixed']}>"


__all__ = [
    "__version__",
    "parse",
    "parse_with_validation",
    "parse_batch",
    "search",
    "findall",
    "findall_iter",
    "FindallIter",
    "with_pattern",
    "composed_type",
    "ComposedType",
    "ValidationError",
    "ValidationWarning",
    "MultipleValidationErrors",
    "ValidationMode",
    "ValidatorMap",
    "apply_validators",
    "ValidationPipeline",
    "in_range",
    "non_empty_str",
    "is_valid_email",
    "is_valid_url",
    "validator",
    "ValidatedParser",
]
