//! Pattern parsing — implementation in [`formatparse_core::parser::pattern`].

#[allow(unused_imports)]
pub use formatparse_core::parser::pattern::{
    field_types_match, parse_field_path, parse_format_spec, parse_pattern,
    validate_multiline_mvp, ParsedPatternParts, MAX_NESTED_FORMAT_DEPTH,
};

use formatparse_core::error::FormatParseError;
use pyo3::prelude::*;

/// Map core pattern-compile errors to Python exceptions (``PatternParseMismatch`` for narrow cases).
pub fn pattern_compile_error_to_py(err: FormatParseError) -> PyErr {
    match err {
        FormatParseError::PatternError(ref msg)
            if msg.contains("Unclosed '{' in pattern")
                || msg.contains("Expected '}' after field specification") =>
        {
            crate::error::PatternParseMismatch::new_err(format!("Pattern error: {}", msg))
        }
        other => crate::error::core_error_to_py_err(other),
    }
}

#[cfg(test)]
mod nested_candidate_tests {
    use super::*;
    use formatparse_core::FieldType;
    use std::collections::HashMap;

    #[test]
    fn parse_nested_outer_field_type() {
        let r = parse_pattern("{outer:{inner:d}}", &HashMap::new(), true, 0)
            .expect("parse_pattern");
        let specs = &r.2;
        assert_eq!(specs.len(), 1);
        assert!(matches!(specs[0].field_type, FieldType::Nested));
    }
}
