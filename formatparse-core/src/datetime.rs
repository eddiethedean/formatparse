//! Pure datetime-related string parsing (no PyO3).
//!
//! Shared helpers used by the PyO3 bindings; keep logic here when it has no
//! Python dependency so it can be unit-tested from Rust alone.

use crate::error::FormatParseError;

/// Parse a fractional-second digit run to microseconds in `0..=999_999`.
///
/// Truncates to six digits when longer; right-pads with zeros when shorter
/// (same rules as the previous `datetime::common::parse_microseconds` in pyo3).
pub fn parse_microsecond_digits(micros_str: &str) -> Result<u32, FormatParseError> {
    let micros_str = if micros_str.len() > 6 {
        &micros_str[..6]
    } else {
        micros_str
    };
    let padded = format!("{:0<6}", micros_str);
    padded.parse::<u32>().map_err(|_| {
        FormatParseError::ConversionError(micros_str.to_string(), "microseconds".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_microsecond_digits_truncates_and_pads() {
        assert_eq!(parse_microsecond_digits("123456").unwrap(), 123456);
        assert_eq!(parse_microsecond_digits("123").unwrap(), 123000);
        assert_eq!(parse_microsecond_digits("12").unwrap(), 120000);
        assert_eq!(parse_microsecond_digits("1234567").unwrap(), 123456);
    }
}
