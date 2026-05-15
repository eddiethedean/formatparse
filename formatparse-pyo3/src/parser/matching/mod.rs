use crate::error;
use crate::match_rs::{Match, MatchInit};
use crate::parser::format_parser::FormatParser;
use crate::parser::raw_match::{RawMatchData, RawValue};
use crate::result::ParseResult;
use fancy_regex::{Captures, Regex};
use formatparse_core::{count_capturing_groups, FieldSpec, FieldType};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;
use std::sync::Arc;

/// String stored on `Match` when `evaluate_result` is false: fold input continuations for `:ml` / `:blk`.
pub(crate) fn capture_string_for_match_storage(spec: &FieldSpec, raw: &str) -> String {
    if matches!(
        spec.field_type,
        FieldType::Multiline | FieldType::IndentBlock
    ) {
        formatparse_core::normalize_input_line_continuations(raw)
    } else {
        raw.to_string()
    }
}

/// Precomputed field metadata slices passed into capture-based match helpers.
pub struct FieldCaptureSlices<'a> {
    pub field_specs: &'a [FieldSpec],
    pub field_names: &'a [Option<String>],
    pub normalized_names: &'a [Option<String>],
    pub custom_type_groups: &'a [usize],
    pub has_nested_dict_fields: &'a [bool],
    pub nested_parsers: &'a [Option<Arc<FormatParser>>],
}

/// Pattern, field layout, and Python conversion context for [`match_with_captures`].
pub struct CapturedMatchContext<'a> {
    pub pattern: &'a str,
    pub fields: FieldCaptureSlices<'a>,
    pub py: Python<'a>,
    pub custom_converters: &'a HashMap<String, PyObject>,
    pub evaluate_result: bool,
}

/// Inputs to [`match_with_regex`] beyond the compiled `Regex`.
pub struct RegexMatchContext<'a> {
    pub string: &'a str,
    pub pattern: &'a str,
    pub field_specs: &'a [FieldSpec],
    pub field_names: &'a [Option<String>],
    pub normalized_names: &'a [Option<String>],
    pub nested_parsers: &'a [Option<Arc<FormatParser>>],
    pub py: Python<'a>,
    pub custom_converters: &'a HashMap<String, PyObject>,
    pub evaluate_result: bool,
}


mod capture;
mod custom_type;
mod nested_dict;
mod py_match;
mod raw;

pub use capture::extract_capture;
pub use custom_type::validate_custom_type_pattern;
pub use nested_dict::{get_nested_dict_value, insert_nested_dict};
pub use py_match::{match_empty_default_string_parse, match_with_captures, match_with_regex};
pub use raw::match_with_captures_raw;
