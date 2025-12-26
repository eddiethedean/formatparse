use crate::datetime;
use crate::error;
use super::definitions::{FieldSpec, FieldType};
use pyo3::prelude::*;
use std::collections::HashMap;

impl FieldSpec {
    pub fn convert_value(&self, value: &str, py: Python, custom_converters: &HashMap<String, PyObject>) -> PyResult<PyObject> {
        // Check if this type has a custom converter (even if it's a built-in type name)
        let type_name = match &self.field_type {
            FieldType::Custom(name) => name.clone(),
            FieldType::String => "s".to_string(),
            FieldType::Integer => "d".to_string(),  // Use 'd' as the canonical integer type name
            FieldType::Float => "f".to_string(),
            FieldType::Boolean => "b".to_string(),
            FieldType::Letters => "l".to_string(),
            FieldType::Word => "w".to_string(),
            FieldType::NonLetters => "W".to_string(),
            FieldType::NonWhitespace => "S".to_string(),
            FieldType::NonDigits => "D".to_string(),
            FieldType::NumberWithThousands => "n".to_string(),
            FieldType::Scientific => "e".to_string(),
            FieldType::GeneralNumber => "g".to_string(),
            FieldType::Percentage => "%".to_string(),
            FieldType::DateTimeISO => "ti".to_string(),
            FieldType::DateTimeRFC2822 => "te".to_string(),
            FieldType::DateTimeGlobal => "tg".to_string(),
            FieldType::DateTimeUS => "ta".to_string(),
            FieldType::DateTimeCtime => "tc".to_string(),
            FieldType::DateTimeHTTP => "th".to_string(),
            FieldType::DateTimeTime => "tt".to_string(),
            FieldType::DateTimeSystem => "ts".to_string(),
            FieldType::DateTimeStrftime => "strftime".to_string(),
        };
        
        // If there's a custom converter for this type name, use it instead of built-in
        if custom_converters.contains_key(&type_name) {
            if let Some(converter) = custom_converters.get(&type_name) {
                let args = (value,);
                return converter.call1(py, args);
            }
        }
        
        // Use built-in conversion
        match &self.field_type {
            FieldType::String => {
                // Strip whitespace based on alignment
                let trimmed = match self.alignment {
                    Some('<') => value.trim_end(),  // Left-aligned: strip trailing spaces
                    Some('>') => value.trim_start(), // Right-aligned: strip leading spaces
                    Some('^') => value.trim(),      // Center-aligned: strip both
                    _ => value,                     // No alignment: keep as-is
                };
                Ok(trimmed.to_object(py))
            },
            FieldType::Integer => {
                // Strip whitespace before parsing (width may include spaces)
                let mut trimmed_str = value.trim().to_string();
                
                // Strip fill characters if alignment is '=' with fill
                // Fill characters appear between sign and digits (e.g., "-xxx12" or "+xxx12")
                // But NOT between sign and prefix (e.g., "-0o10" should not strip '0')
                if let (Some(fill_ch), Some('=')) = (self.fill, self.alignment) {
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
                    let result = if self.original_type_char == Some('x') || self.original_type_char == Some('X') {
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
                    let result = match self.original_type_char {
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
                // Strip whitespace before parsing (width may include spaces)
                let trimmed = value.trim();
                match trimmed.parse::<f64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(error::conversion_error(value, "float")),
                }
            }
            FieldType::Boolean => {
                let lower = value.to_lowercase();
                let b = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
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
                if let Some(ref fmt) = self.strftime_format {
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
}

