//! Shared builtin scalar conversion (raw batch path and Python path).

use crate::types::conversion::trim_string_or_multiline_value;
use formatparse_core::{FieldSpec, FieldType};

/// Rust scalar produced without calling Python (except via `to_py_object` later).
#[derive(Clone, Debug, PartialEq)]
pub enum ConvertedScalar {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

/// Result of attempting builtin conversion before the Python-specific path.
#[derive(Clone, Debug, PartialEq)]
pub enum ConvertOutcome {
    Ok(ConvertedScalar),
    /// Datetime, custom, nested, or other types that need GIL / callables.
    NeedsPython,
    /// Builtin type but text did not parse.
    Err(String),
}

/// Parse integer text using the same rules as the legacy raw/Python paths.
pub fn parse_integer_text(spec: &FieldSpec, value: &str) -> Result<i64, String> {
    if spec.fill.is_none() && spec.alignment != Some('=') && spec.original_type_char.is_none() {
        if let Ok(n) = value.trim().parse::<i64>() {
            return Ok(n);
        }
    }

    let mut trimmed_str = value.trim().to_string();
    if let (Some(fill_ch), Some('=')) = (spec.fill, spec.alignment) {
        if trimmed_str.starts_with('-') || trimmed_str.starts_with('+') {
            let sign_char = &trimmed_str[..1];
            let rest = &trimmed_str[1..];
            let rest_trimmed = rest.trim_start_matches(fill_ch);
            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
        } else {
            trimmed_str = trimmed_str.trim_start_matches(fill_ch).to_string();
        }
    }

    let trimmed = trimmed_str.as_str();
    let (is_negative, num_str) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };

    let v = if num_str.starts_with("0x") || num_str.starts_with("0X") {
        i64::from_str_radix(&num_str[2..], 16).map(|n| if is_negative { -n } else { n })
    } else if num_str.starts_with("0o") || num_str.starts_with("0O") {
        i64::from_str_radix(&num_str[2..], 8).map(|n| if is_negative { -n } else { n })
    } else if num_str.starts_with("0b") || num_str.starts_with("0B") {
        let result = if spec.original_type_char == Some('x') || spec.original_type_char == Some('X')
        {
            if num_str == "0B" || num_str == "0b" {
                i64::from_str_radix("B", 16)
            } else if num_str.len() > 2 {
                i64::from_str_radix(&num_str[1..], 16)
            } else {
                i64::from_str_radix(&num_str[2..], 2)
            }
        } else {
            i64::from_str_radix(&num_str[2..], 2)
        };
        result.map(|n| if is_negative { -n } else { n })
    } else {
        match spec.original_type_char {
            Some('b') => i64::from_str_radix(num_str, 2),
            Some('o') => i64::from_str_radix(num_str, 8),
            Some('x') | Some('X') => i64::from_str_radix(num_str, 16),
            _ => num_str.parse::<i64>(),
        }
        .map(|n| if is_negative { -n } else { n })
    };

    v.map_err(|_| format!("Could not convert '{}' to integer", value))
}

/// Convert captured text for builtin field types without Python callables.
pub fn convert_builtin_scalar(spec: &FieldSpec, value: &str) -> ConvertOutcome {
    match &spec.field_type {
        FieldType::String => {
            let t = trim_string_or_multiline_value(spec, value);
            ConvertOutcome::Ok(ConvertedScalar::String(t.into_owned()))
        }
        FieldType::Multiline => {
            let folded = formatparse_core::normalize_input_line_continuations(value);
            let t = trim_string_or_multiline_value(spec, folded.as_str());
            ConvertOutcome::Ok(ConvertedScalar::String(t.into_owned()))
        }
        FieldType::IndentBlock => {
            let folded = formatparse_core::normalize_input_line_continuations(value);
            let t = trim_string_or_multiline_value(spec, folded.as_str());
            ConvertOutcome::Ok(ConvertedScalar::String(
                formatparse_core::strip_common_indent(t.as_ref()),
            ))
        }
        FieldType::Integer => match parse_integer_text(spec, value) {
            Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Integer(n)),
            Err(e) => ConvertOutcome::Err(e),
        },
        FieldType::Float => match value.parse::<f64>() {
            Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Float(n)),
            Err(_) => match value.trim().parse::<f64>() {
                Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Float(n)),
                Err(_) => ConvertOutcome::Err(format!("Could not convert '{}' to float", value)),
            },
        },
        FieldType::Boolean => {
            let b = match value.len() {
                1 => value == "1",
                2 => matches!(value, "on" | "ON"),
                3 => matches!(value, "yes" | "YES"),
                4 => matches!(value, "true" | "TRUE"),
                _ => {
                    let lower = value.to_lowercase();
                    matches!(lower.as_str(), "true" | "1" | "yes" | "on")
                }
            };
            ConvertOutcome::Ok(ConvertedScalar::Boolean(b))
        }
        FieldType::Letters
        | FieldType::Word
        | FieldType::NonLetters
        | FieldType::NonWhitespace
        | FieldType::NonDigits => ConvertOutcome::Ok(ConvertedScalar::String(value.to_string())),
        FieldType::NumberWithThousands => {
            let cleaned = value.trim().replace(",", "").replace(".", "");
            match cleaned.parse::<i64>() {
                Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Integer(n)),
                Err(_) => ConvertOutcome::Err(format!(
                    "Could not convert '{}' to number with thousands",
                    value
                )),
            }
        }
        FieldType::Scientific => match value.parse::<f64>() {
            Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Float(n)),
            Err(_) => ConvertOutcome::Err(format!(
                "Could not convert '{}' to scientific notation",
                value
            )),
        },
        FieldType::GeneralNumber => {
            let trimmed = value.trim();
            let lower = trimmed.to_lowercase();
            if lower == "nan" {
                ConvertOutcome::Ok(ConvertedScalar::Float(f64::NAN))
            } else if lower == "inf" || lower == "+inf" {
                ConvertOutcome::Ok(ConvertedScalar::Float(f64::INFINITY))
            } else if lower == "-inf" {
                ConvertOutcome::Ok(ConvertedScalar::Float(f64::NEG_INFINITY))
            } else if let Ok(n) = trimmed.parse::<i64>() {
                ConvertOutcome::Ok(ConvertedScalar::Integer(n))
            } else if let Ok(n) = trimmed.parse::<f64>() {
                ConvertOutcome::Ok(ConvertedScalar::Float(n))
            } else {
                ConvertOutcome::Err(format!("Could not convert '{}' to number", value))
            }
        }
        FieldType::Percentage => {
            let trimmed = value.trim_end_matches('%').trim();
            match trimmed.parse::<f64>() {
                Ok(n) => ConvertOutcome::Ok(ConvertedScalar::Float(n / 100.0)),
                Err(_) => {
                    ConvertOutcome::Err(format!("Could not convert '{}' to percentage", value))
                }
            }
        }
        FieldType::BracedContent => ConvertOutcome::Ok(ConvertedScalar::String(value.to_string())),
        FieldType::DateTimeISO
        | FieldType::DateTimeRFC2822
        | FieldType::DateTimeGlobal
        | FieldType::DateTimeUS
        | FieldType::DateTimeCtime
        | FieldType::DateTimeHTTP
        | FieldType::DateTimeTime
        | FieldType::DateTimeSystem
        | FieldType::DateTimeStrftime
        | FieldType::Custom(_)
        | FieldType::Nested => ConvertOutcome::NeedsPython,
    }
}

impl ConvertedScalar {
    pub fn to_py_object(
        &self,
        py: pyo3::Python<'_>,
    ) -> pyo3::PyResult<pyo3::Py<pyo3::types::PyAny>> {
        use pyo3::IntoPyObjectExt;
        match self {
            ConvertedScalar::String(s) => s.into_py_any(py),
            ConvertedScalar::Integer(n) => n.into_py_any(py),
            ConvertedScalar::Float(f) => f.into_py_any(py),
            ConvertedScalar::Boolean(b) => b.into_py_any(py),
        }
    }

    pub fn to_raw_value(&self) -> crate::parser::raw_match::RawValue {
        use crate::parser::raw_match::RawValue;
        match self {
            ConvertedScalar::String(s) => RawValue::String(s.clone()),
            ConvertedScalar::Integer(n) => RawValue::Integer(*n),
            ConvertedScalar::Float(f) => RawValue::Float(*f),
            ConvertedScalar::Boolean(b) => RawValue::Boolean(*b),
        }
    }
}
