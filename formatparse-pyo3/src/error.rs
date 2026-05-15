use formatparse_core::error::FormatParseError;
use pyo3::prelude::*;

/// Convert a formatparse-core error to a PyO3 error.
///
/// Error types for formatparse PyO3 bindings; bridges formatparse-core errors to PyO3.
pub fn core_error_to_py_err(err: FormatParseError) -> PyErr {
    match err {
        FormatParseError::PatternError(msg) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Pattern error: {}", msg))
        }
        FormatParseError::RegexError(msg) => errors::regex_error(msg.as_str()),
        FormatParseError::ConversionError(value, target_type) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid {}: {}",
                target_type, value
            ))
        }
        FormatParseError::RepeatedNameError(name) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Repeated name '{}' with mismatched types",
                name
            ))
        }
        FormatParseError::CustomTypeError(type_name, msg) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Custom type '{}' error: {}",
                type_name, msg
            ))
        }
        FormatParseError::RegexGroupIndexError(type_name, actual, expected) => {
            PyErr::new::<pyo3::exceptions::PyIndexError, _>(format!(
                "Custom type '{}' pattern has {} capturing groups but regex_group_count is {}",
                type_name, actual, expected
            ))
        }
        FormatParseError::NotImplementedError(feature) => {
            PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(format!(
                "{} is not supported",
                feature
            ))
        }
        FormatParseError::MissingFieldError(field) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Missing required field: {}",
                field
            ))
        }
    }
}

/// Match-time error from the fancy-regex engine (used for compiled format patterns).
pub fn fancy_regex_match_error(e: fancy_regex::Error) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Regex match error: {}", e))
}

// Subclass of ValueError: malformed patterns where reference `parse` returns None from
// `parse()`. `compile()` still raises this type. Used by `parse` / `parse_batch` without string matching.
pyo3::create_exception!(
    _formatparse,
    PatternParseMismatch,
    pyo3::exceptions::PyValueError
);

/// Error module for formatparse PyO3 operations
pub mod errors {
    use super::*;

    /// Create a pattern parsing error
    pub fn pattern_error(msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Pattern error: {}", msg))
    }

    /// Malformed pattern: ``parse()`` / ``parse_batch()`` return no match (``None`` / list of ``None``).
    pub fn pattern_error_parse_mismatch(msg: &str) -> PyErr {
        super::PatternParseMismatch::new_err(format!("Pattern error: {}", msg))
    }

    /// Create a regex compilation error
    pub fn regex_error(msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", msg))
    }

    /// Create a type conversion error
    pub fn conversion_error(value: &str, target_type: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid {}: {}",
            target_type, value
        ))
    }

    /// Create a repeated name error with type mismatch
    pub fn repeated_name_error(name: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Repeated name '{}' with mismatched types",
            name
        ))
    }

    /// Create a custom type validation error
    pub fn custom_type_error(type_name: &str, msg: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Custom type '{}' error: {}",
            type_name, msg
        ))
    }

    /// Create an index error for regex group access
    pub fn regex_group_index_error(type_name: &str, actual: usize, expected: i64) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyIndexError, _>(format!(
            "Custom type '{}' pattern has {} capturing groups but regex_group_count is {}",
            type_name, actual, expected
        ))
    }

    /// Create a not implemented error
    pub fn not_implemented_error(feature: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(format!(
            "{} is not supported",
            feature
        ))
    }

    /// Create a missing field error
    pub fn missing_field_error(field: &str) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Missing required field: {}",
            field
        ))
    }
}

/// Re-export error functions for convenience
pub use errors::*;
