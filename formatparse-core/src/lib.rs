//! formatparse-core: Core Rust library for parsing strings using Python format() syntax
//!
//! This crate contains the pure Rust logic for pattern parsing, regex generation,
//! and type definitions. It has no dependencies on Python or PyO3.

pub mod error;
pub mod types;
pub mod parser;

pub use parser::{
    validate_pattern_length, validate_input_length, validate_field_name,
    MAX_PATTERN_LENGTH, MAX_INPUT_LENGTH, MAX_FIELDS, MAX_FIELD_NAME_LENGTH,
};
// pub mod datetime;  // TODO: Extract pure Rust datetime utilities

pub use types::{FieldType, FieldSpec};
pub use types::regex::strftime_to_regex;
pub use parser::regex::*;

