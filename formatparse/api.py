"""Core parse API wrappers."""

from __future__ import annotations

from typing import Any, Iterator, List, Optional, Sequence, Union

from ._native import (
    FormatParser,
    ParseResult,
    Results,
    _compile,
    _findall,
    _findall_iter,
    _parse,
    _parse_batch,
    _search,
)
from .exceptions import RepeatedNameError
from .types import ExtraTypes, ValidationMode, ValidatorMap
from .validation import ValidationPipeline, post_parse_validate


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
    :raises PatternParseMismatch: For some malformed patterns (missing ``}`` after a field);
        subclass of :exc:`ValueError`. :func:`parse` returns ``None`` for the same pattern.
    :raises ValueError: For other invalid patterns or internal errors

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
    :raises ValueError: If the pattern is invalid in a way that still raises from the native
        compiler (for example some unclosed nested format specs), or if both ``validators``
        and ``pipeline`` are set. For a narrow class of malformed patterns (missing ``}`` after
        a field), this function returns ``None`` while :func:`compile` raises
        :exc:`PatternParseMismatch`, which is a :exc:`ValueError` subclass (same split as the
        original ``parse`` library).
    :raises NotImplementedError: For unsupported pattern features (for example quoted dict keys).
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
    r = _parse(pattern, string, extra_types, case_sensitive, evaluate_result)
    return post_parse_validate(
        r,
        validators=validators,
        pipeline=pipeline,
        validation_mode=validation_mode,
    )


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
        r = self._parser.parse(string, case_sensitive, extra_types, evaluate_result)
        return post_parse_validate(
            r,
            validators=validators,
            pipeline=pipeline,
            validation_mode=validation_mode,
        )

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
    :raises ValueError: Same pattern-compile rules as :func:`parse`; if the pattern is in the
        narrow class where :func:`parse` returns ``None``, this function returns a list of
        ``None`` with one entry per input string.

    Example::

        >>> out = parse_batch("{:d}", ["1", "2", "x"])
        >>> out[0].fixed[0]
        1
        >>> out[2] is None
        True
    """
    if isinstance(strings, (str, bytes)):
        raise TypeError("expected a sequence of strings, not a single str")
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
    max_matches: Optional[int] = None,
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
    :param max_matches: Stop after this many matches (default: no limit). Useful for
        untrusted input; see the Security guide in the project docs (``docs/security.rst``).
    :type max_matches: int, optional
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
    return _findall(
        pattern,
        string,
        extra_types,
        case_sensitive,
        evaluate_result,
        max_matches,
    )


def findall_iter(
    pattern: str,
    string: str,
    extra_types: Optional[ExtraTypes] = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
    max_matches: Optional[int] = None,
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

    :param max_matches: Same as :func:`findall` (default: no limit).
    :type max_matches: int, optional

    :returns: Iterator of :class:`ParseResult` or :class:`Match` (same as ``findall``)
    """
    return _findall_iter(
        pattern,
        string,
        extra_types,
        case_sensitive,
        evaluate_result,
        max_matches,
    )
