use std::collections::HashMap;
use pyo3::prelude::*;
use formatparse_core::{FieldSpec, FieldType};

/// Raw match data without Python objects (for batch processing)
/// This allows us to collect all matches first, then batch convert to Python objects
#[derive(Clone, Debug)]
pub struct RawMatchData {
    pub fixed: Vec<RawValue>,
    pub named: HashMap<String, RawValue>,
    pub span: (usize, usize),
    pub field_spans: HashMap<String, (usize, usize)>,
}

/// Raw value types (Rust types, not Python objects)
#[derive(Clone, Debug)]
pub enum RawValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    None,
}

impl RawMatchData {
    pub fn new() -> Self {
        Self {
            fixed: Vec::new(),
            named: HashMap::new(),
            span: (0, 0),
            field_spans: HashMap::new(),
        }
    }
    
    pub fn with_capacity(field_count: usize) -> Self {
        Self {
            fixed: Vec::with_capacity(field_count),
            named: HashMap::with_capacity(field_count),
            span: (0, 0),
            field_spans: HashMap::with_capacity(field_count),
        }
    }
}

/// Convert a value string to RawValue (no Python objects created)
/// This is used for batch processing to defer Python object creation
pub fn convert_value_raw(spec: &FieldSpec, value: &str) -> Result<RawValue, String> {
        // Handle custom converters - for now, we'll need to handle this differently
        // Custom converters require Python, so we'll need a hybrid approach
        // For now, only handle built-in types
        
        match &spec.field_type {
            FieldType::String => {
                // Fast path: no alignment means no trimming needed
                if spec.alignment.is_none() {
                    Ok(RawValue::String(value.to_string()))
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
                    Ok(RawValue::String(trimmed.to_string()))
                }
            },
            FieldType::Integer => {
                // Fast path: common case - decimal integer, no special formatting
                if spec.fill.is_none() && spec.alignment != Some('=') && spec.original_type_char.is_none() {
                    // Try parsing directly first (most common case)
                    if let Ok(n) = value.trim().parse::<i64>() {
                        return Ok(RawValue::Integer(n));
                    }
                }
                
                // Full path: handle all cases
                let mut trimmed_str = value.trim().to_string();
                
                // Strip fill characters if alignment is '='
                if let (Some(fill_ch), Some('=')) = (spec.fill, spec.alignment) {
                    if trimmed_str.starts_with('-') || trimmed_str.starts_with('+') {
                        let sign_char = &trimmed_str[..1];
                        let rest = &trimmed_str[1..];
                        if rest.starts_with("0x") || rest.starts_with("0X") || 
                           rest.starts_with("0o") || rest.starts_with("0O") ||
                           rest.starts_with("0b") || rest.starts_with("0B") {
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        } else {
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        }
                    } else {
                        trimmed_str = trimmed_str.trim_start_matches(fill_ch).to_string();
                    }
                }
                
                let trimmed = trimmed_str.as_str();
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
                    let result = if spec.original_type_char == Some('x') || spec.original_type_char == Some('X') {
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
                    let result = match spec.original_type_char {
                        Some('b') => i64::from_str_radix(num_str, 2),
                        Some('o') => i64::from_str_radix(num_str, 8),
                        Some('x') | Some('X') => i64::from_str_radix(num_str, 16),
                        _ => num_str.parse::<i64>(),
                    };
                    result.map(|n| if is_negative { -n } else { n })
                };
                
                match v {
                    Ok(n) => Ok(RawValue::Integer(n)),
                    Err(_) => Err(format!("Could not convert '{}' to integer", value)),
                }
            }
            FieldType::Float => {
                match value.parse::<f64>() {
                    Ok(n) => Ok(RawValue::Float(n)),
                    Err(_) => {
                        let trimmed = value.trim();
                        match trimmed.parse::<f64>() {
                            Ok(n) => Ok(RawValue::Float(n)),
                            Err(_) => Err(format!("Could not convert '{}' to float", value)),
                        }
                    }
                }
            }
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
                Ok(RawValue::Boolean(b))
            }
            FieldType::Letters | FieldType::Word | FieldType::NonLetters | 
            FieldType::NonWhitespace | FieldType::NonDigits => {
                Ok(RawValue::String(value.to_string()))
            }
            FieldType::NumberWithThousands => {
                let trimmed = value.trim();
                let cleaned = trimmed.replace(",", "").replace(".", "");
                match cleaned.parse::<i64>() {
                    Ok(n) => Ok(RawValue::Integer(n)),
                    Err(_) => Err(format!("Could not convert '{}' to number with thousands", value)),
                }
            }
            FieldType::Scientific => {
                match value.parse::<f64>() {
                    Ok(n) => Ok(RawValue::Float(n)),
                    Err(_) => Err(format!("Could not convert '{}' to scientific notation", value)),
                }
            }
            FieldType::GeneralNumber => {
                // Try integer first, then float
                if let Ok(n) = value.trim().parse::<i64>() {
                    Ok(RawValue::Integer(n))
                } else if let Ok(n) = value.trim().parse::<f64>() {
                    Ok(RawValue::Float(n))
                } else {
                    Err(format!("Could not convert '{}' to number", value))
                }
            }
            FieldType::Percentage => {
                let trimmed = value.trim_end_matches('%').trim();
                match trimmed.parse::<f64>() {
                    Ok(n) => Ok(RawValue::Float(n / 100.0)),
                    Err(_) => Err(format!("Could not convert '{}' to percentage", value)),
                }
            }
            // DateTime types and Custom types need Python, so we'll handle them differently
            _ => {
                // For types that require Python (datetime, custom), we can't convert to raw
                // This will be handled by falling back to the Python path
                Err(format!("Type {:?} requires Python conversion", spec.field_type))
        }
    }
}

/// Convert RawValue to PyObject (batch conversion)
impl RawValue {
    pub fn to_py_object(&self, py: Python) -> PyObject {
        match self {
            RawValue::String(s) => s.to_object(py),
            RawValue::Integer(n) => n.to_object(py),
            RawValue::Float(f) => f.to_object(py),
            RawValue::Boolean(b) => b.to_object(py),
            RawValue::None => py.None(),
        }
    }
}

/// Convert RawMatchData to ParseResult Python object (optimized batch conversion)
impl RawMatchData {
    pub fn to_parse_result(&self, py: Python) -> PyResult<pyo3::Py<crate::result::ParseResult>> {
        use crate::result::ParseResult;
        
        // Pre-allocate vectors with known capacity for better performance
        let fixed: Vec<PyObject> = self.fixed.iter()
            .map(|v| v.to_py_object(py))
            .collect();
        
        // Pre-allocate HashMap with known capacity
        let mut named: HashMap<String, PyObject> = HashMap::with_capacity(self.named.len());
        for (k, v) in &self.named {
            named.insert(k.clone(), v.to_py_object(py));
        }
        
        let parse_result = ParseResult::new_with_spans(
            fixed,
            named,
            self.span,
            self.field_spans.clone(),
        );
        
        // Py::new() is already optimized when GIL is held
        Ok(Py::new(py, parse_result)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_match_data_new() {
        let data = RawMatchData::new();
        assert!(data.fixed.is_empty());
        assert!(data.named.is_empty());
        assert_eq!(data.span, (0, 0));
        assert!(data.field_spans.is_empty());
    }

    #[test]
    fn test_raw_match_data_with_capacity() {
        let data = RawMatchData::with_capacity(10);
        assert_eq!(data.fixed.capacity(), 10);
        assert_eq!(data.named.capacity(), 10);
        assert_eq!(data.field_spans.capacity(), 10);
    }

    #[test]
    fn test_convert_value_raw_string() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "hello");
        assert!(matches!(result, Ok(RawValue::String(ref s)) if s == "hello"));
    }

    #[test]
    fn test_convert_value_raw_string_with_left_alignment() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            alignment: Some('<'),
            fill: Some(' '),
            ..Default::default()
        };
        
        // Left-aligned: strip trailing spaces
        let result = convert_value_raw(&spec, "hello     ");
        assert!(matches!(result, Ok(RawValue::String(ref s)) if s == "hello"));
    }

    #[test]
    fn test_convert_value_raw_string_with_right_alignment() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            alignment: Some('>'),
            fill: Some(' '),
            ..Default::default()
        };
        
        // Right-aligned: strip leading spaces
        let result = convert_value_raw(&spec, "     hello");
        assert!(matches!(result, Ok(RawValue::String(ref s)) if s == "hello"));
    }

    #[test]
    fn test_convert_value_raw_integer() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "123");
        assert!(matches!(result, Ok(RawValue::Integer(123))));
        
        let result_neg = convert_value_raw(&spec, "-456");
        assert!(matches!(result_neg, Ok(RawValue::Integer(-456))));
    }

    #[test]
    fn test_convert_value_raw_integer_hex() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "0xff");
        assert!(matches!(result, Ok(RawValue::Integer(255))));
        
        let result_upper = convert_value_raw(&spec, "0xFF");
        assert!(matches!(result_upper, Ok(RawValue::Integer(255))));
    }

    #[test]
    fn test_convert_value_raw_integer_octal() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "0o777");
        assert!(matches!(result, Ok(RawValue::Integer(511))));
    }

    #[test]
    fn test_convert_value_raw_integer_binary() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "0b1010");
        assert!(matches!(result, Ok(RawValue::Integer(10))));
    }

    #[test]
    fn test_convert_value_raw_float() {
        let spec = FieldSpec {
            field_type: FieldType::Float,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "3.14");
        assert!(matches!(result, Ok(RawValue::Float(f)) if (f - 3.14).abs() < 0.001));
        
        let result_neg = convert_value_raw(&spec, "-2.5");
        assert!(matches!(result_neg, Ok(RawValue::Float(f)) if (f - (-2.5)).abs() < 0.001));
    }

    #[test]
    fn test_convert_value_raw_boolean() {
        let spec = FieldSpec {
            field_type: FieldType::Boolean,
            ..Default::default()
        };
        
        assert!(matches!(convert_value_raw(&spec, "true"), Ok(RawValue::Boolean(true))));
        assert!(matches!(convert_value_raw(&spec, "TRUE"), Ok(RawValue::Boolean(true))));
        assert!(matches!(convert_value_raw(&spec, "1"), Ok(RawValue::Boolean(true))));
        assert!(matches!(convert_value_raw(&spec, "yes"), Ok(RawValue::Boolean(true))));
        assert!(matches!(convert_value_raw(&spec, "on"), Ok(RawValue::Boolean(true))));
        assert!(matches!(convert_value_raw(&spec, "false"), Ok(RawValue::Boolean(false))));
        assert!(matches!(convert_value_raw(&spec, "0"), Ok(RawValue::Boolean(false))));
    }

    #[test]
    fn test_convert_value_raw_number_with_thousands() {
        let spec = FieldSpec {
            field_type: FieldType::NumberWithThousands,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "1,000");
        assert!(matches!(result, Ok(RawValue::Integer(1000))));
        
        let result_dot = convert_value_raw(&spec, "1.000");
        assert!(matches!(result_dot, Ok(RawValue::Integer(1000))));
    }

    #[test]
    fn test_convert_value_raw_percentage() {
        let spec = FieldSpec {
            field_type: FieldType::Percentage,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "50%");
        assert!(matches!(result, Ok(RawValue::Float(f)) if (f - 0.5).abs() < 0.001));
        
        let result_no_space = convert_value_raw(&spec, "25%");
        assert!(matches!(result_no_space, Ok(RawValue::Float(f)) if (f - 0.25).abs() < 0.001));
    }

    #[test]
    fn test_convert_value_raw_scientific() {
        let spec = FieldSpec {
            field_type: FieldType::Scientific,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "1e10");
        assert!(matches!(result, Ok(RawValue::Float(f)) if (f - 1e10).abs() < 1.0));
    }

    #[test]
    fn test_convert_value_raw_general_number() {
        let spec = FieldSpec {
            field_type: FieldType::GeneralNumber,
            ..Default::default()
        };
        
        // Should parse as integer
        let result_int = convert_value_raw(&spec, "42");
        assert!(matches!(result_int, Ok(RawValue::Integer(42))));
        
        // Should parse as float
        let result_float = convert_value_raw(&spec, "3.14");
        assert!(matches!(result_float, Ok(RawValue::Float(_))));
    }

    #[test]
    fn test_convert_value_raw_invalid_integer() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "not a number");
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_value_raw_invalid_float() {
        let spec = FieldSpec {
            field_type: FieldType::Float,
            ..Default::default()
        };
        
        let result = convert_value_raw(&spec, "not a float");
        assert!(result.is_err());
    }

    // Note: This test requires Python to be linked (PyO3 dependency)
    // It will only work when running tests with Python available
    // Most other tests in this file are pure Rust and don't need Python
    #[test]
    #[cfg(feature = "python-tests")]
    fn test_raw_value_to_py_object() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let string_val = RawValue::String("hello".to_string());
            let py_obj = string_val.to_py_object(py);
            assert_eq!(py_obj.extract::<String>(py).unwrap(), "hello");
            
            let int_val = RawValue::Integer(42);
            let py_obj = int_val.to_py_object(py);
            assert_eq!(py_obj.extract::<i64>(py).unwrap(), 42);
            
            let float_val = RawValue::Float(3.14);
            let py_obj = float_val.to_py_object(py);
            assert_eq!(py_obj.extract::<f64>(py).unwrap(), 3.14);
            
            let bool_val = RawValue::Boolean(true);
            let py_obj = bool_val.to_py_object(py);
            assert_eq!(py_obj.extract::<bool>(py).unwrap(), true);
            
            let none_val = RawValue::None;
            let py_obj = none_val.to_py_object(py);
            assert!(py_obj.is_none(py));
        });
    }
}

