use crate::datetime;
use crate::error;
use crate::types::builtin_convert::{convert_builtin_scalar, ConvertOutcome};
use formatparse_core::{FieldSpec, FieldType};
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

/// Validate alignment+precision constraints for string fields
/// Returns false if validation fails (should reject the match)
///
/// This validates the constraints described in issue #3 (parse#218):
/// - Fill characters should only be in correct positions (left for right-align, right for left-align)
/// - Total width (including fill chars) should not exceed specified width when width is specified
/// - Content length (after removing fill chars) should not exceed precision
pub fn validate_alignment_precision(spec: &FieldSpec, value: &str) -> bool {
    if !matches!(
        &spec.field_type,
        FieldType::String | FieldType::Multiline | FieldType::IndentBlock
    ) {
        return true;
    }
    if let (Some(prec), Some(align)) = (spec.precision, spec.alignment) {
        // When width == precision == captured length and the field is zero-filled
        // right-aligned, leading/trailing '0' cannot be distinguished from content
        // (GitHub issue #40; parse parity).
        if align == '>'
            && spec.fill == Some('0')
            && spec.width == Some(prec)
            && Some(value.len()) == spec.width
        {
            return true;
        }

        let fill_ch = spec.fill.unwrap_or(' ');
        let has_leading_fill = value.starts_with(fill_ch);
        let has_trailing_fill = value.ends_with(fill_ch);

        // Count leading and trailing fill characters
        let leading_count = value.chars().take_while(|&c| c == fill_ch).count();
        let trailing_count = value.chars().rev().take_while(|&c| c == fill_ch).count();
        // Avoid underflow: if all chars are fill, content_len is 0
        let content_len = if leading_count + trailing_count >= value.len() {
            0
        } else {
            value.len() - leading_count - trailing_count
        };

        // Special case: if all chars are fill (content_len == 0), allow it if total length equals width
        if content_len == 0 {
            if let Some(width) = spec.width {
                if value.len() == width {
                    return true; // Valid: empty content, all fill, total = width
                }
            }
            return false; // Invalid: all fill but doesn't match width
        }

        match align {
            '>' => {
                // Right-aligned: fill chars should only be on the left
                // Reject if fill char on both sides (invalid) - but only if there's actual content
                if has_leading_fill && has_trailing_fill {
                    return false;
                }
                // Reject if fill char on right (should only be on left), except for a fixed-width
                // cell where width == precision == len: LTR parsing cannot separate trailing fill
                // from content the way str.format output would (parse parity; GitHub issue #97).
                if has_trailing_fill && !(spec.width == Some(prec) && value.len() == prec) {
                    return false;
                }
                // Reject if content exceeds precision
                if content_len > prec {
                    return false;
                }
                // Reject if width is specified and total width exceeds it
                // When width is specified with precision, total should not exceed width
                if let Some(width) = spec.width {
                    if value.len() > width {
                        return false;
                    }
                } else {
                    // No width specified, but precision is: reject if fill enables extra content
                    if has_leading_fill && value.len() > prec {
                        let leading_count = value.chars().take_while(|&c| c == fill_ch).count();
                        let content_len = value.len() - leading_count;
                        if content_len > prec {
                            return false;
                        }
                    }
                }
            }
            '<' => {
                // Left-aligned: fill chars should only be on the right
                // Reject if fill char on left (should only be on right)
                if has_leading_fill {
                    return false;
                }
                // Reject if content exceeds precision
                if content_len > prec {
                    return false;
                }
                // Reject if width is specified and total width exceeds it
                if let Some(width) = spec.width {
                    if value.len() > width {
                        return false;
                    }
                } else {
                    // No width specified, but precision is: reject if fill enables extra content
                    if has_trailing_fill && value.len() > prec {
                        let trailing_count =
                            value.chars().rev().take_while(|&c| c == fill_ch).count();
                        let content_len = value.len() - trailing_count;
                        if content_len > prec {
                            return false;
                        }
                    }
                }
            }
            '^' => {
                // Center-aligned: reject if content exceeds precision
                if content_len > prec {
                    return false;
                }
                // Reject if width is specified and total width exceeds it
                if let Some(width) = spec.width {
                    if value.len() > width {
                        return false;
                    }
                } else {
                    // No width specified, but precision is: reject if content exceeds precision
                    if content_len > prec {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    true
}

/// Like [`validate_alignment_precision`], but for captures that may include backslash line
/// continuations: multiline and indent-block slices are folded first (issue #80).
pub(crate) fn validate_alignment_precision_for_capture(spec: &FieldSpec, raw: &str) -> bool {
    if matches!(
        spec.field_type,
        FieldType::Multiline | FieldType::IndentBlock
    ) {
        let folded = formatparse_core::normalize_input_line_continuations(raw);
        validate_alignment_precision(spec, folded.as_str())
    } else {
        validate_alignment_precision(spec, raw)
    }
}

/// Trim fill / whitespace for string-like fields that support alignment (``String``, ``Multiline``, ``IndentBlock``).
pub(crate) fn trim_string_or_multiline_value<'a>(
    spec: &FieldSpec,
    value: &'a str,
) -> std::borrow::Cow<'a, str> {
    use std::borrow::Cow;
    if spec.alignment.is_none() {
        return Cow::Borrowed(value);
    }
    let trimmed = match spec.alignment {
        Some('<') => {
            if spec.width.is_some() {
                value
            } else if let Some(fill_ch) = spec.fill {
                value.trim_end_matches(fill_ch).trim_end()
            } else {
                value.trim_end()
            }
        }
        Some('>') => {
            if spec.fill == Some('0')
                && spec.width == spec.precision
                && spec.width == Some(value.len())
            {
                value
            } else if let Some(fill_ch) = spec.fill {
                value.trim_start_matches(fill_ch).trim_start()
            } else {
                value.trim_start()
            }
        }
        Some('^') => {
            if let Some(fill_ch) = spec.fill {
                value.trim_matches(fill_ch).trim()
            } else {
                value.trim()
            }
        }
        _ => value,
    };
    Cow::Borrowed(trimmed)
}

pub fn convert_value(
    spec: &FieldSpec,
    value: &str,
    py: Python,
    custom_converters: &HashMap<String, PyObject>,
) -> PyResult<PyObject> {
    // Fast path: if no custom converters, skip the lookup entirely
    if !custom_converters.is_empty() {
        // Check if this type has a custom converter (even if it's a built-in type name)
        let type_name = match &spec.field_type {
            FieldType::Custom(name) => name.as_str(),
            FieldType::String => "s",
            FieldType::Integer => "d",
            FieldType::Float => "f",
            FieldType::Boolean => "b",
            FieldType::Letters => "l",
            FieldType::Word => "w",
            FieldType::NonLetters => "W",
            FieldType::NonWhitespace => "S",
            FieldType::NonDigits => "D",
            FieldType::NumberWithThousands => "n",
            FieldType::Scientific => "e",
            FieldType::GeneralNumber => "g",
            FieldType::Percentage => "%",
            FieldType::DateTimeISO => "ti",
            FieldType::DateTimeRFC2822 => "te",
            FieldType::DateTimeGlobal => "tg",
            FieldType::DateTimeUS => "ta",
            FieldType::DateTimeCtime => "tc",
            FieldType::DateTimeHTTP => "th",
            FieldType::DateTimeTime => "tt",
            FieldType::DateTimeSystem => "ts",
            FieldType::DateTimeStrftime => "strftime",
            FieldType::BracedContent => "brace",
            FieldType::Multiline => "ml",
            FieldType::IndentBlock => "blk",
            FieldType::Nested => "nested",
        };

        // If there's a custom converter for this type name, use it instead of built-in
        if let Some(converter) = custom_converters.get(type_name) {
            let args = (value,);
            return converter.call1(py, args);
        }
    }

    match convert_builtin_scalar(spec, value) {
        ConvertOutcome::Ok(scalar) => scalar.to_py_object(py),
        ConvertOutcome::Err(_) => Err(error::conversion_error(
            value,
            builtin_conversion_type_label(&spec.field_type),
        )),
        ConvertOutcome::NeedsPython => match &spec.field_type {
            FieldType::DateTimeISO => datetime::parse_iso_datetime(py, value),
            FieldType::DateTimeRFC2822 => datetime::parse_rfc2822_datetime(py, value),
            FieldType::DateTimeGlobal => datetime::parse_global_datetime(py, value),
            FieldType::DateTimeUS => datetime::parse_us_datetime(py, value),
            FieldType::DateTimeCtime => datetime::parse_ctime_datetime(py, value),
            FieldType::DateTimeHTTP => datetime::parse_http_datetime(py, value),
            FieldType::DateTimeTime => datetime::parse_time(py, value),
            FieldType::DateTimeSystem => datetime::parse_system_datetime(py, value),
            FieldType::DateTimeStrftime => {
                if let Some(fmt) = &spec.strftime_format {
                    datetime::parse_strftime_datetime(py, value, fmt)
                } else {
                    Ok(value.into_py_any(py)?)
                }
            }
            FieldType::BracedContent => Ok(value.into_py_any(py)?),
            FieldType::Nested => Err(error::conversion_error(
                value,
                "nested format (handled by parser, not convert_value)",
            )),
            FieldType::Custom(_) => Ok(value.into_py_any(py)?),
            _ => Err(error::conversion_error(
                value,
                builtin_conversion_type_label(&spec.field_type),
            )),
        },
    }
}

fn builtin_conversion_type_label(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::Integer => "integer",
        FieldType::Float => "float",
        FieldType::NumberWithThousands => "number with thousands",
        FieldType::Scientific => "scientific notation",
        FieldType::GeneralNumber => "number",
        FieldType::Percentage => "percentage",
        _ => "value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_alignment_precision_right_align_valid() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            width: Some(10),
            precision: Some(5),
            alignment: Some('>'),
            fill: Some(' '),
            ..Default::default()
        };

        // Right-aligned: fill chars on left only
        assert!(validate_alignment_precision(&spec, "     hello")); // 5 spaces + 5 chars = 10 total, 5 content
        assert!(validate_alignment_precision(&spec, "hello")); // No fill, just content
        assert!(!validate_alignment_precision(&spec, "     hello ")); // Fill on right (invalid)
    }

    #[test]
    fn test_validate_alignment_precision_left_align_valid() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            width: Some(10),
            precision: Some(5),
            alignment: Some('<'),
            fill: Some(' '),
            ..Default::default()
        };

        // Left-aligned: fill chars on right only
        assert!(validate_alignment_precision(&spec, "hello     ")); // 5 chars + 5 spaces = 10 total
        assert!(validate_alignment_precision(&spec, "hello")); // No fill, just content
        assert!(!validate_alignment_precision(&spec, " hello     ")); // Fill on left (invalid)
    }

    #[test]
    fn test_validate_alignment_precision_center_align_valid() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            width: Some(10),
            precision: Some(5),
            alignment: Some('^'),
            fill: Some(' '),
            ..Default::default()
        };

        // Center-aligned: fill chars on both sides
        assert!(validate_alignment_precision(&spec, "  hello   ")); // 2 spaces + 5 chars + 3 spaces = 10 total
        assert!(validate_alignment_precision(&spec, "hello")); // No fill, just content
        assert!(!validate_alignment_precision(&spec, "  hello  x")); // Content exceeds precision (6 > 5)
    }

    #[test]
    fn test_validate_alignment_precision_all_fill_chars() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            width: Some(5),
            precision: Some(3),
            alignment: Some('>'),
            fill: Some('x'),
            ..Default::default()
        };

        // All fill chars, matches width - should be valid
        assert!(validate_alignment_precision(&spec, "xxxxx")); // 5 x's, matches width

        // All fill chars, doesn't match width - should be invalid
        assert!(!validate_alignment_precision(&spec, "xxxx")); // 4 x's, doesn't match width 5
    }

    #[test]
    fn test_validate_alignment_precision_non_string_type() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            width: Some(10),
            precision: Some(5),
            alignment: Some('>'),
            ..Default::default()
        };

        // Non-string types should always return true (no validation)
        assert!(validate_alignment_precision(&spec, "12345"));
        assert!(validate_alignment_precision(&spec, "anything"));
    }

    #[test]
    fn validate_issue40_zero_fill_right_width_precision() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            fill: Some('0'),
            alignment: Some('>'),
            width: Some(18),
            precision: Some(18),
            ..Default::default()
        };
        assert!(validate_alignment_precision(&spec, "000000000000100000"));
    }

    #[test]
    fn test_validate_alignment_precision_no_precision_or_alignment() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            width: Some(10),
            precision: None,
            alignment: None,
            ..Default::default()
        };

        // No precision or alignment means no validation
        assert!(validate_alignment_precision(&spec, "any string"));
        assert!(validate_alignment_precision(
            &spec,
            "very long string that exceeds width"
        ));
    }
}
