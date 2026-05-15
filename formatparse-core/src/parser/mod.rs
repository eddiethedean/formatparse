/// Parser module for formatparse-core
pub mod pattern;
pub mod regex;

/// Security constants for input validation
pub const MAX_PATTERN_LENGTH: usize = 10_000;
pub const MAX_INPUT_LENGTH: usize = 10_000_000; // 10MB
pub const MAX_FIELDS: usize = 100;
pub const MAX_FIELD_NAME_LENGTH: usize = 200;

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
///
/// Handles `(?P<name>...)` named captures; other `(?...)` extensions are treated as non-capturing
/// at the opening parenthesis (same rule of thumb as skipping `(?:...)`, `(?=...)`, etc.).
pub fn count_capturing_groups(pattern: &str) -> usize {
    let mut count = 0;
    let mut i = 0;
    let chars: Vec<char> = pattern.chars().collect();

    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            if i > chars.len() {
                break;
            }
            continue;
        }
        if chars[i] == '(' {
            if i + 1 < chars.len() && chars[i + 1] == '?' {
                i += 2;
                if i + 1 < chars.len() && chars[i] == 'P' && chars[i + 1] == '<' {
                    i += 2;
                    while i < chars.len() && chars[i] != '>' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1;
                    }
                    count += 1;
                    continue;
                }
                continue;
            }
            count += 1;
        }
        i += 1;
    }
    count
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
}
