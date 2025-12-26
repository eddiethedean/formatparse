use crate::error;
use crate::result::ParseResult;
use crate::types::FieldSpec;
use crate::match_rs::Match;
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
    if let Some(norm_name) = normalized_names.get(field_index).and_then(|n| n.as_ref()) {
        // Use normalized name to get the capture
        captures.name(norm_name.as_str())
    } else {
        // For alignment patterns, we have nested capturing groups
        // The outermost group includes padding, the innermost has the text
        let capture_group_index = actual_capture_index + group_offset;
        if field_spec.alignment.is_some() {
            // Try to find the innermost capturing group (usually next group for alignment patterns)
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

/// Match a regex against a string and extract results
pub fn match_with_regex(
    regex: &Regex,
    string: &str,
    pattern: &str,
    field_specs: &[FieldSpec],
    field_names: &[Option<String>],
    normalized_names: &[Option<String>],
    py: Python,
    custom_converters: HashMap<String, PyObject>,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    if let Some(captures) = regex.captures(string) {
        let mut fixed = Vec::new();
        let mut named: HashMap<String, PyObject> = HashMap::new();
        let mut field_spans: HashMap<String, (usize, usize)> = HashMap::new();
        let mut captures_vec = Vec::new();  // For Match object when evaluate_result=False
        let mut named_captures = HashMap::new();  // For Match object when evaluate_result=False

        let full_match = captures.get(0).unwrap();
        let start = full_match.start();
        let end = full_match.end();

        let mut fixed_index = 0;
        let mut group_offset = 0;
        // Track the actual capture group index (accounts for both named and unnamed groups)
        let mut actual_capture_index = 1;  // Start at 1 (group 0 is full match)
        
        for (i, spec) in field_specs.iter().enumerate() {
            // Validate regex_group_count for custom types with capturing groups
            // Also track how many groups this pattern adds for group_offset calculation
            let pattern_groups = validate_custom_type_pattern(spec, &custom_converters, py)?;
            
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
                
                // Store raw capture for Match object
                captures_vec.push(Some(value_str.to_string()));
                if let Some(norm_name) = normalized_names.get(i).and_then(|n| n.as_ref()) {
                    named_captures.insert(norm_name.clone(), value_str.to_string());
                }
                
                if evaluate_result {
                    let converted = spec.convert_value(value_str, py, &custom_converters)?;

                    // Use original field name (with hyphens/dots) for the result
                    if let Some(ref original_name) = field_names[i] {
                        // Check if this is a dict-style field name (contains [])
                        if original_name.contains('[') {
                            // Parse the path and insert into nested dict structure
                            let path = crate::parser::pattern::parse_field_path(original_name);
                            // Check for repeated field names - compare values if path already exists
                            if let Some(existing_value) = get_nested_dict_value(&named, &path, py)? {
                                // Compare values using Python's equality
                                let existing_obj = existing_value.to_object(py);
                                let converted_obj = converted.to_object(py);
                                let are_equal: bool = existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false);
                                if !are_equal {
                                    // Values don't match for repeated name
                                    return Ok(None);
                                }
                            }
                            insert_nested_dict(&mut named, &path, converted, py)?;
                        } else {
                            // Regular flat field name
                            // Check for repeated field names - values must match
                            if let Some(existing_value) = named.get(original_name) {
                                // Compare values using Python's equality
                                let existing_obj = existing_value.to_object(py);
                                let converted_obj = converted.to_object(py);
                                let are_equal: bool = existing_obj.bind(py).eq(converted_obj.bind(py)).unwrap_or(false);
                                if !are_equal {
                                    // Values don't match for repeated name
                                    return Ok(None);
                                }
                            }
                            named.insert(original_name.clone(), converted);
                        }
                        // Store span by field name
                        field_spans.insert(original_name.clone(), (field_start, field_end));
                    } else {
                        fixed.push(converted);
                        // Store span by fixed index
                        field_spans.insert(fixed_index.to_string(), (field_start, field_end));
                        fixed_index += 1;
                    }
                } else {
                    // Store span even when not evaluating
                    if let Some(ref original_name) = field_names[i] {
                        field_spans.insert(original_name.clone(), (field_start, field_end));
                    } else {
                        field_spans.insert(fixed_index.to_string(), (field_start, field_end));
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
            Ok(Some(Py::new(py, match_obj)?.to_object(py)))
        }
    } else {
        Ok(None)
    }
}

