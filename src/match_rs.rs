use crate::error;
use crate::result::ParseResult;
use crate::types::FieldSpec;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Match object that stores raw regex captures without type conversion
#[pyclass]
pub struct Match {
    pattern: String,
    field_specs: Vec<FieldSpec>,
    field_names: Vec<Option<String>>,
    normalized_names: Vec<Option<String>>,
    captures: Vec<Option<String>>,  // Raw captured strings
    named_captures: HashMap<String, String>,  // Raw captured strings by normalized name
    #[pyo3(get)]
    pub span: (usize, usize),
    field_spans: HashMap<String, (usize, usize)>,  // Spans by original field name
}

#[pymethods]
impl Match {
    /// Evaluate the match to convert captured values to their types
    #[pyo3(signature = (*, extra_types=None))]
    fn evaluate_result(&self, py: Python, extra_types: Option<HashMap<String, PyObject>>) -> PyResult<PyObject> {
        let custom_converters = extra_types.unwrap_or_default();
        let mut fixed = Vec::new();
        let mut named: HashMap<String, PyObject> = HashMap::new();
        
        // Apply type conversions using stored field specs
        for (i, spec) in self.field_specs.iter().enumerate() {
            let value_str = if let Some(ref norm_name) = self.normalized_names.get(i).and_then(|n| n.as_ref()) {
                self.named_captures.get(norm_name.as_str()).map(|s| s.as_str())
            } else {
                self.captures.get(i).and_then(|s| s.as_ref()).map(|s| s.as_str())
            };
            
            if let Some(value_str) = value_str {
                let converted = spec.convert_value(value_str, py, &custom_converters)?;
                
                if let Some(original_name) = self.field_names.get(i).and_then(|n| n.as_ref()) {
                    // Check if this is a dict-style field name (contains [])
                    if original_name.contains('[') {
                        // Parse the path and insert into nested dict structure
                        let path = crate::parser::parse_field_path(original_name);
                        crate::parser::matching::insert_nested_dict(&mut named, &path, converted, py)?;
                    } else {
                        // Regular flat field name
                        // Check for repeated field names - values must match
                        if let Some(existing_value) = named.get(original_name.as_str()) {
                            let existing_obj = existing_value.to_object(py);
                            let converted_obj = converted.to_object(py);
                            let are_equal: bool = existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false);
                            if !are_equal {
                                return Err(error::repeated_name_error(original_name));
                            }
                        }
                        named.insert(original_name.clone(), converted);
                    }
                } else {
                    fixed.push(converted);
                }
            }
        }
        
        let parse_result = ParseResult::new_with_spans(fixed, named, self.span, self.field_spans.clone());
        Ok(Py::new(py, parse_result)?.to_object(py))
    }
}

impl Match {
    pub fn new(
        pattern: String,
        field_specs: Vec<FieldSpec>,
        field_names: Vec<Option<String>>,
        normalized_names: Vec<Option<String>>,
        captures: Vec<Option<String>>,
        named_captures: HashMap<String, String>,
        span: (usize, usize),
        field_spans: HashMap<String, (usize, usize)>,
    ) -> Self {
        Self {
            pattern,
            field_specs,
            field_names,
            normalized_names,
            captures,
            named_captures,
            span,
            field_spans,
        }
    }
}

