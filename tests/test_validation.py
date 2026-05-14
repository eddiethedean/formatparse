"""Post-parse validators (GitHub issue #10)."""

import pytest

from formatparse import (
    MultipleValidationErrors,
    ValidationError,
    ValidatedParser,
    apply_validators,
    compile,
    parse,
    validator,
)


def test_apply_validators_named_strict_passes():
    r = parse("{age:d}", "21")
    assert r is not None
    apply_validators(
        r,
        {
            "age": lambda v: (
                None if v >= 18 else (_ for _ in ()).throw(ValidationError("minor"))
            )
        },
    )
    assert r.named["age"] == 21


def test_apply_validators_named_strict_raises():
    r = parse("{age:d}", "15")
    assert r is not None
    with pytest.raises(ValidationError, match="minor"):
        apply_validators(
            r,
            {
                "age": lambda v: (
                    None if v >= 18 else (_ for _ in ()).throw(ValidationError("minor"))
                )
            },
        )
    assert r.named["age"] == 15


def test_apply_validators_fixed_index():
    r = parse("{:d}-{}", "10-x")
    assert r is not None
    apply_validators(
        r,
        {
            0: lambda v: (
                None
                if v > 0
                else (_ for _ in ()).throw(ValidationError("need positive"))
            )
        },
    )
    assert r.fixed == (10, "x")


def test_parse_validators_keyword_only():
    r = parse(
        "{a:d}",
        "3",
        validators={
            "a": lambda v: (
                None if v % 2 == 1 else (_ for _ in ()).throw(ValidationError("even"))
            )
        },
    )
    assert r is not None
    assert r.named["a"] == 3


def test_parse_no_match_skips_validators():
    assert (
        parse(
            "{a:d}",
            "not a number",
            validators={
                "a": lambda _: (_ for _ in ()).throw(AssertionError("should not run"))
            },
        )
        is None
    )


def test_collect_mode_multiple_errors():
    r = parse("{:d}-{:d}", "1-2")
    assert r is not None
    with pytest.raises(MultipleValidationErrors) as exc_info:
        apply_validators(
            r,
            {
                0: lambda v: (_ for _ in ()).throw(ValidationError("bad a")),
                1: lambda v: (_ for _ in ()).throw(ValidationError("bad b")),
            },
            mode="collect",
        )
    err = exc_info.value
    assert len(err.errors) == 2
    assert {e.field for e in err.errors} == {0, 1}


def test_unknown_named_field_key():
    r = parse("{x:d}", "1")
    assert r is not None
    with pytest.raises(ValidationError, match="no named field"):
        apply_validators(r, {"y": lambda v: None})


def test_wrapped_non_validation_error():
    r = parse("{x:d}", "1")
    assert r is not None

    def boom(_v):
        raise RuntimeError("inner")

    with pytest.raises(ValidationError, match="validator failed"):
        apply_validators(r, {"x": boom})
    try:
        apply_validators(r, {"x": boom})
    except ValidationError as e:
        assert isinstance(e.__cause__, RuntimeError)


def test_validator_decorator_sets_attr():
    @validator
    def check(x: int) -> None:
        assert x > 0

    assert getattr(check, "_formatparse_validator", False) is True


def test_validated_parser_parse_and_forward():
    inner = compile("{n}")
    vp = ValidatedParser(inner)
    assert vp.pattern == "{n}"
    r = vp.parse(
        "hi",
        validators={
            "n": lambda s: (
                None if s else (_ for _ in ()).throw(ValidationError("empty"))
            )
        },
    )
    assert r is not None
    assert r.named["n"] == "hi"
