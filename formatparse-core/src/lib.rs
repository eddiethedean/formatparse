//! formatparse-core: Core Rust library for parsing strings using Python format() syntax
//!
//! This crate contains the pure Rust logic for pattern parsing, regex generation,
//! and type definitions. It has no dependencies on Python or PyO3.

pub mod error;
pub mod indent_block;
pub mod input_line_continuations;
pub mod parser;
pub mod types;

pub use indent_block::strip_common_indent;
pub use input_line_continuations::normalize_input_line_continuations;
pub use parser::{
    count_capturing_groups, validate_field_name, validate_input_length, validate_pattern_length,
    MAX_FIELDS, MAX_FIELD_NAME_LENGTH, MAX_INPUT_LENGTH, MAX_PATTERN_LENGTH,
};
// pub mod datetime;  // TODO: Extract pure Rust datetime utilities

pub use parser::regex::*;
pub use types::regex::strftime_to_regex;
pub use types::{FieldSpec, FieldType};
