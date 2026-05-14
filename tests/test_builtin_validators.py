"""Built-in post-parse validators (issue #11): email and URL."""

import pytest

from formatparse import (
    ValidationError,
    ValidationPipeline,
    ValidationWarning,
    is_valid_email,
    is_valid_url,
    parse,
)


def test_is_valid_email_accepts_common_forms():
    is_valid_email("user@example.com")
    is_valid_email("  u.name+tag@host.co.uk  ")
    is_valid_email("a@b.co")


def test_is_valid_email_rejects():
    with pytest.raises(ValidationError, match="non-empty string"):
        is_valid_email("")
    with pytest.raises(ValidationError, match="non-empty string"):
        is_valid_email("   ")
    with pytest.raises(ValidationError, match="invalid email"):
        is_valid_email("not-an-email")
    with pytest.raises(ValidationError, match="invalid email"):
        is_valid_email("@host.com")
    with pytest.raises(ValidationError, match="invalid email"):
        is_valid_email("user@")
    with pytest.raises(ValidationError, match="non-empty string"):
        is_valid_email(None)  # type: ignore[arg-type]


def test_is_valid_url_accepts_http_https():
    is_valid_url("https://example.com/path?q=1")
    is_valid_url("http://127.0.0.1:8080/")
    is_valid_url("http://localhost/foo")


def test_is_valid_url_rejects():
    with pytest.raises(ValidationError, match="non-empty string"):
        is_valid_url("")
    with pytest.raises(ValidationError, match="http or https"):
        is_valid_url("ftp://example.com/")
    with pytest.raises(ValidationError, match="http or https"):
        is_valid_url("mailto:a@b.com")
    with pytest.raises(ValidationError, match="missing host"):
        is_valid_url("https://")
    with pytest.raises(ValidationError, match="missing host"):
        is_valid_url("http:///path")
    with pytest.raises(ValidationError, match="non-empty string"):
        is_valid_url(None)  # type: ignore[arg-type]


def test_parse_with_is_valid_url():
    r = parse("{u}", "https://x.example/", validators={"u": is_valid_url})
    assert r is not None
    assert r.named["u"] == "https://x.example/"


def test_parse_strict_is_valid_email_raises():
    with pytest.raises(ValidationError, match="invalid email"):
        parse("{e}", "bad", validators={"e": is_valid_email})


def test_parse_lenient_is_valid_email_warns():
    with pytest.warns(ValidationWarning, match="invalid email"):
        r = parse(
            "{e}", "bad", validators={"e": is_valid_email}, validation_mode="lenient"
        )
    assert r is not None
    assert r.named["e"] == "bad"


def test_validation_pipeline_is_valid_email():
    pl = ValidationPipeline().add_validator("e", is_valid_email)
    r = parse("{e}", "hi@there.org")
    assert r is not None
    pl.apply(r)
