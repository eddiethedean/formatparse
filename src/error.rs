use pyo3::prelude::*;

/// Error types for formatparse operations
pub mod errors {
    use super::*;

    /// Create a pattern parsing error
    pub fn pattern_error(msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Pattern error: {}", msg))
    }

    /// Create a regex compilation error
    pub fn regex_error(msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", msg))
    }

    /// Create a type conversion error
    pub fn conversion_error(value: &str, target_type: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid {}: {}", target_type, value)
        )
    }

    /// Create a repeated name error with type mismatch
    pub fn repeated_name_error(name: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Repeated name '{}' with mismatched types", name)
        )
    }

    /// Create a custom type validation error
    pub fn custom_type_error(type_name: &str, msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Custom type '{}' error: {}", type_name, msg)
        )
    }

    /// Create an index error for regex group access
    pub fn regex_group_index_error(type_name: &str, actual: usize, expected: i64) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyIndexError, _>(
            format!(
                "Custom type '{}' pattern has {} capturing groups but regex_group_count is {}",
                type_name, actual, expected
            )
        )
    }

    /// Create a not implemented error
    pub fn not_implemented_error(feature: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            format!("{} is not supported", feature)
        )
    }

    /// Create a missing field error
    pub fn missing_field_error(field: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Missing required field: {}", field)
        )
    }

    /// Create a validation error (used to signal invalid alignment+precision combinations)
    /// This error should be caught and converted to None in matching code
    pub fn validation_error(msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(msg.to_string())
    }
}

/// Re-export error functions for convenience
pub use errors::*;

