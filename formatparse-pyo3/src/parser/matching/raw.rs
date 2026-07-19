use crate::parser::raw_match::{RawMatchData, RawValue};
use fancy_regex::Captures;
use formatparse_core::FieldType;

use super::capture::extract_capture;
use super::FieldCaptureSlices;

pub fn match_with_captures_raw(
    captures: &Captures,
    _string: &str,
    _match_start: usize,
    fields: &FieldCaptureSlices<'_>,
) -> Result<Option<RawMatchData>, String> {
    let field_specs = fields.field_specs;
    let field_names = fields.field_names;
    let normalized_names = fields.normalized_names;
    let custom_type_groups = fields.custom_type_groups;
    let has_nested_dict_fields = fields.has_nested_dict_fields;

    let full_match = captures
        .get(0)
        .ok_or_else(|| "regex match missing capture group 0".to_string())?;
    let start = full_match.start();
    let end = full_match.end();

    let field_count = field_specs.len();
    let mut raw_data = RawMatchData::with_capacity(field_count);
    raw_data.span = (start, end);

    let mut group_offset = 0;

    for (i, spec) in field_specs.iter().enumerate() {
        let pattern_groups = custom_type_groups.get(i).copied().unwrap_or(0);

        if matches!(spec.field_type, FieldType::Nested) {
            return Err("Nested format fields require Python conversion".to_string());
        }

        // Capture group 0 is the full match, so field capture indices start at 1.
        let cap = extract_capture(captures, i, normalized_names, spec, i + 1, group_offset);

        if let Some(cap) = cap {
            let value_str = cap.as_str();
            let field_start = cap.start();
            let field_end = cap.end();

            if !crate::types::conversion::validate_alignment_precision_for_capture(spec, value_str)
            {
                return Ok(None);
            }

            // Try to convert to raw value (fails for custom types and datetime)
            match crate::parser::raw_match::convert_value_raw(spec, value_str) {
                Ok(raw_value) => {
                    if let Some(ref original_name) = field_names[i] {
                        // Check for repeated field names
                        if has_nested_dict_fields.get(i).copied().unwrap_or(false) {
                            // Nested dict fields require Python conversion (complex dict structure)
                            return Err("Nested dict fields require Python conversion".to_string());
                        } else {
                            // Regular flat field name
                            if let Some(existing) = raw_data.named.get(original_name) {
                                // Check if values match (for repeated names)
                                if !values_equal(existing, &raw_value) {
                                    return Ok(None); // Values don't match
                                }
                            } else {
                                raw_data.named.insert(original_name.clone(), raw_value);
                            }
                        }
                        raw_data
                            .field_spans
                            .insert(original_name.clone(), (field_start, field_end));
                    } else {
                        raw_data.fixed.push(raw_value);
                    }
                }
                Err(_) => {
                    // Type requires Python conversion (custom or datetime)
                    return Err("Type requires Python conversion".to_string());
                }
            }
        }

        if spec.alignment.is_some() {
            group_offset += 1;
        }
        if pattern_groups > 0 {
            group_offset += pattern_groups;
        }
    }

    Ok(Some(raw_data))
}

/// Compare two RawValues for equality
pub(crate) fn values_equal(a: &RawValue, b: &RawValue) -> bool {
    match (a, b) {
        (RawValue::String(s1), RawValue::String(s2)) => s1 == s2,
        (RawValue::Integer(n1), RawValue::Integer(n2)) => n1 == n2,
        (RawValue::Float(f1), RawValue::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
        (RawValue::Boolean(b1), RawValue::Boolean(b2)) => b1 == b2,
        (RawValue::None, RawValue::None) => true,
        _ => false,
    }
}
