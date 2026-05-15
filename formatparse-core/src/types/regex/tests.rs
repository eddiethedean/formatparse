use super::helpers::STRING_WIDTH_PRECISION_CHAR;
use super::strftime_to_regex;
use crate::types::definitions::{FieldSpec, FieldType};
use fancy_regex::Regex;
use std::collections::HashMap;

#[test]
fn test_strftime_to_regex_year() {
    assert_eq!(strftime_to_regex("%Y"), r"\d{4}");
    assert_eq!(strftime_to_regex("%y"), r"\d{2}");
}

#[test]
fn test_strftime_to_regex_date() {
    assert_eq!(strftime_to_regex("%m"), r"\d{1,2}");
    assert_eq!(strftime_to_regex("%d"), r"\d{1,2}");
}

#[test]
fn test_strftime_to_regex_month_names() {
    assert_eq!(strftime_to_regex("%b"), r"[A-Za-z]{3}");
    assert_eq!(strftime_to_regex("%B"), r"[A-Za-z]+");
    assert_eq!(strftime_to_regex("%h"), r"[A-Za-z]{3}");
}

#[test]
fn test_strftime_to_regex_weekday() {
    assert_eq!(strftime_to_regex("%a"), r"[A-Za-z]{3}");
    assert_eq!(strftime_to_regex("%A"), r"[A-Za-z]+");
    assert_eq!(strftime_to_regex("%w"), r"\d");
}

#[test]
fn test_strftime_to_regex_literal() {
    assert_eq!(strftime_to_regex("%%"), "%");
}

#[test]
fn test_strftime_to_regex_complex() {
    let result = strftime_to_regex("%Y-%m-%d");
    assert!(result.contains(r"\d{4}"));
    assert!(result.contains(r"\d{1,2}"));
    // Should escape the dashes
    assert!(result.contains(r"\-"));
}

#[test]
fn test_strftime_to_regex_unknown() {
    let result = strftime_to_regex("%Z");
    assert_eq!(result, ".+?");
}

#[test]
fn test_field_spec_indent_block_matches_multiline_regex() {
    let mut ml = FieldSpec::new();
    ml.field_type = FieldType::Multiline;
    ml.width = Some(3);
    let mut blk = FieldSpec::new();
    blk.field_type = FieldType::IndentBlock;
    blk.width = Some(3);
    let custom = HashMap::new();
    assert_eq!(
        ml.to_regex_pattern(&custom, None, false),
        blk.to_regex_pattern(&custom, None, false)
    );
}

#[test]
fn test_field_spec_string_default() {
    let spec = FieldSpec::new();
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r".+?");
}

#[test]
fn test_field_spec_string_delimited_allows_empty_capture() {
    let spec = FieldSpec::new();
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, true);
    assert_eq!(pattern, r".*?");
}

#[test]
fn test_field_spec_multiline_with_width() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Multiline;
    spec.width = Some(3);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r"[\s\S]{3,}");
}

#[test]
fn test_field_spec_multiline_with_width_exact_next_field() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Multiline;
    spec.width = Some(4);
    let pattern = spec.to_regex_pattern(&HashMap::new(), Some(false), false);
    assert_eq!(pattern, r"[\s\S]{4}");
}

#[test]
fn test_field_spec_multiline_align_right() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Multiline;
    spec.alignment = Some('>');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r" *([\s\S]+?)");
}

#[test]
fn test_field_spec_multiline_precision_left_fill() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Multiline;
    spec.precision = Some(3);
    spec.alignment = Some('<');
    spec.fill = Some('.');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r"[\s\S]{3}(?:\.*)");
}

#[test]
fn test_field_spec_string_with_precision() {
    let mut spec = FieldSpec::new();
    spec.precision = Some(5);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{5}");
    assert_eq!(pattern, expected);
}

#[test]
fn test_field_spec_string_with_width() {
    let mut spec = FieldSpec::new();
    spec.width = Some(10);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{10,}");
    assert_eq!(pattern, expected);
}

#[test]
fn test_field_spec_string_with_width_next_greedy() {
    let mut spec = FieldSpec::new();
    spec.width = Some(10);
    // When next field is greedy, use greedy pattern
    let pattern = spec.to_regex_pattern(&HashMap::new(), Some(true), false);
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{10,}");
    assert_eq!(pattern, expected);
}

#[test]
fn test_field_spec_string_with_width_next_non_greedy() {
    let mut spec = FieldSpec::new();
    spec.width = Some(10);
    // When next field is non-greedy (like {}), use exact width
    let pattern = spec.to_regex_pattern(&HashMap::new(), Some(false), false);
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{10}");
    assert_eq!(pattern, expected);
}

#[test]
fn test_string_precision_under_dotall_rejects_line_separator_in_run() {
    use regex::Regex;
    let mut spec = FieldSpec::new();
    spec.precision = Some(4);
    let p = spec.to_regex_pattern(&HashMap::new(), None, false);
    let re = Regex::new(&format!(r"(?s)^{}$", p)).unwrap();
    assert!(re.is_match("abcd"));
    assert!(!re.is_match("abc\u{2028}"));
    assert!(!re.is_match("abc\u{2029}"));
    assert!(!re.is_match("abc\u{0085}"));
    assert!(!re.is_match("ab\u{000B}c"));
}

#[test]
fn test_field_spec_string_with_alignment_left() {
    let mut spec = FieldSpec::new();
    spec.alignment = Some('<');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"([^\{\}\s]+"));
}

#[test]
fn test_field_spec_string_with_alignment_right() {
    let mut spec = FieldSpec::new();
    spec.alignment = Some('>');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r" *(.+?)");
}

#[test]
fn test_field_spec_string_with_alignment_center() {
    let mut spec = FieldSpec::new();
    spec.alignment = Some('^');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"([^\{\}\s]+"));
}

#[test]
fn test_field_spec_string_with_precision_and_alignment() {
    let mut spec = FieldSpec::new();
    spec.precision = Some(5);
    spec.alignment = Some('<');
    spec.fill = Some('x');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{5}");
    assert!(pattern.contains(&expected));
    assert!(pattern.contains("x*"));
}

#[test]
fn test_field_spec_string_width_eq_precision_omits_optional_fill() {
    // Issue #97: width == precision is an exact-width cell; optional fill after `{prec}`
    // would greedily consume characters that belong to a following literal.
    let mut spec = FieldSpec::new();
    spec.precision = Some(5);
    spec.width = Some(5);
    spec.alignment = Some('<');
    let mut expected = STRING_WIDTH_PRECISION_CHAR.to_string();
    expected.push_str("{5}");
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, expected);

    let mut spec_r = FieldSpec::new();
    spec_r.precision = Some(5);
    spec_r.width = Some(5);
    spec_r.alignment = Some('>');
    assert_eq!(
        spec_r.to_regex_pattern(&HashMap::new(), None, false),
        expected
    );

    let mut spec_c = FieldSpec::new();
    spec_c.precision = Some(5);
    spec_c.width = Some(5);
    spec_c.alignment = Some('^');
    assert_eq!(
        spec_c.to_regex_pattern(&HashMap::new(), None, false),
        expected
    );
}

#[test]
fn test_field_spec_integer() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[+-]?"));
    assert!(pattern.contains(r"\s*[0-9]+") || pattern.contains(r"[0-9]+"));
}

#[test]
fn test_field_spec_integer_decimal_leading_whitespace_before_digits() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    let p = spec.to_regex_pattern(&HashMap::new(), None, false);
    let re = Regex::new(&format!("^{}$", p)).unwrap();
    assert!(re.is_match("    0").unwrap());
    assert!(re.is_match("  42").unwrap());
}

#[test]
fn test_field_spec_integer_with_zero_pad() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.zero_pad = true;
    spec.width = Some(5);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("[0-9]{1,5}"));
}

#[test]
fn test_field_spec_integer_hex() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.original_type_char = Some('x');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("0[xX]"));
    assert!(pattern.contains("[0-9a-fA-F]+"));
}

#[test]
fn test_field_spec_integer_octal() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.original_type_char = Some('o');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("0[oO]"));
    assert!(pattern.contains("[0-7]+"));
}

#[test]
fn test_field_spec_integer_binary() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.original_type_char = Some('b');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("0[bB]"));
    assert!(pattern.contains("[01]+"));
}

#[test]
fn test_field_spec_float() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Float;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[+-]?"));
    assert!(pattern.contains(r"\d+\.\d+"));
}

#[test]
fn test_field_spec_float_with_precision() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Float;
    spec.precision = Some(2);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"\.\d{2}"));
}

#[test]
fn test_field_spec_float_precision_zero_accepts_integer_like_text() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Float;
    spec.precision = Some(0);
    spec.width = Some(2);
    let p = spec.to_regex_pattern(&HashMap::new(), None, false);
    let re = Regex::new(&format!("^{}$", p)).unwrap();
    assert!(re.is_match("20").unwrap());
    assert!(re.is_match(" 20").unwrap());
    assert!(re.is_match("20.000").unwrap());
}

#[test]
fn test_field_spec_boolean() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Boolean;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("true"));
    assert!(pattern.contains("false"));
    assert!(pattern.contains("1"));
    assert!(pattern.contains("0"));
}

#[test]
fn test_field_spec_letters() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Letters;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r"[a-zA-Z]+");
}

#[test]
fn test_field_spec_word() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Word;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r"\w+");
}

#[test]
fn test_field_spec_datetime_iso() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::DateTimeISO;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"\d{4}-\d{2}-\d{2}"));
}

#[test]
fn test_field_spec_datetime_strftime() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::DateTimeStrftime;
    spec.strftime_format = Some("%Y-%m-%d".to_string());
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"\d{4}"));
    assert!(pattern.contains(r"\d{1,2}"));
}

#[test]
fn test_field_spec_custom_type() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Custom("MyType".to_string());
    let mut custom_patterns = HashMap::new();
    custom_patterns.insert("MyType".to_string(), r"\d+".to_string());
    let pattern = spec.to_regex_pattern(&custom_patterns, None, false);
    assert_eq!(pattern, r"\d+");
}

#[test]
fn test_field_spec_custom_type_no_pattern() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Custom("MyType".to_string());
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    // Should default to non-whitespace
    assert_eq!(pattern, r"\S+");
}

#[test]
fn test_field_spec_braced_content_placeholder_regex() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::BracedContent;
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert_eq!(pattern, r".*?");
}

#[test]
fn test_field_spec_integer_sign_plus() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.sign = Some('+');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"\+?"));
}

#[test]
fn test_field_spec_integer_sign_space() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.sign = Some(' ');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[- ]?"));
}

#[test]
fn test_field_spec_integer_fill_equals_alignment() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.fill = Some('x');
    spec.alignment = Some('=');
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    // Should have fill pattern between sign and digits
    assert!(pattern.contains("x*"));
}

#[test]
fn test_field_spec_integer_decimal_width_and_precision_bounded() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.width = Some(2);
    spec.precision = Some(2);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[0-9]{2,2}"), "pattern: {}", pattern);
    let re = Regex::new(&format!("^{}$", pattern)).unwrap();
    assert!(re.is_match("99").unwrap());
    assert!(!re.is_match("999").unwrap());
}

#[test]
fn test_field_spec_integer_hex_width_and_precision_bounded() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.original_type_char = Some('x');
    spec.width = Some(2);
    spec.precision = Some(2);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(
        pattern.contains(r"[0-9a-fA-F]{2,2}"),
        "pattern: {}",
        pattern
    );
}

#[test]
fn test_field_spec_integer_width_gt_precision_nonzero_pad_degenerates() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.width = Some(5);
    spec.precision = Some(2);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[0-9]{2,2}"), "pattern: {}", pattern);
}

#[test]
fn test_field_spec_integer_width_gt_precision_zero_pad_uses_width_only() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.zero_pad = true;
    spec.width = Some(30);
    spec.precision = Some(2);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains("[0-9]{1,30}"), "pattern: {}", pattern);
}

#[test]
fn test_field_spec_integer_decimal_width_2_precision_5() {
    let mut spec = FieldSpec::new();
    spec.field_type = FieldType::Integer;
    spec.width = Some(2);
    spec.precision = Some(5);
    let pattern = spec.to_regex_pattern(&HashMap::new(), None, false);
    assert!(pattern.contains(r"[0-9]{2,5}"), "pattern: {}", pattern);
}
