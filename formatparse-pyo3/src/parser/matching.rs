use crate::error;
use crate::match_rs::{Match, MatchInit};
use crate::parser::format_parser::FormatParser;
use crate::parser::raw_match::{RawMatchData, RawValue};
use crate::result::ParseResult;
use formatparse_core::{count_capturing_groups, FieldSpec, FieldType};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObjectExt;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Arc;

/// String stored on `Match` when `evaluate_result` is false: fold input continuations for `:ml` / `:blk`.
fn capture_string_for_match_storage(spec: &FieldSpec, raw: &str) -> String {
    if matches!(
        spec.field_type,
        FieldType::Multiline | FieldType::IndentBlock
    ) {
        formatparse_core::normalize_input_line_continuations(raw)
    } else {
        raw.to_string()
    }
}

/// Precomputed field metadata slices passed into capture-based match helpers.
pub struct FieldCaptureSlices<'a> {
    pub field_specs: &'a [FieldSpec],
    pub field_names: &'a [Option<String>],
    pub normalized_names: &'a [Option<String>],
    pub custom_type_groups: &'a [usize],
    pub has_nested_dict_fields: &'a [bool],
    pub nested_parsers: &'a [Option<Arc<FormatParser>>],
}

/// Pattern, field layout, and Python conversion context for [`match_with_captures`].
pub struct CapturedMatchContext<'a> {
    pub pattern: &'a str,
    pub fields: FieldCaptureSlices<'a>,
    pub py: Python<'a>,
    pub custom_converters: &'a HashMap<String, PyObject>,
    pub evaluate_result: bool,
}

/// Inputs to [`match_with_regex`] beyond the compiled `Regex`.
pub struct RegexMatchContext<'a> {
    pub string: &'a str,
    pub pattern: &'a str,
    pub field_specs: &'a [FieldSpec],
    pub field_names: &'a [Option<String>],
    pub normalized_names: &'a [Option<String>],
    pub nested_parsers: &'a [Option<Arc<FormatParser>>],
    pub py: Python<'a>,
    pub custom_converters: &'a HashMap<String, PyObject>,
    pub evaluate_result: bool,
}

/// Get a value from a nested dict structure in the named HashMap
/// Returns None if the path doesn't exist or any intermediate value is not a dict
pub fn get_nested_dict_value(
    named: &HashMap<String, PyObject>,
    path: &[String],
    py: Python,
) -> PyResult<Option<PyObject>> {
    if path.is_empty() {
        return Ok(None);
    }

    if path.len() == 1 {
        // Simple case - just get directly
        return Ok(named.get(&path[0]).map(|obj| obj.clone_ref(py)));
    }

    // Navigate through nested dicts
    let first_key = &path[0];
    let mut current_obj: PyObject = match named.get(first_key) {
        Some(v) => v.clone_ref(py),
        None => return Ok(None),
    };

    for key in path.iter().skip(1) {
        let current_dict = match current_obj.bind(py).downcast::<PyDict>() {
            Ok(d) => d,
            Err(_) => return Ok(None), // Not a dict, path doesn't exist
        };

        match current_dict.get_item(key.as_str())? {
            Some(v) => {
                // Get the PyObject to continue navigation
                current_obj = v.into();
            }
            None => return Ok(None), // Path doesn't exist
        }
    }

    Ok(Some(current_obj))
}

/// Insert a value into a nested dict structure in the named HashMap
pub fn insert_nested_dict(
    named: &mut HashMap<String, PyObject>,
    path: &[String],
    value: PyObject,
    py: Python,
) -> PyResult<()> {
    if path.is_empty() {
        return Ok(());
    }

    if path.len() == 1 {
        // Simple case - just insert directly
        named.insert(path[0].clone(), value);
        return Ok(());
    }

    // Need to create nested dicts
    let first_key = &path[0];

    // Get or create the top-level dict
    let top_dict = if let Some(existing) = named.get(first_key) {
        // Check if it's already a dict
        if let Ok(dict) = existing.bind(py).downcast::<PyDict>() {
            dict.clone()
        } else {
            // It's not a dict, we can't nest - this is an error case
            // For now, just replace it (this shouldn't happen in practice)
            let new_dict = PyDict::new(py);
            let new_dict_obj = new_dict.clone().into_py_any(py)?;
            named.insert(first_key.clone(), new_dict_obj);
            new_dict
        }
    } else {
        let new_dict = PyDict::new(py);
        let new_dict_obj = new_dict.clone().into_py_any(py)?;
        named.insert(first_key.clone(), new_dict_obj);
        new_dict
    };

    // Navigate/create nested dicts
    let mut current_dict = top_dict;
    for key in path.iter().skip(1).take(path.len() - 2) {
        let nested_dict = if let Some(existing) = current_dict.get_item(key.as_str())? {
            if let Ok(dict) = existing.downcast::<PyDict>() {
                dict.clone()
            } else {
                // Not a dict, replace it
                let new_dict = PyDict::new(py);
                let new_dict_obj = new_dict.clone().into_py_any(py)?;
                current_dict.set_item(key.as_str(), new_dict_obj)?;
                new_dict
            }
        } else {
            let new_dict = PyDict::new(py);
            let new_dict_obj = new_dict.clone().into_py_any(py)?;
            current_dict.set_item(key.as_str(), new_dict_obj)?;
            new_dict
        };
        current_dict = nested_dict;
    }

    // Set the final value
    let final_key = &path[path.len() - 1];
    current_dict.set_item(final_key.as_str(), value)?;

    Ok(())
}

/// Extract capture group for a field, handling named/unnamed groups and alignment patterns
pub fn extract_capture<'a>(
    captures: &'a Captures<'a>,
    field_index: usize,
    normalized_names: &'a [Option<String>],
    field_spec: &'a FieldSpec,
    actual_capture_index: usize,
    group_offset: usize,
) -> Option<regex::Match<'a>> {
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
fn per_field_capture_geometry(
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

/// Validate custom type pattern and return number of groups it adds
pub fn validate_custom_type_pattern(
    field_spec: &FieldSpec,
    custom_converters: &HashMap<String, PyObject>,
    py: Python,
) -> PyResult<usize> {
    if matches!(&field_spec.field_type, FieldType::Nested) {
        return Ok(0);
    }
    let mut pattern_groups = 0;

    if let formatparse_core::FieldType::Custom(type_name) = &field_spec.field_type {
        if let Some(converter_obj) = custom_converters.get(type_name) {
            let converter_ref = converter_obj.bind(py);
            if let Ok(pattern_attr) = converter_ref.getattr("pattern") {
                if let Ok(pattern_str) = pattern_attr.extract::<String>() {
                    let actual_groups = count_capturing_groups(&pattern_str);
                    pattern_groups = actual_groups;

                    if let Ok(group_count_attr) = converter_ref.getattr("regex_group_count") {
                        // Try to extract as int first
                        if let Ok(group_count) = group_count_attr.extract::<i64>() {
                            if group_count < 0 {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("regex_group_count must be >= 0, got {}", group_count),
                                ));
                            }
                            if group_count == 0 && actual_groups > 0 {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is 0", type_name, actual_groups)
                                ));
                            }
                            if group_count > actual_groups as i64 {
                                return Err(error::regex_group_index_error(
                                    type_name,
                                    actual_groups,
                                    group_count,
                                ));
                            }
                        } else {
                            // regex_group_count is None
                            if actual_groups > 0 {
                                return Err(error::custom_type_error(
                                            type_name,
                                            &format!("pattern has {} capturing groups but regex_group_count is None", actual_groups)
                                        ));
                            }
                        }
                    } else {
                        // No regex_group_count attribute - must have 0 groups
                        if actual_groups > 0 {
                            return Err(error::custom_type_error(
                                            type_name,
                                            &format!("pattern has {} capturing groups but regex_group_count is not set", actual_groups)
                                        ));
                        }
                    }
                }
            }
        }
    }

    Ok(pattern_groups)
}

/// Match using existing captures and return raw data (no Python objects)
/// This is used for batch processing to defer Python object creation
/// Returns None if custom converters are needed (they require Python)
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

    let full_match = captures.get(0).ok_or_else(|| {
        "regex match missing capture group 0".to_string()
    })?;
    let start = full_match.start();
    let end = full_match.end();

    let field_count = field_specs.len();
    let mut raw_data = RawMatchData::with_capacity(field_count);
    raw_data.span = (start, end);

    let mut group_offset = 0;
    let mut actual_capture_index = 1;

    for (i, spec) in field_specs.iter().enumerate() {
        let pattern_groups = custom_type_groups.get(i).copied().unwrap_or(0);

        if matches!(spec.field_type, FieldType::Nested) {
            return Err("Nested format fields require Python conversion".to_string());
        }

        let cap = extract_capture(
            captures,
            i,
            normalized_names,
            spec,
            actual_capture_index,
            group_offset,
        );

        actual_capture_index += 1;

        if let Some(cap) = cap {
            let value_str = cap.as_str();
            let field_start = cap.start();
            let field_end = cap.end();

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
fn values_equal(a: &RawValue, b: &RawValue) -> bool {
    match (a, b) {
        (RawValue::String(s1), RawValue::String(s2)) => s1 == s2,
        (RawValue::Integer(n1), RawValue::Integer(n2)) => n1 == n2,
        (RawValue::Float(f1), RawValue::Float(f2)) => (f1 - f2).abs() < f64::EPSILON,
        (RawValue::Boolean(b1), RawValue::Boolean(b2)) => b1 == b2,
        (RawValue::None, RawValue::None) => true,
        _ => false,
    }
}

/// Match using existing captures (optimized for findall)
/// Note: captures are from the full string, so positions are already absolute
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
                if !crate::types::conversion::validate_alignment_precision_for_capture(spec, value_str) {
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
                        if !crate::types::conversion::validate_alignment_precision_for_capture(spec_j, vs) {
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

    if let Some(captures) = regex.captures(string) {
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
                    if !crate::types::conversion::validate_alignment_precision_for_capture(spec, value_str) {
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
                            if !crate::types::conversion::validate_alignment_precision_for_capture(spec_j, vs) {
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
                            let nested_arc = nested_parsers.get(i).and_then(|x| x.as_ref()).ok_or_else(
                                || {
                                    pyo3::exceptions::PyValueError::new_err(
                                        "internal error: nested parser missing",
                                    )
                                },
                            )?;
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
            if !crate::types::conversion::validate_alignment_precision_for_capture(spec, value_str) {
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
