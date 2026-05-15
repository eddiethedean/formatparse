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
``=`` alignment are not supported with ``:ml``. Inside the matched text, a backslash
immediately before end-of-line continues the line (same rules as long **patterns** in
issue #68); doubled backslashes keep a literal newline (see `GitHub issue #80
<https://github.com/eddiethedean/formatparse/issues/80>`_).

**Indented blocks:** use ``{name:blk}`` when content is written as an indented block
under a header: matching uses the same rules as ``:ml`` (non-greedy up to the next
literal or field), then the captured text is **dedented** by removing the largest
common prefix of spaces and tabs from each line (blank lines do not contribute to
that margin; tabs count as single characters). See `GitHub issue #69
<https://github.com/eddiethedean/formatparse/issues/69>`_. The same backslash line
continuation rules as for ``:ml`` apply to matched input (issue #80). If a pattern literal ends
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

**Integer decimal ``d``:** optional leading spaces and tabs are allowed before the digit
run so values such as ``"    0"`` match ``{a:d}`` when they mirror padded numeric output
(see `GitHub issue #81 <https://github.com/eddiethedean/formatparse/issues/81>`_). When
both **width** and **.precision** are present on integer or radix fields (e.g. ``{:2.2d}``,
``{:2.2x}``), the digit run is bounded: at least ``width`` digits and at most ``precision``
(`GitHub issue #82 <https://github.com/eddiethedean/formatparse/issues/82>`_; `parse#107
<https://github.com/r1chardj0n3s/parse/issues/107>`_).

**Composition (sub-parsers):** use :func:`composed_type` to wrap a compiled
:class:`FormatParser` and pass it in ``extra_types`` so one field is parsed by the
child parser and returns a nested :class:`ParseResult`. The parent pickle story is
unchanged (only the parent pattern string is restored); re-supply ``extra_types``
after unpickling, including any composed child parsers (see `GitHub issue #7
<https://github.com/eddiethedean/formatparse/issues/7>`_).

**Nested brace patterns in a field spec:** when the substring after ``:`` is a
balanced nested field pattern (for example ``{outer:{inner:d}}``), the inner
pattern is compiled and matched as part of the outer capture, then parsed
again; nested groups appear as :class:`ParseResult` objects under
``ParseResult.named`` (see `GitHub issue #12
<https://github.com/eddiethedean/formatparse/issues/12>`_ and upstream
`parse#206 <https://github.com/r1chardj0n3s/parse/issues/206>`_). Maximum
nesting depth is 10.

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

from ._native import (
    FindallIter,
    FormatParser,
    ParseResult,
    PatternParseMismatch,
    Results,
)
from ._version import __version__
from .api import (
    ValidatedParser,
    compile,
    findall,
    findall_iter,
    parse,
    parse_batch,
    parse_with_validation,
    search,
)
from .bidirectional import BidirectionalPattern, BidirectionalResult
from .compat import Parser, Result, dt_format_to_regex
from .custom import ComposedType, composed_type, with_pattern
from .exceptions import (
    MultipleValidationErrors,
    RepeatedNameError,
    ValidationError,
    ValidationWarning,
)
from .tz import FixedTzOffset
from .types import (
    ConverterProtocol,
    ExtraTypes,
    FieldConstraint,
    ValidationMode,
    ValidatorMap,
)
from .validation import (
    ValidationPipeline,
    apply_validators,
    in_range,
    is_valid_email,
    is_valid_url,
    non_empty_str,
    validator,
)

__all__ = [
    "__version__",
    "PatternParseMismatch",
    "ParseResult",
    "FormatParser",
    "FindallIter",
    "Results",
    "compile",
    "parse",
    "parse_with_validation",
    "parse_batch",
    "search",
    "findall",
    "findall_iter",
    "RepeatedNameError",
    "with_pattern",
    "composed_type",
    "ComposedType",
    "ValidationError",
    "ValidationWarning",
    "MultipleValidationErrors",
    "ValidationMode",
    "ValidatorMap",
    "ConverterProtocol",
    "ExtraTypes",
    "FieldConstraint",
    "apply_validators",
    "ValidationPipeline",
    "in_range",
    "non_empty_str",
    "is_valid_email",
    "is_valid_url",
    "validator",
    "ValidatedParser",
    "FixedTzOffset",
    "BidirectionalPattern",
    "BidirectionalResult",
    "Result",
    "Parser",
    "dt_format_to_regex",
]
