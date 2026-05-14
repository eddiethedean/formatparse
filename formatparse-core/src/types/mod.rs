//! Type definitions for field specifications

pub mod definitions;
pub mod regex;

pub use definitions::{FieldSpec, FieldType};
pub use regex::strftime_to_regex;
