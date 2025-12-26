//! Type system module for structparse
//!
//! This module defines the type system used for parsing:
//! - `definitions`: Core type definitions (FieldType, FieldSpec)
//! - `regex`: Regex pattern generation for field types
//! - `conversion`: Value conversion from strings to Python objects

pub mod definitions;
pub mod regex;
pub mod conversion;

// Re-export public types and functions
pub use definitions::{FieldType, FieldSpec};
pub use regex::strftime_to_regex;

