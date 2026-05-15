//! Raw match data without Python objects (for batch processing).

use crate::types::builtin_convert::{convert_builtin_scalar, ConvertOutcome};
use formatparse_core::FieldSpec;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

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
pub fn convert_value_raw(spec: &FieldSpec, value: &str) -> Result<RawValue, String> {
    match convert_builtin_scalar(spec, value) {
        ConvertOutcome::Ok(scalar) => Ok(scalar.to_raw_value()),
        ConvertOutcome::NeedsPython => Err(format!(
            "Type {:?} requires Python conversion",
            spec.field_type
        )),
        ConvertOutcome::Err(e) => Err(e),
    }
}

/// Convert RawValue to PyObject (batch conversion)
impl RawValue {
    pub fn to_py_object(&self, py: Python) -> PyResult<PyObject> {
        match self {
            RawValue::String(s) => s.into_py_any(py),
            RawValue::Integer(n) => n.into_py_any(py),
            RawValue::Float(f) => f.into_py_any(py),
            RawValue::Boolean(b) => b.into_py_any(py),
            RawValue::None => Ok(py.None()),
        }
    }
}

/// Convert RawMatchData to ParseResult Python object (optimized batch conversion)
impl RawMatchData {
    pub fn to_parse_result(&self, py: Python) -> PyResult<pyo3::Py<crate::result::ParseResult>> {
        use crate::result::ParseResult;

        let fixed: Vec<PyObject> = self
            .fixed
            .iter()
            .map(|v| v.to_py_object(py))
            .collect::<PyResult<_>>()?;

        let mut named: HashMap<String, PyObject> = HashMap::with_capacity(self.named.len());
        for (k, v) in &self.named {
            named.insert(k.clone(), v.to_py_object(py)?);
        }

        let parse_result =
            ParseResult::new_with_spans(fixed, named, self.span, self.field_spans.clone());
        Py::new(py, parse_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use formatparse_core::{FieldSpec, FieldType};

    #[test]
    fn test_raw_match_data_new() {
        let data = RawMatchData::new();
        assert!(data.fixed.is_empty());
        assert!(data.named.is_empty());
        assert_eq!(data.span, (0, 0));
    }

    #[test]
    fn test_convert_value_raw_string() {
        let spec = FieldSpec {
            field_type: FieldType::String,
            ..Default::default()
        };
        let result = convert_value_raw(&spec, "hello");
        assert!(matches!(result, Ok(RawValue::String(s)) if s == "hello"));
    }

    #[test]
    fn test_convert_value_raw_integer() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        let result = convert_value_raw(&spec, "42");
        assert!(matches!(result, Ok(RawValue::Integer(42))));
    }

    #[test]
    fn test_convert_value_raw_invalid_integer() {
        let spec = FieldSpec {
            field_type: FieldType::Integer,
            ..Default::default()
        };
        assert!(convert_value_raw(&spec, "not a number").is_err());
    }
}
