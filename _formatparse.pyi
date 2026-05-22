"""Type stubs for the native ``_formatparse`` extension (PyO3)."""

from collections.abc import Hashable, Iterator, Mapping
from typing import Any

class PatternParseMismatch(ValueError): ...

class ParseResult:
    named: dict[str, Any]
    span: tuple[int, int]
    field_spans: dict[str, tuple[int, int]]
    @property
    def fixed(self) -> tuple[Any, ...]: ...
    @property
    def start(self) -> int: ...
    @property
    def end(self) -> int: ...
    @property
    def spans(self) -> dict[Hashable, tuple[int, int]]: ...
    def __getitem__(self, key: int | str | slice) -> Any: ...
    def __contains__(self, key: object) -> bool: ...

class Format:
    def format(self, *args: Any) -> str: ...

class FormatParser:
    def __init__(
        self, pattern: str | None = None, extra_types: Mapping[str, Any] | None = None
    ) -> None: ...
    def parse(
        self,
        string: str,
        case_sensitive: bool = False,
        extra_types: Mapping[str, Any] | None = None,
        evaluate_result: bool = True,
    ) -> ParseResult | None: ...
    def search(
        self,
        string: str,
        case_sensitive: bool = True,
        extra_types: Mapping[str, Any] | None = None,
        evaluate_result: bool = True,
    ) -> ParseResult | None: ...
    def findall_iter(
        self,
        string: str,
        case_sensitive: bool = False,
        extra_types: Mapping[str, Any] | None = None,
        evaluate_result: bool = True,
    ) -> FindallIter: ...
    @property
    def named_fields(self) -> list[str]: ...
    @property
    def regex_subpattern(self) -> str: ...
    @property
    def regex_capturing_group_count(self) -> int: ...
    @property
    def _expression(self) -> str: ...
    @property
    def format(self) -> Format: ...
    def __getstate__(self) -> dict[str, Any]: ...
    def __setstate__(self, state: dict[str, Any]) -> None: ...

class FindallIter:
    def __iter__(self) -> Iterator[ParseResult]: ...
    def __next__(self) -> ParseResult | None: ...

class Results:
    def __len__(self) -> int: ...
    def __getitem__(self, key: int | slice) -> Any: ...
    def __iter__(self) -> Iterator[Any]: ...

class Match:
    pattern: str
    span: tuple[int, int]
    def evaluate_result(
        self, *, extra_types: Mapping[str, Any] | None = None
    ) -> ParseResult: ...

class FixedTzOffset:
    def __init__(self, offset_minutes: int, name: str) -> None: ...
    def utcoffset(self, dt: Any = None) -> Any: ...
    def dst(self, dt: Any = None) -> Any: ...
    def tzname(self, dt: Any = None) -> str: ...

def parse(
    pattern: str,
    string: str,
    extra_types: Mapping[str, Any] | None = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> ParseResult | None: ...
def parse_batch(
    pattern: str,
    strings: list[str],
    extra_types: Mapping[str, Any] | None = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> list[ParseResult | None]: ...
def search(
    pattern: str,
    string: str,
    pos: int = 0,
    endpos: int | None = None,
    extra_types: Mapping[str, Any] | None = None,
    case_sensitive: bool = True,
    evaluate_result: bool = True,
) -> ParseResult | None: ...
def findall(
    pattern: str,
    string: str,
    extra_types: Mapping[str, Any] | None = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> Results: ...
def findall_iter(
    pattern: str,
    string: str,
    extra_types: Mapping[str, Any] | None = None,
    case_sensitive: bool = False,
    evaluate_result: bool = True,
) -> FindallIter: ...
def compile(
    pattern: str, extra_types: Mapping[str, Any] | None = None
) -> FormatParser: ...
def extract_format(
    format_string: str, _match_dict: dict[str, Any] | None = None
) -> dict[str, Any]: ...
