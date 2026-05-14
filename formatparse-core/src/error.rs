//! Error types for formatparse-core
//!
//! Pure Rust error types (no PyO3 dependencies)

use std::fmt;

/// Errors that can occur during pattern parsing or matching
#[derive(Debug, Clone)]
pub enum FormatParseError {
    /// Pattern parsing error
    PatternError(String),
    /// Regex compilation error
    RegexError(String),
    /// Type conversion error
    ConversionError(String, String), // (value, target_type)
    /// Repeated field name with mismatched types
    RepeatedNameError(String),
    /// Custom type validation error
    CustomTypeError(String, String), // (type_name, message)
    /// Regex group index error
    RegexGroupIndexError(String, usize, i64), // (type_name, actual, expected)
    /// Feature not implemented
    NotImplementedError(String),
    /// Missing required field
    MissingFieldError(String),
}

impl fmt::Display for FormatParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatParseError::PatternError(msg) => write!(f, "Pattern error: {}", msg),
            FormatParseError::RegexError(msg) => write!(f, "Invalid regex pattern: {}", msg),
            FormatParseError::ConversionError(value, target_type) => {
                write!(f, "Invalid {}: {}", target_type, value)
            }
            FormatParseError::RepeatedNameError(name) => {
                write!(f, "Repeated name '{}' with mismatched types", name)
            }
            FormatParseError::CustomTypeError(type_name, msg) => {
                write!(f, "Custom type '{}' error: {}", type_name, msg)
            }
            FormatParseError::RegexGroupIndexError(type_name, actual, expected) => {
                write!(
                    f,
                    "Custom type '{}' pattern has {} capturing groups but regex_group_count is {}",
                    type_name, actual, expected
                )
            }
            FormatParseError::NotImplementedError(feature) => {
                write!(f, "{} is not supported", feature)
            }
            FormatParseError::MissingFieldError(field) => {
                write!(f, "Missing required field: {}", field)
            }
        }
    }
}

impl std::error::Error for FormatParseError {}
