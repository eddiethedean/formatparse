/// Parser module for formatparse-core
pub mod regex;

/// Security constants for input validation
pub const MAX_PATTERN_LENGTH: usize = 10_000;
pub const MAX_INPUT_LENGTH: usize = 10_000_000;  // 10MB
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
