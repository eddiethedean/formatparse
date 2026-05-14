"""ValidationPipeline (per-field validators, whole-result hooks) and builtins (#11)."""

import pytest

from formatparse import (
    MultipleValidationErrors,
    ValidationError,
    ValidationPipeline,
    ValidatedParser,
    apply_validators,
    compile,
    in_range,
    non_empty_str,
    parse,
)


def test_hooks_run_after_field_validators():
    order: list[str] = []
    p = (
        ValidationPipeline()
        .add_validator("a", lambda v: order.append("field"))
        .add_hook(lambda r: order.append("hook"))
    )
    r = parse("{a:d}", "1")
    assert r is not None
    p.apply(r)
    assert order == ["field", "hook"]


def test_apply_none_skips_validators_and_hooks():
    called: list[int] = []
    p = ValidationPipeline().add_hook(lambda r: called.append(1))
    assert p.apply(None) is None
    assert called == []


def test_hook_strict_raises():
    p = ValidationPipeline().add_hook(
        lambda r: (_ for _ in ()).throw(ValidationError("cross-field"))
    )
    r = parse("{a:d}", "1")
    assert r is not None
    with pytest.raises(ValidationError, match="cross-field") as exc:
        p.apply(r)
    assert exc.value.field is None


def test_hooks_collect_two_failures():
    p = (
        ValidationPipeline()
        .add_hook(lambda r: (_ for _ in ()).throw(ValidationError("first")))
        .add_hook(lambda r: (_ for _ in ()).throw(ValidationError("second")))
    )
    r = parse("{x:d}", "0")
    assert r is not None
    with pytest.raises(MultipleValidationErrors) as exc:
        p.apply(r, mode="collect")
    msgs = [e.args[0] for e in exc.value.errors]
    assert msgs == ["first", "second"]


def test_cross_field_hook_end_after_start():
    def dates_ok(res):
        if res.named["end"] < res.named["start"]:
            raise ValidationError("end before start")

    p = (
        ValidationPipeline()
        .add_validator("start", lambda _: None)
        .add_validator("end", lambda _: None)
        .add_hook(dates_ok)
    )
    ok = parse("{start:d}-{end:d}", "2-5", pipeline=p)
    assert ok is not None
    with pytest.raises(ValidationError, match="end before start"):
        parse("{start:d}-{end:d}", "5-2", pipeline=p)


def test_pipeline_apply_delegates_to_collect():
    p = ValidationPipeline()
    p.add_validator("a", lambda v: (_ for _ in ()).throw(ValidationError("bad a")))
    p.add_validator("b", lambda v: (_ for _ in ()).throw(ValidationError("bad b")))
    r = parse("{a:d}-{b:d}", "1-2")
    assert r is not None
    with pytest.raises(MultipleValidationErrors) as exc:
        p.apply(r, mode="collect")
    assert {e.field for e in exc.value.errors} == {"a", "b"}


def test_pipeline_last_validator_wins_same_field():
    pipe = (
        ValidationPipeline()
        .add_validator("x", lambda _: None)
        .add_validator("x", lambda _: (_ for _ in ()).throw(ValidationError("second")))
    )
    r = parse("{x:d}", "1")
    assert r is not None
    with pytest.raises(ValidationError, match="second"):
        pipe.apply(r)


def test_parse_with_pipeline_keyword():
    pl = ValidationPipeline().add_validator("x", in_range(0, 5))
    r = parse("{x:d}", "3", pipeline=pl)
    assert r is not None
    assert r.named["x"] == 3


    pl = ValidationPipeline().add_validator("a", lambda _: None)
    with pytest.raises(ValueError, match="only one of"):
        parse("{a:d}", "1", validators={"a": lambda _: None}, pipeline=pl)


def test_validated_parser_with_pipeline():
    pl = (
        ValidationPipeline()
        .add_validator("age", in_range(0, 150))
        .add_validator("name", non_empty_str)
    )
    vp = ValidatedParser(compile("{name} {age:d}"))
    r = vp.parse("Ann 40", pipeline=pl)
    assert r is not None
    assert r.named["name"] == "Ann"
    assert r.named["age"] == 40


def test_validated_parser_pipeline_conflict():
    pl = ValidationPipeline().add_validator("a", lambda _: None)
    vp = ValidatedParser(compile("{a:d}"))
    with pytest.raises(ValueError, match="only one of"):
        vp.parse("1", validators={"a": lambda _: None}, pipeline=pl)


def test_in_range_pass_and_fail():
    r = parse("{n:d}", "10", validators={"n": in_range(1, 20)})
    assert r is not None
    assert r.named["n"] == 10
    r2 = parse("{n:d}", "99")
    assert r2 is not None
    with pytest.raises(ValidationError, match="expected value <="):
        apply_validators(r2, {"n": in_range(1, 20)})


def test_non_empty_str():
    r = parse("{s}", "hi", validators={"s": non_empty_str})
    assert r is not None
    with pytest.raises(ValidationError, match="non-empty"):
        parse("{s}", "   ", validators={"s": non_empty_str})
