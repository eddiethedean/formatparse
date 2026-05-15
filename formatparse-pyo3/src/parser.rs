//! Parser module for formatparse
//!
//! This module contains the core parsing logic, organized into sub-modules:
//! - `pattern`: Parses format strings into field specifications
//! - `regex`: Builds regex patterns from field specifications
//! - `matching`: Executes regex matches and extracts values
//! - `format_parser`: Main FormatParser struct and Format class

pub mod pattern;
// regex module is in formatparse-core
pub mod findall_engine;
pub mod findall_iter;
pub mod format_parser;
pub mod format_parser_pymethods;
pub mod matching;
pub mod raw_match;

pub use findall_iter::FindallIter;
pub use format_parser::FormatParser;
pub use format_parser_pymethods::Format;
pub use pattern::parse_field_path;
