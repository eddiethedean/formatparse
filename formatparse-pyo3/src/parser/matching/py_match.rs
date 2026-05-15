use crate::match_rs::{Match, MatchInit};
use crate::result::ParseResult;
use fancy_regex::{Captures, Regex};
use formatparse_core::{count_capturing_groups, FieldSpec, FieldType};
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

use super::custom_type::validate_custom_type_pattern;
use super::nested_dict::{get_nested_dict_value, insert_nested_dict};
use super::{
    capture::{extract_capture, per_field_capture_geometry, strftime_merge_leader_per_field},
    capture_string_for_match_storage, CapturedMatchContext, RegexMatchContext,
};

pub fn match_with_captures(
    captures: &Captures,
    ctx: &CapturedMatchContext<'_>,
) -> PyResult<Option<PyObject>> {
    let field_specs = ctx.fields.field_specs;
    let field_names = ctx.fields.field_names;
    let normalized_names = ctx.fields.normalized_names;
    let custom_type_groups = ctx.fields.custom_type_groups;
    let has_nested_dict_fields = ctx.fields.has_nested_dict_fields;
    let pattern = ctx.pattern;
    let py = ctx.py;
    let custom_converters = ctx.custom_converters;
    let evaluate_result = ctx.evaluate_result;

    let full_match = captures.get(0).ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err("regex match missing capture group 0")
    })?;
    let start = full_match.start(); // Already absolute position in full string
    let end = full_match.end(); // Already absolute position in full string

    let strftime_merge_leader = strftime_merge_leader_per_field(field_specs, field_names);
    let capture_geom = per_field_capture_geometry(
        field_specs,
        normalized_names,
        py,
        custom_converters,
        Some(custom_type_groups),
    )?;

    // Pre-allocate with capacity based on expected field count
    let field_count = field_specs.len();
    // Fast path: for single-field patterns, use optimized allocation
    let mut fixed = Vec::with_capacity(field_count);
    let mut named: HashMap<String, PyObject> = HashMap::with_capacity(field_count.max(1));
    let mut field_spans: HashMap<String, (usize, usize)> =
        HashMap::with_capacity(field_count.max(1));
    let mut captures_vec = Vec::with_capacity(field_count); // For Match object when evaluate_result=False
    let mut named_captures = HashMap::with_capacity(field_count); // For Match object when evaluate_result=False
    let mut group_offset = 0;
    // Track the actual capture group index (accounts for both named and unnamed groups)
    let mut actual_capture_index = 1; // Start at 1 (group 0 is full match)

    for (i, spec) in field_specs.iter().enumerate() {
        // Use pre-computed pattern_groups (cached during FormatParser creation)
        let pattern_groups = custom_type_groups.get(i).copied().unwrap_or(0);

        // Extract capture group
        let cap = extract_capture(
            captures,
            i,
            normalized_names,
            spec,
            actual_capture_index,
            group_offset,
        );

        // Increment actual_capture_index for the next field (both named and unnamed groups consume an index)
        // But only increment if we actually used a positional group (not a named group)
        if normalized_names.get(i).and_then(|n| n.as_ref()).is_none() {
            actual_capture_index += 1;
        } else {
            // Named groups still consume an index in the regex, so increment
            actual_capture_index += 1;
        }

        if let Some(cap) = cap {
            let value_str = cap.as_str();
            let field_start = cap.start();
            let field_end = cap.end();

            // Store raw capture for Match object (only if needed)
            // Only allocate strings when evaluate_result=False (Match objects need owned strings)
            if !evaluate_result {
                captures_vec.push(Some(capture_string_for_match_storage(spec, value_str)));
                if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                    named_captures.insert(
                        norm_name.clone(),
                        capture_string_for_match_storage(spec, value_str),
                    );
                }
            }
            // For evaluate_result=True, we don't need to store raw captures, saving allocations

            if evaluate_result {
                if !crate::types::conversion::validate_alignment_precision_for_capture(
                    spec, value_str,
                ) {
                    return Ok(None);
                }

                let merge_leader_i = strftime_merge_leader.get(i).copied().flatten();
                let is_merge_follower = merge_leader_i.is_some_and(|leader_idx| leader_idx != i);
                let is_merge_leader = merge_leader_i == Some(i);

                if is_merge_follower {
                    // Merge leader performs conversion and inserts `named` / `field_spans`.
                } else if is_merge_leader {
                    let mut parts: Vec<(String, String)> = Vec::new();
                    let mut span_lo = usize::MAX;
                    let mut span_hi = 0usize;
                    for (j, spec_j) in field_specs.iter().enumerate() {
                        if strftime_merge_leader.get(j).copied().flatten() != Some(i) {
                            continue;
                        }
                        let (aci, goj) = capture_geom[j];
                        let cap_j =
                            extract_capture(captures, j, normalized_names, spec_j, aci, goj);
                        let Some(cap_j) = cap_j else {
                            return Ok(None);
                        };
                        let vs = cap_j.as_str();
                        if !crate::types::conversion::validate_alignment_precision_for_capture(
                            spec_j, vs,
                        ) {
                            return Ok(None);
                        }
                        let Some(fmt) = spec_j.strftime_format.as_ref() else {
                            return Ok(None);
                        };
                        parts.push((fmt.clone(), vs.to_string()));
                        span_lo = span_lo.min(cap_j.start());
                        span_hi = span_hi.max(cap_j.end());
                    }
                    let converted = crate::datetime::parse_merged_strftime_datetime(py, &parts)?;
                    let field_start = span_lo;
                    let field_end = span_hi;

                    if let Some(ref original_name) = field_names[i] {
                        if has_nested_dict_fields.get(i).copied().unwrap_or(false) {
                            let path = crate::parser::pattern::parse_field_path(original_name);
                            if let Some(existing_value) = get_nested_dict_value(&named, &path, py)?
                            {
                                let are_equal: bool = {
                                    let existing_obj = existing_value.bind(py);
                                    let converted_obj = converted.bind(py);
                                    existing_obj.eq(converted_obj).unwrap_or(false)
                                };
                                if !are_equal {
                                    return Ok(None);
                                }
                            }
                            insert_nested_dict(&mut named, &path, converted, py)?;
                        } else {
                            match named.get(original_name) {
                                Some(existing_value) => {
                                    let are_equal: bool = {
                                        let existing_obj = existing_value.clone_ref(py);
                                        let converted_obj = converted.clone_ref(py);
                                        existing_obj
                                            .bind(py)
                                            .eq(converted_obj.bind(py))
                                            .unwrap_or(false)
                                    };
                                    if !are_equal {
                                        return Ok(None);
                                    }
                                }
                                None => {
                                    named.insert(original_name.clone(), converted);
                                }
                            }
                        }
                        field_spans.insert(original_name.clone(), (field_start, field_end));
                    } else {
                        fixed.push(converted);
                    }
                } else {
                    let converted: PyObject = if matches!(spec.field_type, FieldType::Nested) {
                        let nested_arc = ctx
                            .fields
                            .nested_parsers
                            .get(i)
                            .and_then(|x| x.as_ref())
                            .ok_or_else(|| {
                                pyo3::exceptions::PyValueError::new_err(
                                    "internal error: nested parser missing",
                                )
                            })?;
                        match nested_arc.parse_nested_capture(py, value_str, custom_converters)? {
                            Some(pr) => pr.into_py_any(py)?,
                            None => return Ok(None),
                        }
                    } else {
                        crate::types::conversion::convert_value(
                            spec,
                            value_str,
                            py,
                            custom_converters,
                        )?
                    };

                    // Use original field name (with hyphens/dots) for the result
                    if let Some(ref original_name) = field_names[i] {
                        // Use pre-computed flag to avoid contains('[') check in hot path
                        if has_nested_dict_fields.get(i).copied().unwrap_or(false) {
                            // Parse the path and insert into nested dict structure
                            let path = crate::parser::pattern::parse_field_path(original_name);
                            // Check for repeated field names - compare values if path already exists
                            if let Some(existing_value) = get_nested_dict_value(&named, &path, py)?
                            {
                                // Compare values using Python's equality (batch GIL operation)
                                let are_equal: bool = {
                                    let existing_obj = existing_value.bind(py);
                                    let converted_obj = converted.bind(py);
                                    existing_obj.eq(converted_obj).unwrap_or(false)
                                };
                                if !are_equal {
                                    // Values don't match for repeated name
                                    return Ok(None);
                                }
                            }
                            insert_nested_dict(&mut named, &path, converted, py)?;
                        } else {
                            // Regular flat field name
                            // Fast path: most fields are not repeated, so check first
                            // Use get() directly instead of contains_key + get (one less lookup)
                            match named.get(original_name) {
                                Some(existing_value) => {
                                    // Field exists - check if values match (repeated name case)
                                    let are_equal: bool = {
                                        let existing_obj = existing_value.clone_ref(py);
                                        let converted_obj = converted.clone_ref(py);
                                        existing_obj
                                            .bind(py)
                                            .eq(converted_obj.bind(py))
                                            .unwrap_or(false)
                                    };
                                    if !are_equal {
                                        // Values don't match for repeated name
                                        return Ok(None);
                                    }
                                }
                                None => {
                                    // New field - insert it
                                    named.insert(original_name.clone(), converted);
                                }
                            }
                        }

                        // Store field span (already absolute position in original string)
                        field_spans.insert(original_name.clone(), (field_start, field_end));
                    } else {
                        // Positional field
                        fixed.push(converted);
                    }
                }
            }
        }

        // Increment group offset for alignment patterns (they add an extra group)
        if spec.alignment.is_some() {
            group_offset += 1;
        }
        // Increment group offset for custom patterns with groups (the groups inside the pattern become part of the overall regex)
        if pattern_groups > 0 {
            group_offset += pattern_groups;
        }
    }

    // Create result object (positions are already absolute)
    if evaluate_result {
        let parse_result = ParseResult::new_with_spans(fixed, named, (start, end), field_spans);
        // Py::new() is already optimized when GIL is held
        Ok(Some(Py::new(py, parse_result)?.into_py_any(py)?))
    } else {
        // Create Match object with raw captures
        // Note: pattern is static, but Match needs owned String - this is acceptable
        // as Match objects are only created when evaluate_result=False (less common)
        let match_obj = Match::new(MatchInit {
            pattern: pattern.to_string(),
            field_specs: field_specs.to_vec(),
            field_names: field_names.to_vec(),
            normalized_names: normalized_names.to_vec(),
            captures: captures_vec,
            named_captures,
            span: (start, end),
            field_spans,
        });
        Ok(Some(Py::new(py, match_obj)?.into_py_any(py)?))
    }
}

/// Match a regex against a string and extract results
pub fn match_with_regex(regex: &Regex, ctx: &RegexMatchContext<'_>) -> PyResult<Option<PyObject>> {
    let string = ctx.string;
    let pattern = ctx.pattern;
    let field_specs = ctx.field_specs;
    let field_names = ctx.field_names;
    let normalized_names = ctx.normalized_names;
    let py = ctx.py;
    let custom_converters = ctx.custom_converters;
    let evaluate_result = ctx.evaluate_result;
    let nested_parsers = ctx.nested_parsers;

    let captures = regex
        .captures(string)
        .map_err(crate::error::fancy_regex_match_error)?;
    if let Some(captures) = captures {
        // Pre-allocate with capacity based on expected field count
        let field_count = field_specs.len();
        let mut fixed = Vec::with_capacity(field_count);
        let mut named: HashMap<String, PyObject> = HashMap::with_capacity(field_count);
        let mut field_spans: HashMap<String, (usize, usize)> = HashMap::with_capacity(field_count);
        let mut captures_vec = Vec::with_capacity(field_count); // For Match object when evaluate_result=False
        let mut named_captures = HashMap::with_capacity(field_count); // For Match object when evaluate_result=False

        let full_match = captures.get(0).ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("regex match missing capture group 0")
        })?;
        let start = full_match.start();
        let end = full_match.end();

        let strftime_merge_leader = strftime_merge_leader_per_field(field_specs, field_names);
        let capture_geom =
            per_field_capture_geometry(field_specs, normalized_names, py, custom_converters, None)?;

        let mut fixed_index = 0;
        let mut group_offset = 0;
        // Track the actual capture group index (accounts for both named and unnamed groups)
        let mut actual_capture_index = 1; // Start at 1 (group 0 is full match)

        for (i, spec) in field_specs.iter().enumerate() {
            // Capturing groups inside this field's regex fragment (custom converters or nested).
            let pattern_groups = if matches!(spec.field_type, FieldType::Nested) {
                spec.nested_regex_body
                    .as_ref()
                    .map(|b| count_capturing_groups(b))
                    .unwrap_or(0)
            } else if !custom_converters.is_empty() {
                validate_custom_type_pattern(spec, custom_converters, py)?
            } else {
                0
            };

            // Extract capture group
            let cap = extract_capture(
                &captures,
                i,
                normalized_names,
                spec,
                actual_capture_index,
                group_offset,
            );

            // Increment actual_capture_index for the next field (both named and unnamed groups consume an index)
            // But only increment if we actually used a positional group (not a named group)
            if normalized_names.get(i).and_then(|n| n.as_ref()).is_none() {
                actual_capture_index += 1;
            } else {
                // Named groups still consume an index in the regex, so increment
                actual_capture_index += 1;
            }

            if let Some(cap) = cap {
                let value_str = cap.as_str();
                let field_start = cap.start();
                let field_end = cap.end();

                // Store capture for Match object (only if needed); fold input continuations for :ml/:blk
                if !evaluate_result {
                    captures_vec.push(Some(capture_string_for_match_storage(spec, value_str)));
                    if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                        named_captures.insert(
                            norm_name.clone(),
                            capture_string_for_match_storage(spec, value_str),
                        );
                    }
                }

                if evaluate_result {
                    if !crate::types::conversion::validate_alignment_precision_for_capture(
                        spec, value_str,
                    ) {
                        return Ok(None);
                    }

                    let merge_leader_i = strftime_merge_leader.get(i).copied().flatten();
                    let is_merge_follower =
                        merge_leader_i.is_some_and(|leader_idx| leader_idx != i);
                    let is_merge_leader = merge_leader_i == Some(i);

                    if is_merge_follower {
                        // Typed conversion happens on the merge leader only.
                    } else if is_merge_leader {
                        let mut parts: Vec<(String, String)> = Vec::new();
                        let mut span_lo = usize::MAX;
                        let mut span_hi = 0usize;
                        for (j, spec_j) in field_specs.iter().enumerate() {
                            if strftime_merge_leader.get(j).copied().flatten() != Some(i) {
                                continue;
                            }
                            let (aci, goj) = capture_geom[j];
                            let cap_j =
                                extract_capture(&captures, j, normalized_names, spec_j, aci, goj);
                            let Some(cap_j) = cap_j else {
                                return Ok(None);
                            };
                            let vs = cap_j.as_str();
                            if !crate::types::conversion::validate_alignment_precision_for_capture(
                                spec_j, vs,
                            ) {
                                return Ok(None);
                            }
                            let Some(fmt) = spec_j.strftime_format.as_ref() else {
                                return Ok(None);
                            };
                            parts.push((fmt.clone(), vs.to_string()));
                            span_lo = span_lo.min(cap_j.start());
                            span_hi = span_hi.max(cap_j.end());
                        }
                        let converted =
                            crate::datetime::parse_merged_strftime_datetime(py, &parts)?;
                        let field_start = span_lo;
                        let field_end = span_hi;

                        if let Some(ref original_name) = field_names[i] {
                            if original_name.contains('[') {
                                let path = crate::parser::pattern::parse_field_path(original_name);
                                if let Some(existing_value) =
                                    get_nested_dict_value(&named, &path, py)?
                                {
                                    let are_equal: bool = {
                                        let existing_obj = existing_value.bind(py);
                                        let converted_obj = converted.bind(py);
                                        existing_obj.eq(converted_obj).unwrap_or(false)
                                    };
                                    if !are_equal {
                                        return Ok(None);
                                    }
                                }
                                insert_nested_dict(&mut named, &path, converted, py)?;
                            } else {
                                match named.get(original_name) {
                                    Some(existing_value) => {
                                        let are_equal: bool = {
                                            let existing_obj = existing_value.clone_ref(py);
                                            let converted_obj = converted.clone_ref(py);
                                            existing_obj
                                                .bind(py)
                                                .eq(converted_obj.bind(py))
                                                .unwrap_or(false)
                                        };
                                        if !are_equal {
                                            return Ok(None);
                                        }
                                        field_spans.insert(
                                            original_name.clone(),
                                            (field_start, field_end),
                                        );
                                    }
                                    None => {
                                        let name_for_named = original_name.clone();
                                        named.insert(name_for_named.clone(), converted);
                                        field_spans
                                            .insert(name_for_named, (field_start, field_end));
                                    }
                                }
                            }
                        } else {
                            fixed.push(converted);
                            field_spans.insert(fixed_index.to_string(), (field_start, field_end));
                            fixed_index += 1;
                        }
                    } else {
                        let converted: PyObject = if matches!(spec.field_type, FieldType::Nested) {
                            let nested_arc = nested_parsers
                                .get(i)
                                .and_then(|x| x.as_ref())
                                .ok_or_else(|| {
                                    pyo3::exceptions::PyValueError::new_err(
                                        "internal error: nested parser missing",
                                    )
                                })?;
                            match nested_arc.parse_nested_capture(
                                py,
                                value_str,
                                custom_converters,
                            )? {
                                Some(pr) => pr.into_py_any(py)?,
                                None => return Ok(None),
                            }
                        } else {
                            crate::types::conversion::convert_value(
                                spec,
                                value_str,
                                py,
                                custom_converters,
                            )?
                        };

                        // Use original field name (with hyphens/dots) for the result
                        if let Some(ref original_name) = field_names[i] {
                            // Check if this is a dict-style field name (contains [])
                            if original_name.contains('[') {
                                // Parse the path and insert into nested dict structure
                                let path = crate::parser::pattern::parse_field_path(original_name);
                                // Check for repeated field names - compare values if path already exists
                                if let Some(existing_value) =
                                    get_nested_dict_value(&named, &path, py)?
                                {
                                    // Compare values using Python's equality (batch GIL operation)
                                    let are_equal: bool = {
                                        let existing_obj = existing_value.bind(py);
                                        let converted_obj = converted.bind(py);
                                        existing_obj.eq(converted_obj).unwrap_or(false)
                                    };
                                    if !are_equal {
                                        // Values don't match for repeated name
                                        return Ok(None);
                                    }
                                }
                                insert_nested_dict(&mut named, &path, converted, py)?;
                            } else {
                                // Regular flat field name
                                // Fast path: most fields are not repeated, so check first
                                // Use get() directly instead of contains_key + get (one less lookup)
                                match named.get(original_name) {
                                    Some(existing_value) => {
                                        // Field exists - check if values match (repeated name case)
                                        // Compare values using Python's equality (batch GIL operation)
                                        let are_equal: bool = {
                                            let existing_obj = existing_value.clone_ref(py);
                                            let converted_obj = converted.clone_ref(py);
                                            existing_obj
                                                .bind(py)
                                                .eq(converted_obj.bind(py))
                                                .unwrap_or(false)
                                        };
                                        if !are_equal {
                                            // Values don't match for repeated name
                                            return Ok(None);
                                        }
                                        // Store span for repeated name
                                        field_spans.insert(
                                            original_name.clone(),
                                            (field_start, field_end),
                                        );
                                    }
                                    None => {
                                        // First occurrence - just insert (common case)
                                        // Reuse the clone for both insertions
                                        let name_for_named = original_name.clone();
                                        named.insert(name_for_named.clone(), converted);
                                        field_spans
                                            .insert(name_for_named, (field_start, field_end));
                                    }
                                }
                            }
                        } else {
                            fixed.push(converted);
                            // Store span by fixed index (only if needed - most cases don't need spans)
                            // Use format! only when necessary to avoid allocation
                            field_spans.insert(fixed_index.to_string(), (field_start, field_end));
                            fixed_index += 1;
                        }
                    }
                } else {
                    // Store span even when not evaluating
                    if let Some(ref original_name) = field_names[i] {
                        field_spans.insert(original_name.clone(), (field_start, field_end));
                    } else {
                        let index_str = fixed_index.to_string();
                        field_spans.insert(index_str, (field_start, field_end));
                        fixed_index += 1;
                    }
                }
            } else {
                captures_vec.push(None);
            }

            // Increment group offset for alignment patterns (they add an extra group)
            if spec.alignment.is_some() {
                group_offset += 1;
            }
            // Increment group offset for custom patterns with groups (the groups inside the pattern become part of the overall regex)
            if pattern_groups > 0 {
                group_offset += pattern_groups;
            }
        }

        if evaluate_result {
            let parse_result = ParseResult::new_with_spans(fixed, named, (start, end), field_spans);
            // Py::new() is already optimized when GIL is held
            Ok(Some(Py::new(py, parse_result)?.into_py_any(py)?))
        } else {
            // Create Match object with raw captures
            let match_obj = Match::new(MatchInit {
                pattern: pattern.to_string(),
                field_specs: field_specs.to_vec(),
                field_names: field_names.to_vec(),
                normalized_names: normalized_names.to_vec(),
                captures: captures_vec,
                named_captures,
                span: (start, end),
                field_spans,
            });
            // Use Py::new_bound for better performance
            // Py::new() is already optimized when GIL is held
            Ok(Some(Py::new(py, match_obj)?.into_py_any(py)?))
        }
    } else {
        Ok(None)
    }
}

/// Full parse of `""` when the pattern has no non-empty literals and every field is a
/// default unconstrained string (`parse_pattern` sets `allows_empty_default_string_match`).
pub fn match_empty_default_string_parse(
    pattern: &str,
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
    normalized_names: &[Option<String>],
    py: Python<'_>,
    custom_converters: &HashMap<String, PyObject>,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    let value_str = "";
    let field_start: usize = 0;
    let field_end: usize = 0;
    let field_count = field_specs.len();
    let mut fixed: Vec<PyObject> = Vec::with_capacity(field_count);
    let mut named: HashMap<String, PyObject> = HashMap::with_capacity(field_count);
    let mut field_spans: HashMap<String, (usize, usize)> = HashMap::with_capacity(field_count);
    let mut captures_vec: Vec<Option<String>> = Vec::with_capacity(field_count);
    let mut named_captures: HashMap<String, String> = HashMap::with_capacity(field_count);

    let start = 0usize;
    let end = 0usize;
    let mut fixed_index = 0usize;

    for (i, spec) in field_specs.iter().enumerate() {
        if !custom_converters.is_empty() {
            let _pattern_groups = validate_custom_type_pattern(spec, custom_converters, py)?;
        }

        if !evaluate_result {
            captures_vec.push(Some(value_str.to_string()));
            if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                named_captures.insert(norm_name.clone(), value_str.to_string());
            }
        }

        if evaluate_result {
            if !crate::types::conversion::validate_alignment_precision_for_capture(spec, value_str)
            {
                return Ok(None);
            }

            let converted =
                crate::types::conversion::convert_value(spec, value_str, py, custom_converters)?;

            if let Some(ref original_name) = field_names[i] {
                if original_name.contains('[') {
                    let path = crate::parser::pattern::parse_field_path(original_name);
                    if let Some(existing_value) = get_nested_dict_value(&named, &path, py)? {
                        let are_equal: bool = {
                            let existing_obj = existing_value.bind(py);
                            let converted_obj = converted.bind(py);
                            existing_obj.eq(converted_obj).unwrap_or(false)
                        };
                        if !are_equal {
                            return Ok(None);
                        }
                    }
                    insert_nested_dict(&mut named, &path, converted, py)?;
                } else {
                    match named.get(original_name) {
                        Some(existing_value) => {
                            let are_equal: bool = {
                                let existing_obj = existing_value.clone_ref(py);
                                let converted_obj = converted.clone_ref(py);
                                existing_obj
                                    .bind(py)
                                    .eq(converted_obj.bind(py))
                                    .unwrap_or(false)
                            };
                            if !are_equal {
                                return Ok(None);
                            }
                            field_spans.insert(original_name.clone(), (field_start, field_end));
                        }
                        None => {
                            let name_for_named = original_name.clone();
                            named.insert(name_for_named.clone(), converted);
                            field_spans.insert(name_for_named, (field_start, field_end));
                        }
                    }
                }
            } else {
                fixed.push(converted);
                field_spans.insert(fixed_index.to_string(), (field_start, field_end));
                fixed_index += 1;
            }
        } else if let Some(ref original_name) = field_names[i] {
            field_spans.insert(original_name.clone(), (field_start, field_end));
        } else {
            let index_str = fixed_index.to_string();
            field_spans.insert(index_str, (field_start, field_end));
            fixed_index += 1;
        }
    }

    if evaluate_result {
        let parse_result = ParseResult::new_with_spans(fixed, named, (start, end), field_spans);
        Ok(Some(Py::new(py, parse_result)?.into_py_any(py)?))
    } else {
        let match_obj = Match::new(MatchInit {
            pattern: pattern.to_string(),
            field_specs: field_specs.to_vec(),
            field_names: field_names.to_vec(),
            normalized_names: normalized_names.to_vec(),
            captures: captures_vec,
            named_captures,
            span: (start, end),
            field_spans,
        });
        Ok(Some(Py::new(py, match_obj)?.into_py_any(py)?))
    }
}
