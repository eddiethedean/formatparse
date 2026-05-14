"""parse_with_validation helper (GitHub issue #11)."""

import pytest

from formatparse import (
    ValidationError,
    ValidationPipeline,
    ValidationWarning,
    ValidatedParser,
    compile,
    in_range,
    parse_with_validation,
)


def test_parse_with_validation_success():
    parser = compile("{name} {age:d}")
    pl = ValidationPipeline().add_validator("age", in_range(0, 150))
    r = parse_with_validation(parser, "Ann 40", pl)
    assert r is not None
    assert r.named["name"] == "Ann"
    assert r.named["age"] == 40


def test_parse_with_validation_strict_raises():
    parser = compile("{n:d}")
    pl = ValidationPipeline().add_validator("n", in_range(0, 5))
    with pytest.raises(ValidationError):
        parse_with_validation(parser, "9", pl)


def test_parse_with_validation_lenient_warns():
    parser = compile("{n:d}")
    pl = ValidationPipeline().add_validator("n", in_range(0, 5))
    with pytest.warns(ValidationWarning):
        r = parse_with_validation(parser, "9", pl, validation_mode="lenient")
    assert r is not None
    assert r.named["n"] == 9


def test_parse_with_validation_none_short_circuit():
    parser = compile("{n:d}")
    pl = ValidationPipeline().add_validator("n", lambda _: (_ for _ in ()).throw(AssertionError("no")))
    assert parse_with_validation(parser, "not-a-number", pl) is None


def test_validated_parser_parse_with_validation():
    vp = ValidatedParser(compile("{x:d}"))
    pl = ValidationPipeline().add_validator("x", in_range(1, 10))
    r = vp.parse_with_validation("5", pl)
    assert r is not None
    assert r.named["x"] == 5
