//! Parser module for structparse
//!
//! This module contains the core parsing logic, organized into sub-modules:
//! - `pattern`: Parses format strings into field specifications
//! - `regex`: Builds regex patterns from field specifications
//! - `matching`: Executes regex matches and extracts values
//! - `format_parser`: Main FormatParser struct and Format class

pub mod pattern;
pub mod regex;
pub mod matching;
pub mod format_parser;

pub use format_parser::{FormatParser, Format};
pub use pattern::parse_field_path;

