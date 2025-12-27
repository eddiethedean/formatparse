use crate::error;
use crate::result::ParseResult;
use crate::types::FieldSpec;
use crate::match_rs::Match;
use crate::parser::raw_match::{RawMatchData, RawValue};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use regex::{Regex, Captures};
use std::collections::HashMap;

/// Count the number of capturing groups in a regex pattern
pub fn count_capturing_groups(pattern: &str) -> usize {
    let mut count = 0;
    let mut i = 0;
    let chars: Vec<char> = pattern.chars().collect();
    
    while i < chars.len() {
        if chars[i] == '\\' {
            // Skip escaped character
            i += 2;
            if i > chars.len() {
                break;
            }
            continue;
        }
        if chars[i] == '(' {
            // Check if it's a non-capturing group
            if i + 1 < chars.len() && chars[i + 1] == '?' {
                // Non-capturing group: (?: ...), (?= ...), (?! ...), etc.
                i += 2;
                if i < chars.len() && (chars[i] == ':' || chars[i] == '=' || chars[i] == '!' || 
                                       chars[i] == '<' || (i > 0 && chars[i-1] == '?' && chars[i] == 'P')) {
                    if chars[i] == 'P' && i + 1 < chars.len() && chars[i + 1] == '<' {
                        // Named group (?P<name>...), skip the name
                        i += 2;
                        while i < chars.len() && chars[i] != '>' {
                            i += 1;
                        }
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                }
                continue;
            }
            // It's a capturing group
            count += 1;
        }
        i += 1;
    }
    count
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
        return Ok(named.get(&path[0]).cloned());
    }
    
    // Navigate through nested dicts
    let first_key = &path[0];
    let mut current_obj = match named.get(first_key) {
        Some(v) => v.clone(),
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
                current_obj = v.to_object(py);
            },
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
            let new_dict = PyDict::new_bound(py);
            named.insert(first_key.clone(), new_dict.to_object(py));
            new_dict
        }
    } else {
        let new_dict = PyDict::new_bound(py);
        named.insert(first_key.clone(), new_dict.to_object(py));
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
                let new_dict = PyDict::new_bound(py);
                current_dict.set_item(key.as_str(), new_dict.to_object(py))?;
                new_dict
            }
        } else {
            let new_dict = PyDict::new_bound(py);
            current_dict.set_item(key.as_str(), new_dict.to_object(py))?;
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
            captures.get(capture_group_index + 1).or_else(|| captures.get(capture_group_index))
        } else {
            captures.get(capture_group_index)
        }
    }
}

/// Validate custom type pattern and return number of groups it adds
pub fn validate_custom_type_pattern(
    field_spec: &FieldSpec,
    custom_converters: &HashMap<String, PyObject>,
    py: Python,
) -> PyResult<usize> {
    let mut pattern_groups = 0;
    
    if let crate::types::FieldType::Custom(type_name) = &field_spec.field_type {
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
                                    format!("regex_group_count must be >= 0, got {}", group_count)
                                ));
                            }
                            if group_count == 0 && actual_groups > 0 {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is 0", type_name, actual_groups)
                                ));
                            }
                            if group_count > actual_groups as i64 {
                                return Err(error::regex_group_index_error(type_name, actual_groups, group_count));
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
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
    normalized_names: &[Option<String>],
    custom_type_groups: &[usize],
    has_nested_dict_fields: &[bool],
) -> Result<Option<RawMatchData>, String> {
    let full_match = captures.get(0).unwrap();
    let start = full_match.start();
    let end = full_match.end();
    
    let field_count = field_specs.len();
    let mut raw_data = RawMatchData::with_capacity(field_count);
    raw_data.span = (start, end);
    
    let mut group_offset = 0;
    let mut actual_capture_index = 1;
    
    for (i, spec) in field_specs.iter().enumerate() {
        let pattern_groups = custom_type_groups.get(i).copied().unwrap_or(0);
        
        let cap = extract_capture(
            captures,
            i,
            normalized_names,
            spec,
            actual_capture_index,
            group_offset,
        );
        
        if normalized_names.get(i).and_then(|n| n.as_ref()).is_none() {
            actual_capture_index += 1;
        } else {
            actual_capture_index += 1;
        }
        
        if let Some(cap) = cap {
            let value_str = cap.as_str();
            let field_start = cap.start();
            let field_end = cap.end();
            
            // Try to convert to raw value (fails for custom types and datetime)
            match spec.convert_value_raw(value_str) {
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
                                    return Ok(None);  // Values don't match
                                }
                            } else {
                                raw_data.named.insert(original_name.clone(), raw_value);
                            }
                        }
                        raw_data.field_spans.insert(original_name.clone(), (field_start, field_end));
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
    _string: &str,
    _match_start: usize,
    pattern: &str,
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
    normalized_names: &[Option<String>],
    custom_type_groups: &[usize],  // Pre-computed pattern_groups per field
    has_nested_dict_fields: &[bool],  // Pre-computed flags: does field name contain '['?
    py: Python,
    custom_converters: &HashMap<String, PyObject>,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    let full_match = captures.get(0).unwrap();
    let start = full_match.start();  // Already absolute position in full string
    let end = full_match.end();      // Already absolute position in full string
    
    // Pre-allocate with capacity based on expected field count
    let field_count = field_specs.len();
    // Fast path: for single-field patterns, use optimized allocation
    let mut fixed = Vec::with_capacity(field_count);
    let mut named: HashMap<String, PyObject> = HashMap::with_capacity(field_count.max(1));
    let mut field_spans: HashMap<String, (usize, usize)> = HashMap::with_capacity(field_count.max(1));
    let mut captures_vec = Vec::with_capacity(field_count);  // For Match object when evaluate_result=False
    let mut named_captures = HashMap::with_capacity(field_count);  // For Match object when evaluate_result=False
    let mut group_offset = 0;
    // Track the actual capture group index (accounts for both named and unnamed groups)
    let mut actual_capture_index = 1;  // Start at 1 (group 0 is full match)
    
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
                captures_vec.push(Some(value_str.to_string()));
                if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                    named_captures.insert(norm_name.clone(), value_str.to_string());
                }
            }
            // For evaluate_result=True, we don't need to store raw captures, saving allocations
            
            if evaluate_result {
                // Validate alignment+precision constraints (issue #3)
                // This prevents invalid cases where fill characters are in wrong positions
                if !spec.validate_alignment_precision(value_str) {
                    return Ok(None);
                }
                
                let converted = spec.convert_value(value_str, py, &custom_converters)?;

                // Use original field name (with hyphens/dots) for the result
                if let Some(ref original_name) = field_names[i] {
                    // Use pre-computed flag to avoid contains('[') check in hot path
                    if has_nested_dict_fields.get(i).copied().unwrap_or(false) {
                        // Parse the path and insert into nested dict structure
                        let path = crate::parser::pattern::parse_field_path(original_name);
                        // Check for repeated field names - compare values if path already exists
                        if let Some(existing_value) = get_nested_dict_value(&named, &path, py)? {
                            // Compare values using Python's equality (batch GIL operation)
                            let are_equal: bool = {
                                let existing_obj = existing_value.to_object(py);
                                let converted_obj = converted.to_object(py);
                                existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false)
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
                                    let existing_obj = existing_value.to_object(py);
                                    let converted_obj = converted.to_object(py);
                                    existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false)
                                };
                                if !are_equal {
                                    // Values don't match for repeated name
                                    return Ok(None);
                                }
                            },
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
        Ok(Some(Py::new(py, parse_result)?.to_object(py)))
    } else {
        // Create Match object with raw captures
        // Note: pattern is static, but Match needs owned String - this is acceptable
        // as Match objects are only created when evaluate_result=False (less common)
        let match_obj = Match::new(
            pattern.to_string(),
            field_specs.to_vec(),
            field_names.to_vec(),
            normalized_names.to_vec(),
            captures_vec,
            named_captures,
            (start, end),
            field_spans,
        );
        Ok(Some(Py::new(py, match_obj)?.to_object(py)))
    }
}

/// Match a regex against a string and extract results
pub fn match_with_regex(
    regex: &Regex,
    string: &str,
    pattern: &str,
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
    normalized_names: &[Option<String>],
    py: Python,
    custom_converters: &HashMap<String, PyObject>,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    if let Some(captures) = regex.captures(string) {
        // Pre-allocate with capacity based on expected field count
        let field_count = field_specs.len();
        let mut fixed = Vec::with_capacity(field_count);
        let mut named: HashMap<String, PyObject> = HashMap::with_capacity(field_count);
        let mut field_spans: HashMap<String, (usize, usize)> = HashMap::with_capacity(field_count);
        let mut captures_vec = Vec::with_capacity(field_count);  // For Match object when evaluate_result=False
        let mut named_captures = HashMap::with_capacity(field_count);  // For Match object when evaluate_result=False

        let full_match = captures.get(0).unwrap();
        let start = full_match.start();
        let end = full_match.end();

        let mut fixed_index = 0;
        let mut group_offset = 0;
        // Track the actual capture group index (accounts for both named and unnamed groups)
        let mut actual_capture_index = 1;  // Start at 1 (group 0 is full match)
        
        for (i, spec) in field_specs.iter().enumerate() {
            // Validate regex_group_count for custom types with capturing groups (only if custom types exist)
            let pattern_groups = if !custom_converters.is_empty() {
                validate_custom_type_pattern(spec, &custom_converters, py)?
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
                
                // Store raw capture for Match object (only if needed)
                if !evaluate_result {
                    captures_vec.push(Some(value_str.to_string()));
                    if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                        named_captures.insert(norm_name.clone(), value_str.to_string());
                    }
                }
                
                if evaluate_result {
                    // Validate alignment+precision constraints (issue #3)
                    // This prevents invalid cases where fill characters are in wrong positions
                    if !spec.validate_alignment_precision(value_str) {
                        return Ok(None);
                    }
                    
                    let converted = spec.convert_value(value_str, py, &custom_converters)?;

                    // Use original field name (with hyphens/dots) for the result
                    if let Some(ref original_name) = field_names[i] {
                        // Check if this is a dict-style field name (contains [])
                        if original_name.contains('[') {
                            // Parse the path and insert into nested dict structure
                            let path = crate::parser::pattern::parse_field_path(original_name);
                            // Check for repeated field names - compare values if path already exists
                            if let Some(existing_value) = get_nested_dict_value(&named, &path, py)? {
                                // Compare values using Python's equality (batch GIL operation)
                                let are_equal: bool = {
                                    let existing_obj = existing_value.to_object(py);
                                    let converted_obj = converted.to_object(py);
                                    existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false)
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
                                        let existing_obj = existing_value.to_object(py);
                                        let converted_obj = converted.to_object(py);
                                        existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false)
                                    };
                                    if !are_equal {
                                        // Values don't match for repeated name
                                        return Ok(None);
                                    }
                                    // Store span for repeated name
                                    field_spans.insert(original_name.clone(), (field_start, field_end));
                                }
                                None => {
                                    // First occurrence - just insert (common case)
                                    // Reuse the clone for both insertions
                                    let name_for_named = original_name.clone();
                                    named.insert(name_for_named.clone(), converted);
                                    field_spans.insert(name_for_named, (field_start, field_end));
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
            Ok(Some(Py::new(py, parse_result)?.to_object(py)))
        } else {
            // Create Match object with raw captures
            let match_obj = Match::new(
                pattern.to_string(),
                field_specs.to_vec(),
                field_names.to_vec(),
                normalized_names.to_vec(),
                captures_vec,
                named_captures,
                (start, end),
                field_spans,
            );
            // Use Py::new_bound for better performance
            // Py::new() is already optimized when GIL is held
            Ok(Some(Py::new(py, match_obj)?.to_object(py)))
        }
    } else {
        Ok(None)
    }
}

