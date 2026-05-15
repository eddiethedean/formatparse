"""Rust extension imports (internal)."""

from __future__ import annotations

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
    PatternParseMismatch,
    Results,
)
