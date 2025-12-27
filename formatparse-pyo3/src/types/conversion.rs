use crate::datetime;
use crate::error;
use formatparse_core::{FieldSpec, FieldType};
use pyo3::prelude::*;
use std::collections::HashMap;

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
        assert!(validate_alignment_precision(&spec, "hello"));      // No fill, just content
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
        assert!(validate_alignment_precision(&spec, "hello"));      // No fill, just content
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
        assert!(validate_alignment_precision(&spec, "hello"));      // No fill, just content
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
        assert!(validate_alignment_precision(&spec, "very long string that exceeds width"));
    }
}

    /// Validate alignment+precision constraints for string fields
    /// Returns false if validation fails (should reject the match)
    /// 
    /// This validates the constraints described in issue #3 (parse#218):
    /// - Fill characters should only be in correct positions (left for right-align, right for left-align)
    /// - Total width (including fill chars) should not exceed specified width when width is specified
    /// - Content length (after removing fill chars) should not exceed precision
pub fn validate_alignment_precision(spec: &FieldSpec, value: &str) -> bool {
    if let FieldType::String = &spec.field_type {
        if let (Some(prec), Some(align)) = (spec.precision, spec.alignment) {
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
                            return true;  // Valid: empty content, all fill, total = width
                        }
                    }
                    return false;  // Invalid: all fill but doesn't match width
                }
                
                match align {
                    '>' => {
                        // Right-aligned: fill chars should only be on the left
                        // Reject if fill char on both sides (invalid) - but only if there's actual content
                        if has_leading_fill && has_trailing_fill {
                            return false;
                        }
                        // Reject if fill char on right (should only be on left)
                        if has_trailing_fill {
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
                    },
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
                                let trailing_count = value.chars().rev().take_while(|&c| c == fill_ch).count();
                                let content_len = value.len() - trailing_count;
                                if content_len > prec {
                                    return false;
                                }
                            }
                        }
                    },
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
                    },
                    _ => {}
                }
            }
        }
        true
    }

pub fn convert_value(spec: &FieldSpec, value: &str, py: Python, custom_converters: &HashMap<String, PyObject>) -> PyResult<PyObject> {
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
            };
            
            // If there's a custom converter for this type name, use it instead of built-in
            if let Some(converter) = custom_converters.get(type_name) {
                let args = (value,);
                return converter.call1(py, args);
            }
        }
        
        // Use built-in conversion
        match &spec.field_type {
            FieldType::String => {
                // Fast path: no alignment means no trimming needed
                if spec.alignment.is_none() {
                    Ok(value.to_object(py))
                } else {
                    // Strip fill characters and whitespace based on alignment
                    let trimmed = match spec.alignment {
                        Some('<') => {
                            // Left-aligned: strip trailing fill chars, then trailing spaces
                            if let Some(fill_ch) = spec.fill {
                                value.trim_end_matches(fill_ch).trim_end()
                            } else {
                                value.trim_end()
                            }
                        },
                        Some('>') => {
                            // Right-aligned: strip leading fill chars, then leading spaces
                            if let Some(fill_ch) = spec.fill {
                                value.trim_start_matches(fill_ch).trim_start()
                            } else {
                                value.trim_start()
                            }
                        },
                        Some('^') => {
                            // Center-aligned: strip both leading and trailing fill chars, then spaces
                            if let Some(fill_ch) = spec.fill {
                                value.trim_matches(fill_ch).trim()
                            } else {
                                value.trim()
                            }
                        },
                        _ => value,  // No alignment: keep as-is
                    };
                    Ok(trimmed.to_object(py))
                }
            },
            FieldType::Integer => {
                // Fast path: common case - decimal integer, no special formatting
                if spec.fill.is_none() && spec.alignment != Some('=') && spec.original_type_char.is_none() {
                    // Try parsing directly first (most common case)
                    if let Ok(n) = value.trim().parse::<i64>() {
                        return Ok(n.to_object(py));
                    }
                }
                
                // Full path: handle all cases
                // Strip whitespace before parsing (width may include spaces)
                let mut trimmed_str = value.trim().to_string();
                
                // Strip fill characters if alignment is '=' with fill
                // Fill characters appear between sign and digits (e.g., "-xxx12" or "+xxx12")
                // But NOT between sign and prefix (e.g., "-0o10" should not strip '0')
                if let (Some(fill_ch), Some('=')) = (spec.fill, spec.alignment) {
                    // Check if there's a sign first
                    if trimmed_str.starts_with('-') || trimmed_str.starts_with('+') {
                        // Keep the sign, strip fill chars after it but before the number part
                        let sign_char = &trimmed_str[..1];
                        let rest = &trimmed_str[1..];
                        // Only strip fill if it's not part of a prefix (0x, 0o, 0b)
                        if rest.starts_with("0x") || rest.starts_with("0X") || 
                           rest.starts_with("0o") || rest.starts_with("0O") ||
                           rest.starts_with("0b") || rest.starts_with("0B") {
                            // Has prefix, don't strip (fill shouldn't appear here)
                            // Actually, fill can appear: "-xxx0o10" -> strip xxx
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        } else {
                            // No prefix, strip fill chars
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        }
                    } else {
                        // No sign, just strip leading fill chars
                        trimmed_str = trimmed_str.trim_start_matches(fill_ch).to_string();
                    }
                }
                
                let trimmed = trimmed_str.as_str();
                // Handle negative numbers with prefixes (e.g., "-0o10")
                let (is_negative, num_str) = if trimmed.starts_with('-') {
                    (true, &trimmed[1..])
                } else if trimmed.starts_with('+') {
                    (false, &trimmed[1..])
                } else {
                    (false, trimmed)
                };
                
                let v = if num_str.starts_with("0x") || num_str.starts_with("0X") {
                    i64::from_str_radix(&num_str[2..], 16).map(|n| if is_negative { -n } else { n })
                } else if num_str.starts_with("0o") || num_str.starts_with("0O") {
                    i64::from_str_radix(&num_str[2..], 8).map(|n| if is_negative { -n } else { n })
                } else if num_str.starts_with("0b") || num_str.starts_with("0B") {
                    // Check if type is 'x' - if so, "0B" should be parsed as hex (0xB)
                    let result = if spec.original_type_char == Some('x') || spec.original_type_char == Some('X') {
                        // For hex type, "0B" means 0xB (hex), not binary
                        if num_str == "0B" || num_str == "0b" {
                            i64::from_str_radix("B", 16)
                        } else if num_str.len() > 2 {
                            // "0B1" should be parsed as "B1" in hex
                            i64::from_str_radix(&num_str[1..], 16)
                        } else {
                            i64::from_str_radix(&num_str[2..], 2)
                        }
                    } else {
                        i64::from_str_radix(&num_str[2..], 2)
                    };
                    result.map(|n| if is_negative { -n } else { n })
                } else {
                    // Check original type character to determine base if no prefix
                    let result = match spec.original_type_char {
                        Some('b') => i64::from_str_radix(num_str, 2), // Binary without 0b prefix
                        Some('o') => i64::from_str_radix(num_str, 8), // Octal without 0o prefix
                        Some('x') | Some('X') => i64::from_str_radix(num_str, 16), // Hex without 0x prefix
                        _ => num_str.parse::<i64>(), // Decimal
                    };
                    result.map(|n| if is_negative { -n } else { n })
                };
                match v {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(error::conversion_error(value, "integer")),
                }
            }
            FieldType::Float => {
                // Fast path: try parsing directly first (most floats don't have leading/trailing spaces)
                match value.parse::<f64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => {
                        // Fallback: strip whitespace and try again
                        let trimmed = value.trim();
                        match trimmed.parse::<f64>() {
                            Ok(n) => Ok(n.to_object(py)),
                            Err(_) => Err(error::conversion_error(value, "float")),
                        }
                    }
                }
            }
            FieldType::Boolean => {
                // Fast path: check common cases without allocation
                let b = match value.len() {
                    1 => value == "1",
                    2 => matches!(value, "on" | "ON"),
                    3 => matches!(value, "yes" | "YES"),
                    4 => matches!(value, "true" | "TRUE"),
                    _ => {
                        // Fallback: lowercase comparison
                        let lower = value.to_lowercase();
                        matches!(lower.as_str(), "true" | "1" | "yes" | "on")
                    }
                };
                Ok(b.to_object(py))
            }
            FieldType::Letters => Ok(value.to_object(py)),  // Letters are just strings
            FieldType::Word => Ok(value.to_object(py)),     // Words are just strings
            FieldType::NonLetters => Ok(value.to_object(py)), // Non-letters are just strings
            FieldType::NonWhitespace => Ok(value.to_object(py)), // Non-whitespace are just strings
            FieldType::NonDigits => Ok(value.to_object(py)), // Non-digits are just strings
            FieldType::NumberWithThousands => {
                // Strip thousands separators (comma or dot) and parse as integer
                let trimmed = value.trim();
                let cleaned = trimmed.replace(",", "").replace(".", "");
                match cleaned.parse::<i64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(error::conversion_error(value, "number with thousands")),
                }
            },
            FieldType::Scientific => {
                // Parse as float (supports scientific notation)
                let trimmed = value.trim();
                match trimmed.parse::<f64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(error::conversion_error(value, "scientific notation")),
                }
            },
            FieldType::GeneralNumber => {
                // Parse as int if possible, otherwise float, or nan/inf
                let trimmed = value.trim();
                let lower = trimmed.to_lowercase();
                // Check for nan/inf first
                if lower == "nan" {
                    Ok(f64::NAN.to_object(py))
                } else if lower == "inf" || lower == "+inf" {
                    Ok(f64::INFINITY.to_object(py))
                } else if lower == "-inf" {
                    Ok(f64::NEG_INFINITY.to_object(py))
                } else {
                    // Try int first
                    if let Ok(n) = trimmed.parse::<i64>() {
                        Ok(n.to_object(py))
                    } else if let Ok(n) = trimmed.parse::<f64>() {
                        Ok(n.to_object(py))
                    } else {
                        Err(error::conversion_error(value, "number"))
                    }
                }
            },
            FieldType::Percentage => {
                // Parse number, remove %, divide by 100
                let trimmed = value.trim();
                let num_str = trimmed.trim_end_matches('%');
                match num_str.parse::<f64>() {
                    Ok(n) => Ok((n / 100.0).to_object(py)),
                    Err(_) => Err(error::conversion_error(value, "percentage")),
                }
            },
            FieldType::DateTimeISO => {
                datetime::parse_iso_datetime(py, value)
            },
            FieldType::DateTimeRFC2822 => {
                datetime::parse_rfc2822_datetime(py, value)
            },
            FieldType::DateTimeGlobal => {
                datetime::parse_global_datetime(py, value)
            },
            FieldType::DateTimeUS => {
                datetime::parse_us_datetime(py, value)
            },
            FieldType::DateTimeCtime => {
                datetime::parse_ctime_datetime(py, value)
            },
            FieldType::DateTimeHTTP => {
                datetime::parse_http_datetime(py, value)
            },
            FieldType::DateTimeTime => {
                datetime::parse_time(py, value)
            },
            FieldType::DateTimeSystem => {
                datetime::parse_system_datetime(py, value)
            },
            FieldType::DateTimeStrftime => {
                if let Some(fmt) = &spec.strftime_format {
                    datetime::parse_strftime_datetime(py, value, fmt)
                } else {
                    Ok(value.to_object(py))
                }
            },
            FieldType::Custom(_) => {
                // Already handled above
                Ok(value.to_object(py))
            }
        }
    }
