//! Regex patterns for custom field types.

use crate::types::definitions::{FieldSpec, FieldType};
use std::collections::HashMap;

pub(crate) fn pattern(spec: &FieldSpec, custom_patterns: &HashMap<String, String>) -> String {
    match &spec.field_type {
        FieldType::BracedContent => {
            // Placeholder: named patterns use special regex in the PyO3 pattern compiler.
            r".*?".to_string()
        }
        FieldType::Nested => spec
            .nested_regex_body
            .as_deref()
            .unwrap_or(r"[\s\S]*?")
            .to_string(),
        FieldType::Boolean => {
            "true|false|True|False|TRUE|FALSE|1|0|yes|no|Yes|No|YES|NO|on|off|On|Off|ON|OFF"
                .to_string()
        }
        FieldType::Custom(name) => {
            custom_patterns
                .get(name)
                .cloned()
                .unwrap_or_else(|| r"\S+".to_string()) // Default to non-whitespace for custom types without patterns
        }
        _ => unreachable!("pattern() called with wrong field type"),
    }
}
