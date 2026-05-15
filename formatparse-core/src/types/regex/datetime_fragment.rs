//! Regex patterns for datetime fragment field types.

use super::helpers::strftime_to_regex;
use crate::types::definitions::{FieldSpec, FieldType};

pub(crate) fn pattern(spec: &FieldSpec) -> String {
    match &spec.field_type {
        FieldType::DateTimeISO => {
            // ISO 8601 format: YYYY-MM-DD, YYYY-MM-DDTHH:MM, YYYY-MM-DDTHH:MM:SS, etc.
            // Supports various separators and timezone formats (with optional space before timezone)
            r"\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?)?(?:\s*[Zz]|\s*[+-]\d{2}:?\d{2}|\s*[+-]\d{4})?".to_string()
        }
        FieldType::DateTimeRFC2822 => {
            // RFC2822: Mon, 21 Nov 2011 10:21:36 +1000 or +10:00 (optional weekday)
            r"(?:(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+)?\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{4}\s+\d{2}:\d{2}:\d{2}\s+[+-]\d{2}:?\d{2,4}".to_string()
        }
        FieldType::DateTimeGlobal => {
            // Global format: 21/11/2011 10:21:36 AM +1000 or 21-Nov-2011 10:21:36 AM +1:00
            r"\d{1,2}[-/](?:\d{1,2}|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|June|July|August|September|October|November|December)[-/]\d{4}(?:\s+\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{1,2}:?\d{2,4})?)?".to_string()
        }
        FieldType::DateTimeUS => {
            // US format: 11/21/2011 10:21:36 AM +1000 or 11-Nov-2011 10:21:36 AM +1000 or Nov-21-2011 10:21:36 AM +1000
            r"(?:\d{1,2}|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|June|July|August|September|October|November|December)[-/]\d{1,2}[-/]\d{4}(?:\s+\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{2}:?\d{2,4})?)?".to_string()
        }
        FieldType::DateTimeCtime => {
            // ctime format: Mon Nov 21 10:21:36 2011
            r"(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\d{4}".to_string()
        }
        FieldType::DateTimeHTTP => {
            // HTTP log format: 21/Nov/2011:00:07:11 +0000
            r"\d{2}/[A-Za-z]{3}/\d{4}:\d{2}:\d{2}:\d{2}\s+[+-]\d{2}:?\d{2,4}".to_string()
        }
        FieldType::DateTimeTime => {
            // Time format: 10:21:36 PM -5:30
            r"\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{1,2}:?\d{2,4})?".to_string()
        }
        FieldType::DateTimeSystem => {
            // Linux system log format: Nov 21 10:21:36
            r"[A-Za-z]{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}".to_string()
        }
        FieldType::DateTimeStrftime => {
            // Convert strftime format to regex pattern
            if let Some(ref fmt) = spec.strftime_format {
                strftime_to_regex(fmt)
            } else {
                r".+?".to_string()
            }
        }
        _ => unreachable!("pattern() called with wrong field type"),
    }
}
