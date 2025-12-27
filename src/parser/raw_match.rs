use std::collections::HashMap;
use pyo3::prelude::*;
use crate::types::definitions::{FieldSpec, FieldType};

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
impl FieldSpec {
    pub fn convert_value_raw(&self, value: &str) -> Result<RawValue, String> {
        // Handle custom converters - for now, we'll need to handle this differently
        // Custom converters require Python, so we'll need a hybrid approach
        // For now, only handle built-in types
        
        match &self.field_type {
            FieldType::String => {
                // Fast path: no alignment means no trimming needed
                if self.alignment.is_none() {
                    Ok(RawValue::String(value.to_string()))
                } else {
                    // Strip fill characters and whitespace based on alignment
                    let trimmed = match self.alignment {
                        Some('<') => {
                            // Left-aligned: strip trailing fill chars, then trailing spaces
                            if let Some(fill_ch) = self.fill {
                                value.trim_end_matches(fill_ch).trim_end()
                            } else {
                                value.trim_end()
                            }
                        },
                        Some('>') => {
                            // Right-aligned: strip leading fill chars, then leading spaces
                            if let Some(fill_ch) = self.fill {
                                value.trim_start_matches(fill_ch).trim_start()
                            } else {
                                value.trim_start()
                            }
                        },
                        Some('^') => {
                            // Center-aligned: strip both leading and trailing fill chars, then spaces
                            if let Some(fill_ch) = self.fill {
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
                if self.fill.is_none() && self.alignment != Some('=') && self.original_type_char.is_none() {
                    // Try parsing directly first (most common case)
                    if let Ok(n) = value.trim().parse::<i64>() {
                        return Ok(RawValue::Integer(n));
                    }
                }
                
                // Full path: handle all cases
                let mut trimmed_str = value.trim().to_string();
                
                // Strip fill characters if alignment is '='
                if let (Some(fill_ch), Some('=')) = (self.fill, self.alignment) {
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
                    let result = if self.original_type_char == Some('x') || self.original_type_char == Some('X') {
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
                    let result = match self.original_type_char {
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
                Err(format!("Type {:?} requires Python conversion", self.field_type))
            }
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

