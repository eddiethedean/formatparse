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

use super::custom_type::validate_custom_type_pattern;

pub fn extract_capture<'a>(
    captures: &'a Captures<'a>,
    field_index: usize,
    normalized_names: &'a [Option<String>],
    field_spec: &'a FieldSpec,
    actual_capture_index: usize,
    group_offset: usize,
) -> Option<fancy_regex::Match<'a>> {
    // Fast path: check if this is a named group first (most common case)
    if let Some(Some(norm_name)) = normalized_names.get(field_index) {
        // Use normalized name to get the capture (direct lookup)
        captures.name(norm_name)
    } else {
        // Unnamed group - use index directly
        let capture_group_index = actual_capture_index + group_offset;
        if field_spec.alignment.is_some() {
            // For alignment patterns, try innermost group first, then outer
            captures
                .get(capture_group_index + 1)
                .or_else(|| captures.get(capture_group_index))
        } else {
            captures.get(capture_group_index)
        }
    }
}

/// For each field index, `(actual_capture_index, group_offset)` at the start of
/// processing that field, matching the main match loops' bookkeeping.
pub(crate) fn per_field_capture_geometry(
    field_specs: &[FieldSpec],
    _normalized_names: &[Option<String>],
    py: Python<'_>,
    custom_converters: &HashMap<String, PyObject>,
    precomputed_pattern_groups: Option<&[usize]>,
) -> PyResult<Vec<(usize, usize)>> {
    let mut aci = 1usize;
    let mut go = 0usize;
    let mut out = Vec::with_capacity(field_specs.len());
    for (i, spec) in field_specs.iter().enumerate() {
        let pattern_groups = if let Some(pc) = precomputed_pattern_groups {
            pc.get(i).copied().unwrap_or(0)
        } else if matches!(spec.field_type, FieldType::Nested) {
            spec.nested_regex_body
                .as_ref()
                .map(|b| count_capturing_groups(b))
                .unwrap_or(0)
        } else if !custom_converters.is_empty() {
            validate_custom_type_pattern(spec, custom_converters, py)?
        } else {
            0
        };
        out.push((aci, go));
        aci += 1;
        if spec.alignment.is_some() {
            go += 1;
        }
        if pattern_groups > 0 {
            go += pattern_groups;
        }
    }
    Ok(out)
}

/// `entry[i] == Some(L)` when field `i` merges with others under the same flat name `L`
/// (smallest index in the group). Issue #4 / parse#197.
pub(crate) fn strftime_merge_leader_per_field(
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
) -> Vec<Option<usize>> {
    let n = field_specs.len();
    let mut out = vec![None; n];
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, spec) in field_specs.iter().enumerate() {
        if !matches!(spec.field_type, FieldType::DateTimeStrftime) {
            continue;
        }
        if spec.strftime_format.is_none() {
            continue;
        }
        let Some(name) = field_names.get(i).and_then(|x| x.as_ref()) else {
            continue;
        };
        if name.contains('[') {
            continue;
        }
        by_name.entry(name.clone()).or_default().push(i);
    }
    for indices in by_name.into_values() {
        if indices.len() < 2 {
            continue;
        }
        let leader = *indices.iter().min().unwrap();
        for idx in indices {
            out[idx] = Some(leader);
        }
    }
    out
}

