//! Regex generation from field specifications.

mod custom;
mod datetime_fragment;
mod helpers;
mod numeric;
mod string;
#[cfg(test)]
mod tests;

pub use helpers::strftime_to_regex;

use crate::types::definitions::{FieldSpec, FieldType};
use std::collections::HashMap;

impl FieldSpec {
    /// `allow_empty_delimited`: for default unconstrained string fields only (no width,
    /// precision, alignment), use `.*?` instead of `.+?` so an empty capture is allowed when
    /// the field is delimited by pattern literals (formatparse#83 / parse#136 remainder).
    pub fn to_regex_pattern(
        &self,
        custom_patterns: &HashMap<String, String>,
        next_field_is_greedy: Option<bool>,
        allow_empty_delimited: bool,
    ) -> String {
        match &self.field_type {
            FieldType::String | FieldType::Multiline | FieldType::IndentBlock => {
                string::pattern(self, next_field_is_greedy, allow_empty_delimited)
            }
            FieldType::Integer
            | FieldType::Float
            | FieldType::Letters
            | FieldType::Word
            | FieldType::NonLetters
            | FieldType::NonWhitespace
            | FieldType::NonDigits
            | FieldType::NumberWithThousands
            | FieldType::Scientific
            | FieldType::GeneralNumber
            | FieldType::Percentage => numeric::pattern(self),
            FieldType::DateTimeISO
            | FieldType::DateTimeRFC2822
            | FieldType::DateTimeGlobal
            | FieldType::DateTimeUS
            | FieldType::DateTimeCtime
            | FieldType::DateTimeHTTP
            | FieldType::DateTimeTime
            | FieldType::DateTimeSystem
            | FieldType::DateTimeStrftime => datetime_fragment::pattern(self),
            FieldType::BracedContent
            | FieldType::Nested
            | FieldType::Boolean
            | FieldType::Custom(_) => custom::pattern(self, custom_patterns),
        }
    }
}
