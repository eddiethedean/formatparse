/// Parser module for formatparse-core
pub mod pattern;
pub mod regex;

/// Security constants for input validation
pub const MAX_PATTERN_LENGTH: usize = 10_000;
pub const MAX_INPUT_LENGTH: usize = 10_000_000; // 10MB
pub const MAX_FIELDS: usize = 100;
pub const MAX_FIELD_NAME_LENGTH: usize = 200;
/// Maximum width or precision quantifier in a format spec (ReDoS guard).
pub const MAX_WIDTH_PRECISION: usize = 1000;

/// Validate pattern length
pub fn validate_pattern_length(pattern: &str) -> Result<(), String> {
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err(format!(
            "Pattern length {} exceeds maximum allowed length of {} characters",
            pattern.len(),
            MAX_PATTERN_LENGTH
        ));
    }
    Ok(())
}

/// Validate input string length
pub fn validate_input_length(input: &str) -> Result<(), String> {
    if input.len() > MAX_INPUT_LENGTH {
        return Err(format!(
            "Input length {} exceeds maximum allowed length of {} characters",
            input.len(),
            MAX_INPUT_LENGTH
        ));
    }
    Ok(())
}

/// Validate field name length and characters
pub fn validate_field_name(field_name: &str) -> Result<(), String> {
    if field_name.len() > MAX_FIELD_NAME_LENGTH {
        return Err(format!(
            "Field name length {} exceeds maximum allowed length of {} characters",
            field_name.len(),
            MAX_FIELD_NAME_LENGTH
        ));
    }

    // Check for null bytes
    if field_name.contains('\0') {
        return Err("Field name contains null byte".to_string());
    }

    Ok(())
}

/// Count capturing groups in a regex pattern string (for validating `with_pattern` / custom types).
pub fn count_capturing_groups(pattern: &str) -> usize {
    use fancy_regex::Regex;
    match Regex::new(pattern) {
        Ok(re) => re.captures_len().saturating_sub(1),
        Err(_) => usize::MAX,
    }
}

#[cfg(test)]
mod count_capturing_groups_tests {
    use super::count_capturing_groups;

    #[test]
    fn named_group_counts_as_one() {
        assert_eq!(count_capturing_groups(r"(?P<foo>\d+)"), 1);
    }

    #[test]
    fn named_group_with_nested_capture() {
        assert_eq!(count_capturing_groups(r"(?P<outer>\d(\d))"), 2);
    }

    #[test]
    fn non_capturing_zero() {
        assert_eq!(count_capturing_groups(r"(?:ab)"), 0);
    }

    #[test]
    fn plain_capture_plus_named() {
        assert_eq!(count_capturing_groups(r"(\w)(?P<n>\d+)"), 2);
    }

    #[test]
    fn backreference_not_counted_as_capture_here() {
        assert_eq!(count_capturing_groups(r"(?P<x>a)(?P=x)"), 1);
    }

    #[test]
    fn unicode_escape_not_counted_as_capture() {
        assert_eq!(count_capturing_groups(r"\u{28}abc"), 0);
    }

    #[test]
    fn hex_escape_not_counted_as_capture() {
        assert_eq!(count_capturing_groups(r"\x28abc"), 0);
    }
}
