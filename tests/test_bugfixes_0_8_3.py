"""Regression tests for 0.8.3 bug fixes."""

import pytest

import formatparse as fp
from formatparse import BidirectionalPattern, ValidationPipeline, compile, parse, search
from formatparse.custom import with_pattern
from formatparse.exceptions import RepeatedNameError, ValidationError
from formatparse.validation import apply_validators, in_range, non_empty_str


# --- Bug 1: findall alignment validation parity ---


def test_findall_alignment_precision_parity_with_parse():
    pattern = "{s:>4.4}"
    assert parse(pattern, " aa ") is None
    assert len(fp.findall(pattern, " aa ")) == 0


# --- Bug 2/8/9: bidirectional mixed format and validate ---


def test_bidirectional_mixed_format_round_trip():
    formatter = BidirectionalPattern("{name}, {}, {age:d}")
    result = formatter.parse("Alice, City, 30")
    assert result is not None
    assert result.format() == "Alice, City, 30"


def test_bidirectional_mixed_validate_positional_index():
    formatter = BidirectionalPattern("{name}, {:10s}, {age:d}")
    result = formatter.parse("Alice,          City, 30")
    assert result is not None
    result.fixed[0] = "x" * 50
    valid, errors = result.validate()
    assert not valid
    assert any("exceeds width" in e for e in errors)


# --- Bug 3/7: validation mode guards ---


def test_validation_pipeline_apply_invalid_mode():
    pipeline = ValidationPipeline().add_validator("n", non_empty_str)
    result = parse("{n}", "ok")
    with pytest.raises(ValueError, match="invalid validation_mode"):
        pipeline.apply(result, mode="typo")  # type: ignore[arg-type]


def test_apply_validators_invalid_mode():
    result = parse("{n}", "ok")
    with pytest.raises(ValueError, match="invalid validation_mode"):
        apply_validators(result, {"n": non_empty_str}, mode="strictt")  # type: ignore[arg-type]


# --- Bug 4: repeated custom / strftime types ---


def test_repeated_custom_type_raises():
    @with_pattern(r"\d+")
    def parse_number(text):
        return int(text)

    @with_pattern(r"\d+")
    def parse_other(text):
        return int(text)

    with pytest.raises(RepeatedNameError):
        compile(
            "{a:Foo}{a:Bar}",
            extra_types={"Foo": parse_number, "Bar": parse_other},
        )


def test_repeated_strftime_format_merges():
    """Repeated strftime fragments with the same name merge (issue #4)."""
    result = parse("{t:%Y}-{t:%m}", "2024-03")
    assert result is not None
    assert result.named["t"].year == 2024
    assert result.named["t"].month == 3


# --- Bug 6: field name length ---


def test_field_name_too_long_raises():
    with pytest.raises(Exception):
        compile("{" + "a" * 201 + "}")


# --- Bug 10: integer newline padding ---


def test_integer_rejects_leading_newline():
    assert parse("{n:d}", "\n42") is None
    assert parse("{n:d}", "  42") is not None


# --- Bug 11: custom type underscore ---


def test_custom_type_name_with_underscore():
    @with_pattern(r"[A-Z]+")
    def parse_upper(text):
        return text

    result = parse("{v:My_Type}", "HELLO", extra_types={"My_Type": parse_upper})
    assert result is not None
    assert result.named["v"] == "HELLO"


# --- Bug 12: width/precision cap ---


def test_width_precision_overflow_raises():
    with pytest.raises(Exception):
        compile("{x:99999d}")


# --- Bug 13: search match offset ---


def test_search_match_offset_when_not_evaluated():
    text = "prefix " + "x" * 100 + " value=42 suffix"
    m = search("{v:d}", text, pos=50, evaluate_result=False)
    assert m is not None
    assert m.span[0] >= 50


# --- Bug 14: __eq__ errors propagate ---


def test_match_eq_error_propagates():
    class BadEq:
        def __eq__(self, other):
            raise RuntimeError("eq failed")

    @with_pattern(r"(.)", regex_group_count=1)
    def parse_bad(text):
        return BadEq()

    with pytest.raises(RuntimeError, match="eq failed"):
        parse("{a:Bad}{a:Bad}", "xy", extra_types={"Bad": parse_bad})


# --- Bug 15: evaluate_result validation ---


def test_search_evaluate_result_alignment_check():
    m = search("{s:>4.4}", " aa ", evaluate_result=False)
    assert m is not None
    with pytest.raises(ValueError, match="alignment/precision"):
        m.evaluate_result()


# --- Bug 17: bidirectional format injection ---


def test_bidirectional_rejects_attribute_access_pattern():
    with pytest.raises(ValueError, match="unsafe format field"):
        BidirectionalPattern("{0.__class__}")


# --- Bug 18: MAX_FIELDS in core ---


def test_max_fields_raises():
    pattern = "".join(f"{{{i}}}" for i in range(101))
    with pytest.raises(Exception):
        compile(pattern)


# --- Bug 19: parse_batch str footgun ---


def test_parse_batch_rejects_single_str():
    with pytest.raises(TypeError, match="sequence of strings"):
        fp.parse_batch("{:d}", "12345")


# --- Bug 20: in_range rejects bool ---


def test_in_range_rejects_bool():
    with pytest.raises(ValidationError, match="bool"):
        in_range(0, 10)(True)


# --- Bug 21: validator key types ---


def test_validator_keys_must_be_str_or_int():
    result = parse("{n}", "x")
    with pytest.raises(TypeError, match="str or int"):
        apply_validators(result, {("tuple",): non_empty_str})
