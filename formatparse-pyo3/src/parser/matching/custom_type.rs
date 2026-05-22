use crate::error;
use formatparse_core::{count_capturing_groups, FieldSpec, FieldType};
use pyo3::prelude::*;
use std::collections::HashMap;

pub fn validate_custom_type_pattern(
    field_spec: &FieldSpec,
    custom_converters: &HashMap<String, Py<PyAny>>,
    py: Python,
) -> PyResult<usize> {
    if matches!(&field_spec.field_type, FieldType::Nested) {
        return Ok(0);
    }
    let mut pattern_groups = 0;

    if let formatparse_core::FieldType::Custom(type_name) = &field_spec.field_type {
        if let Some(converter_obj) = custom_converters.get(type_name) {
            let converter_ref = converter_obj.bind(py);
            if let Ok(pattern_attr) = converter_ref.getattr("pattern") {
                if let Ok(pattern_str) = pattern_attr.extract::<String>() {
                    let actual_groups = count_capturing_groups(&pattern_str);
                    pattern_groups = actual_groups;

                    if let Ok(group_count_attr) = converter_ref.getattr("regex_group_count") {
                        // Try to extract as int first
                        if let Ok(group_count) = group_count_attr.extract::<i64>() {
                            if group_count < 0 {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("regex_group_count must be >= 0, got {}", group_count),
                                ));
                            }
                            if group_count == 0 && actual_groups > 0 {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is 0", type_name, actual_groups)
                                ));
                            }
                            if group_count > actual_groups as i64 {
                                return Err(error::regex_group_index_error(
                                    type_name,
                                    actual_groups,
                                    group_count,
                                ));
                            }
                        } else {
                            // regex_group_count is None
                            if actual_groups > 0 {
                                return Err(error::custom_type_error(
                                            type_name,
                                            &format!("pattern has {} capturing groups but regex_group_count is None", actual_groups)
                                        ));
                            }
                        }
                    } else {
                        // No regex_group_count attribute - must have 0 groups
                        if actual_groups > 0 {
                            return Err(error::custom_type_error(
                                            type_name,
                                            &format!("pattern has {} capturing groups but regex_group_count is not set", actual_groups)
                                        ));
                        }
                    }
                }
            }
        }
    }

    Ok(pattern_groups)
}
