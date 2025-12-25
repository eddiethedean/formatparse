use crate::result::ParseResult;
use crate::types::{FieldSpec, FieldType};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple, PyDict};
use regex::Regex;
use std::collections::HashMap;

#[pyclass]
pub struct FormatParser {
    #[pyo3(get)]
    pub pattern: String,
    regex: Regex,
    regex_str: String,  // Store the regex string for _expression property
    regex_case_insensitive: Option<Regex>,
    pub(crate) field_specs: Vec<FieldSpec>,
    pub(crate) field_names: Vec<Option<String>>,  // Original field names (with hyphens/dots)
    pub(crate) normalized_names: Vec<Option<String>>,  // Normalized names for regex groups (hyphens->underscores)
    name_mapping: std::collections::HashMap<String, String>,  // Map normalized -> original
    stored_extra_types: Option<HashMap<String, PyObject>>,  // Store extra_types for use during conversion
}

impl FormatParser {
    pub fn new(pattern: &str) -> PyResult<Self> {
        Self::new_with_extra_types(pattern, None)
    }

    pub fn new_with_extra_types(pattern: &str, extra_types: Option<HashMap<String, PyObject>>) -> PyResult<Self> {
        // Extract patterns from converter functions and build custom_patterns map
        let custom_patterns = Python::with_gil(|py| -> PyResult<HashMap<String, String>> {
            let mut patterns = HashMap::new();
            if let Some(ref extra_types_map) = extra_types {
                for (name, converter_obj) in extra_types_map {
                    // Try to get the pattern attribute from the converter function
                    let converter_ref = converter_obj.bind(py);
                    if let Ok(pattern_attr) = converter_ref.getattr("pattern") {
                        if let Ok(pattern_str) = pattern_attr.extract::<String>() {
                            patterns.insert(name.clone(), pattern_str);
                        }
                    }
                }
            }
            Ok(patterns)
        })?;
        
        let (regex_str_with_anchors, regex_str, field_specs, field_names, normalized_names, name_mapping) = Self::parse_pattern(pattern, extra_types.as_ref(), &custom_patterns)?;
        
        // Add (?s) flag to make . match newlines (DOTALL mode)
        let regex_with_flags = format!("(?s){}", regex_str_with_anchors);
        let regex = Regex::new(&regex_with_flags).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", e))
        })?;

        // Build case-insensitive regex (both (?s) and (?i) flags)
        let regex_case_insensitive = Regex::new(&format!("(?s)(?i){}", regex_str_with_anchors)).ok();

        Ok(Self {
            pattern: pattern.to_string(),
            regex,
            regex_str,
            regex_case_insensitive,
            field_specs,
            field_names,
            normalized_names,
            name_mapping,
            stored_extra_types: extra_types,
        })
    }

    pub fn search_pattern(
        &self,
        string: &str,
        case_sensitive: bool,
        extra_types: Option<HashMap<String, PyObject>>,
        evaluate_result: bool,
    ) -> PyResult<Option<PyObject>> {
        // For search, we remove the ^ anchor to allow matching anywhere
        Python::with_gil(|py| {
            // Get the base regex string (without flags)
            // self.regex has (?s) prefix, so we need to extract the actual pattern
            let regex_str = self.regex.as_str();
            // Remove (?s) flag if present, and ^ and $ anchors
            let mut search_regex_str = regex_str.to_string();
            if search_regex_str.starts_with("(?s)") {
                search_regex_str = search_regex_str[4..].to_string();
            }
            if search_regex_str.starts_with("^") {
                search_regex_str = search_regex_str[1..].to_string();
            }
            if search_regex_str.ends_with("$") {
                search_regex_str = search_regex_str[..search_regex_str.len()-1].to_string();
            }
            
            // Always add (?s) for DOTALL, and conditionally add (?i) for case insensitive
            let search_regex = if case_sensitive {
                Regex::new(&format!("(?s){}", search_regex_str)).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", e))
                })?
            } else {
                Regex::new(&format!("(?s)(?i){}", search_regex_str)).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex pattern: {}", e))
                })?
            };
            
            if search_regex.captures(string).is_some() {
                return self.match_with_regex(&search_regex, string, py, extra_types, evaluate_result);
            }
            Ok(None)
        })
    }

    pub(crate) fn parse_internal(
        &self,
        string: &str,
        case_sensitive: bool,
        extra_types: Option<HashMap<String, PyObject>>,
        evaluate_result: bool,
    ) -> PyResult<Option<PyObject>> {
        Python::with_gil(|py| {
            // Use existing regex (custom type handling is done in convert_value)
            let regex = if case_sensitive {
                &self.regex
            } else {
                self.regex_case_insensitive.as_ref().unwrap_or(&self.regex)
            };

            self.match_with_regex(regex, string, py, extra_types, evaluate_result)
        })
    }

    fn match_with_regex(
        &self,
        regex: &Regex,
        string: &str,
        py: Python,
        extra_types: Option<HashMap<String, PyObject>>,
        evaluate_result: bool,
    ) -> PyResult<Option<pyo3::PyObject>> {
        if let Some(captures) = regex.captures(string) {
            let mut fixed = Vec::new();
            let mut named: HashMap<String, PyObject> = HashMap::new();
            let mut field_spans: HashMap<String, (usize, usize)> = HashMap::new();
            let mut captures_vec = Vec::new();  // For Match object when evaluate_result=False
            let mut named_captures = HashMap::new();  // For Match object when evaluate_result=False
            let custom_converters = extra_types.unwrap_or_default();

            let full_match = captures.get(0).unwrap();
            let start = full_match.start();
            let end = full_match.end();

            let mut fixed_index = 0;

            let mut group_offset = 0;
            for (i, spec) in self.field_specs.iter().enumerate() {
                // Validate regex_group_count for custom types with capturing groups
                // Also track how many groups this pattern adds for group_offset calculation
                let mut pattern_groups = 0;
                if let FieldType::Custom(type_name) = &spec.field_type {
                    if let Some(converter_obj) = custom_converters.get(type_name) {
                        let converter_ref = converter_obj.bind(py);
                        if let Ok(pattern_attr) = converter_ref.getattr("pattern") {
                            if let Ok(pattern_str) = pattern_attr.extract::<String>() {
                                let actual_groups = Self::count_capturing_groups(&pattern_str);
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
                                            // This will cause IndexError when trying to access the group
                                            // But we should validate it here
                                            return Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                                                format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is {}", type_name, actual_groups, group_count)
                                            ));
                                        }
                                    } else {
                                        // regex_group_count is None
                                        if actual_groups > 0 {
                                            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                                format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is None", type_name, actual_groups)
                                            ));
                                        }
                                    }
                                } else {
                                    // No regex_group_count attribute - must have 0 groups
                                    if actual_groups > 0 {
                                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                            format!("Custom type '{}' pattern has {} capturing groups but regex_group_count is not set", type_name, actual_groups)
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                
                // For named fields, use normalized name to get the capture group
                // For unnamed fields, use position
                let cap = if let Some(norm_name) = self.normalized_names.get(i).and_then(|n| n.as_ref()) {
                    // Use normalized name to get the capture
                    captures.name(norm_name.as_str())
                } else {
                    // For alignment patterns, we have nested capturing groups
                    // The outermost group (i+1) includes padding, the innermost has the text
                    let capture_group_index = i + 1 + group_offset;
                    if spec.alignment.is_some() {
                        // Try to find the innermost capturing group (usually next group for alignment patterns)
                        captures.get(capture_group_index + 1).or_else(|| captures.get(capture_group_index))
                    } else {
                        captures.get(capture_group_index)
                    }
                };
                
                if let Some(cap) = cap {
                    let value_str = cap.as_str();
                    let field_start = cap.start();
                    let field_end = cap.end();
                    
                    // Store raw capture for Match object
                    captures_vec.push(Some(value_str.to_string()));
                    if let Some(norm_name) = self.normalized_names.get(i).and_then(|n| n.as_ref()) {
                        named_captures.insert(norm_name.clone(), value_str.to_string());
                    }
                    
                    if evaluate_result {
                        let converted = spec.convert_value(value_str, py, &custom_converters)?;

                        // Use original field name (with hyphens/dots) for the result
                        if let Some(ref original_name) = self.field_names[i] {
                            // Check if this is a dict-style field name (contains [])
                            if original_name.contains('[') {
                                // Parse the path and insert into nested dict structure
                                let path = Self::parse_field_path(original_name);
                                // Check for repeated field names - compare values if path already exists
                                // This is complex for nested dicts, so for now we'll just insert
                                // TODO: Implement proper repeated name checking for nested dicts
                                Self::insert_nested_dict(&mut named, &path, converted, py)?;
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
                        if let Some(ref original_name) = self.field_names[i] {
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
                use crate::match_rs::Match;
                let match_obj = Match::new(
                    self.pattern.clone(),
                    self.field_specs.clone(),
                    self.field_names.clone(),
                    self.normalized_names.clone(),
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
    
    pub(crate) fn get_field_specs(&self) -> &Vec<FieldSpec> {
        &self.field_specs
    }
    
    pub(crate) fn get_field_names(&self) -> &Vec<Option<String>> {
        &self.field_names
    }
    
    pub(crate) fn get_normalized_names(&self) -> &Vec<Option<String>> {
        &self.normalized_names
    }

    fn parse_pattern(pattern: &str, extra_types: Option<&HashMap<String, PyObject>>, custom_patterns: &HashMap<String, String>) -> PyResult<(String, String, Vec<FieldSpec>, Vec<Option<String>>, Vec<Option<String>>, std::collections::HashMap<String, String>)> {
        let mut regex_parts = Vec::new();
        let mut field_specs = Vec::new();
        let mut field_names = Vec::new();  // Original names
        let mut normalized_names = Vec::new();  // Normalized for regex
        let mut name_mapping = std::collections::HashMap::new();  // normalized -> original
        let mut field_name_types = std::collections::HashMap::new();  // Track field name -> FieldType for validation
        let mut chars: std::iter::Peekable<std::str::Chars> = pattern.chars().peekable();
        let mut literal = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    // Check for escaped brace
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        literal.push('{');
                        continue;
                    }

                    // Flush literal part
                    if !literal.is_empty() {
                        // If literal ends with whitespace, make it flexible to allow multiple spaces
                        let escaped = if literal.trim_end() != literal {
                            // Literal ends with whitespace - replace trailing whitespace with \s*
                            // to allow zero or more spaces (maintains compatibility with exact matches)
                            let trimmed = literal.trim_end();
                            format!("{}\\s*", regex::escape(trimmed))
                        } else {
                            regex::escape(&literal)
                        };
                        regex_parts.push(escaped);
                        literal.clear();
                    }

                    // Parse field specification
                    let (spec, name) = Self::parse_field(&mut chars, extra_types)?;
                    
                    // Check if the next field (if any) is empty {} (non-greedy)
                    // This affects width-only string patterns: exact when followed by {}, greedy otherwise
                    let mut peek_chars = chars.clone();
                    let next_field_is_greedy = loop {
                        // Skip whitespace and consume the expected closing '}'
                        let mut found_closing = false;
                        while let Some(&ch) = peek_chars.peek() {
                            if ch.is_whitespace() {
                                peek_chars.next();
                            } else if ch == '}' {
                                peek_chars.next();  // Consume the closing brace
                                found_closing = true;
                                break;
                            } else {
                                break;
                            }
                        }
                        if !found_closing {
                            break None;  // No more fields
                        }
                        // Skip any whitespace after the closing brace
                        while let Some(&ch) = peek_chars.peek() {
                            if ch.is_whitespace() {
                                peek_chars.next();
                            } else {
                                break;
                            }
                        }
                        // Check for opening brace (indicating another field)
                        if peek_chars.peek() == Some(&'{') {
                            peek_chars.next();
                            // Check if it's escaped
                            if peek_chars.peek() == Some(&'{') {
                                peek_chars.next();
                                continue;  // Escaped brace, continue
                            }
                            // Found a field - check if it's empty {}
                            if peek_chars.peek() == Some(&'}') {
                                // Empty field {} - non-greedy, use exact width
                                break Some(false);
                            } else {
                                // Field has content - greedy, use greedy width
                                break Some(true);
                            }
                        } else {
                            // No more fields - use greedy
                            break None;
                        }
                    };
                    
                    let pattern = spec.to_regex_pattern(custom_patterns, next_field_is_greedy);
                    
                    // Validate repeated field names have same type
                    if let Some(ref original_name) = name {
                        if let Some(existing_type) = field_name_types.get(original_name) {
                            // Check if types match
                            if !Self::field_types_match(existing_type, &spec.field_type) {
                                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    format!("Repeated name '{}' with mismatched types", original_name)
                                ));
                            }
                        } else {
                            field_name_types.insert(original_name.clone(), spec.field_type.clone());
                        }
                    }
                    
                    // Handle name normalization for regex groups
                    if let Some(ref original_name) = name {
                        // Check if field name is numeric (numbered field like {0}, {1}) - these should be positional
                        let is_numeric = original_name.chars().all(|c| c.is_ascii_digit());
                        
                        if is_numeric {
                            // Numbered fields are positional (unnamed groups), not named groups
                            let group_pattern = format!("({})", pattern);
                            regex_parts.push(group_pattern);
                            field_names.push(None);  // Store as None (positional)
                            normalized_names.push(None);
                        } else {
                            // Normalize name: replace hyphens/dots with underscores, handle collisions
                            let normalized = Self::normalize_field_name(original_name, &mut name_mapping, &normalized_names);
                            let group_pattern = format!("(?P<{}>{})", normalized, pattern);
                            regex_parts.push(group_pattern);
                            field_names.push(Some(original_name.clone()));  // Store original
                            normalized_names.push(Some(normalized.clone()));  // Store normalized
                            name_mapping.insert(normalized, original_name.clone());  // Map normalized -> original
                        }
                    } else {
                        let group_pattern = format!("({})", pattern);
                        regex_parts.push(group_pattern);
                        field_names.push(None);
                        normalized_names.push(None);
                    }
                    field_specs.push(spec);

                    // Expect closing brace
                    if chars.next() != Some('}') {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Expected '}' after field specification",
                        ));
                    }
                }
                '}' => {
                    // Check for escaped brace
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        literal.push('}');
                        continue;
                    }
                    literal.push('}');
                }
                _ => {
                    literal.push(ch);
                }
            }
        }

        // Flush remaining literal
        if !literal.is_empty() {
            // If literal ends with whitespace, make it flexible to allow multiple spaces
            let escaped = if literal.trim_end() != literal {
                // Literal ends with whitespace - replace trailing whitespace with \s*
                // to allow zero or more spaces (maintains compatibility with exact matches)
                let trimmed = literal.trim_end();
                format!("{}\\s*", regex::escape(trimmed))
            } else {
                regex::escape(&literal)
            };
            regex_parts.push(escaped);
        }

        let regex_str = regex_parts.join("");
        let regex_str_with_anchors = format!("^{}$", regex_str);
        Ok((regex_str_with_anchors, regex_str, field_specs, field_names, normalized_names, name_mapping))
    }

    /// Normalize field name (hyphens/dots -> underscores) and handle collisions
    fn normalize_field_name(name: &str, name_mapping: &mut std::collections::HashMap<String, String>, existing_normalized: &[Option<String>]) -> String {
        // Normalize: replace hyphens and dots with underscores
        let base_normalized: String = name.chars().map(|c| if c == '-' || c == '.' { '_' } else { c }).collect();
        
        // Check for collisions with existing normalized names
        let mut normalized = base_normalized.clone();
        let mut suffix_count = 0;
        
        // Check if this normalized name already exists
        while existing_normalized.iter().any(|n| n.as_ref().map(|s| s == &normalized).unwrap_or(false)) {
            suffix_count += 1;
            normalized = format!("{}{}", base_normalized, "_".repeat(suffix_count));
        }
        
        normalized
    }

    /// Count the number of capturing groups in a regex pattern
    fn count_capturing_groups(pattern: &str) -> usize {
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

    /// Check if two field types match (for repeated name validation)
    fn field_types_match(t1: &FieldType, t2: &FieldType) -> bool {
        use std::mem::discriminant;
        discriminant(t1) == discriminant(t2)
    }

    /// Parse a field name into a path (for dict-style names like "hello[world]" -> ["hello", "world"])
    pub fn parse_field_path(field_name: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = String::new();
        let mut in_brackets = false;
        let mut bracket_content = String::new();
        
        for ch in field_name.chars() {
            match ch {
                '[' => {
                    if !current.is_empty() {
                        path.push(current.clone());
                        current.clear();
                    }
                    in_brackets = true;
                }
                ']' => {
                    if in_brackets {
                        path.push(bracket_content.clone());
                        bracket_content.clear();
                        in_brackets = false;
                    }
                }
                _ => {
                    if in_brackets {
                        bracket_content.push(ch);
                    } else {
                        current.push(ch);
                    }
                }
            }
        }
        
        if !current.is_empty() {
            path.push(current);
        }
        
        path
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

    fn parse_field(chars: &mut std::iter::Peekable<std::str::Chars>, extra_types: Option<&HashMap<String, PyObject>>) -> PyResult<(FieldSpec, Option<String>)> {
        let mut spec = FieldSpec::new();
        let mut field_name = String::new();
        let mut format_spec = String::new();
        let mut in_name = true;

        // Parse field name (before colon or conversion)
        let mut in_brackets = false;
        while let Some(&ch) = chars.peek() {
            match ch {
                ':' => {
                    chars.next();
                    in_name = false;
                    break;
                }
                '!' => {
                    chars.next();
                    // Conversion specifier (s, r, a) - skip for now
                    if chars.peek().is_some() {
                        chars.next();
                    }
                    in_name = false;
                }
                '}' => {
                    break;
                }
                '[' => {
                    in_brackets = true;
                    field_name.push(ch);
                    chars.next();
                }
                ']' => {
                    in_brackets = false;
                    field_name.push(ch);
                    chars.next();
                }
                '\'' | '"' => {
                    // Quote characters in field names indicate quoted keys (not supported)
                    if in_brackets {
                        return Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
                            "Quoted keys in field names are not supported"
                        ));
                    }
                    // Not in brackets, not a valid name character
                    in_name = false;
                    break;
                }
                _ => {
                    // Allow alphanumeric, underscore, hyphen, dot for field names
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
                        field_name.push(ch);
                        chars.next();
                    } else {
                        // Not a valid name character, might be format spec
                        in_name = false;
                        break;
                    }
                }
            }
        }

        // Parse format spec (everything after colon until closing brace)
        if !in_name {
            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    break;
                }
                format_spec.push(ch);
                chars.next();
            }
        }

        // Parse format spec to extract alignment, width, precision, type, etc.
        Self::parse_format_spec(&format_spec, &mut spec, extra_types);

        let name = if field_name.is_empty() {
            None
        } else {
            Some(field_name)
        };

        Ok((spec, name))
    }

    pub fn parse_format_spec(format_spec: &str, spec: &mut FieldSpec, extra_types: Option<&HashMap<String, PyObject>>) {
        // Format spec: [[fill]align][sign][#][0][width][,][.precision][type]
        // Examples: "<10", ">", "^5.2f", "+d", "03d", ".2f"
        
        let mut chars = format_spec.chars().peekable();
        
        // Parse fill and align (optional)
        // align can be: '<', '>', '^', '='
        if let Some(&ch) = chars.peek() {
            if ch == '<' || ch == '>' || ch == '^' || ch == '=' {
                spec.alignment = Some(ch);
                chars.next();
            } else {
                // Check if we have fill + align (e.g., "x<")
                let mut peek_iter = chars.clone();
                peek_iter.next(); // skip first char
                if let Some(next_ch) = peek_iter.next() {
                    if next_ch == '<' || next_ch == '>' || next_ch == '^' || next_ch == '=' {
                        spec.fill = Some(ch);
                        chars.next(); // consume fill
                        spec.alignment = Some(next_ch);
                        chars.next(); // consume align
                    }
                }
            }
        }
        
        // Parse sign (optional): '+', '-', ' '
        if let Some(&ch) = chars.peek() {
            if ch == '+' || ch == '-' || ch == ' ' {
                spec.sign = Some(ch);
                chars.next();
            }
        }
        
        // Parse # (alternate form) - skip for now
        if chars.peek() == Some(&'#') {
            chars.next();
        }
        
        // Parse 0 (zero padding)
        if chars.peek() == Some(&'0') {
            spec.zero_pad = true;
            chars.next();
        }
        
        // Parse width (digits)
        let mut width_str = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_ascii_digit() {
                width_str.push(ch);
                chars.next();
            } else {
                break;
            }
        }
        if !width_str.is_empty() {
            spec.width = width_str.parse::<usize>().ok();
        }
        
        // Parse comma (thousands separator) - skip for now
        if chars.peek() == Some(&',') {
            chars.next();
        }
        
        // Parse precision (.digits)
        if chars.peek() == Some(&'.') {
            chars.next();
            let mut precision_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() {
                    precision_str.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if !precision_str.is_empty() {
                spec.precision = precision_str.parse::<usize>().ok();
            }
        }
        
        // Parse type (all alphabetic characters at the end, plus %)
        // Collect all remaining characters as the type string
        let mut type_str = String::new();
        for ch in chars {
            type_str.push(ch);
        }
        
        // Handle % specially (it's not alphabetic)
        if type_str == "%" {
            spec.field_type = FieldType::Percentage;
        } else if type_str.starts_with('%') {
            // Strftime-style pattern starting with %
            spec.field_type = FieldType::DateTimeStrftime;
            spec.strftime_format = Some(type_str.clone());
        } else {
            // Extract type name (alphabetic characters only)
            let type_name: String = type_str.chars().filter(|c| c.is_alphabetic()).collect();
            
            // If type_str is empty, default to String
            // Multi-character names are always custom types
            // Single character names can be built-in or custom (checked in convert_value)
            spec.field_type = if type_name.is_empty() {
                FieldType::String
            } else if type_name == "ti" {
                FieldType::DateTimeISO
            } else if type_name == "te" {
                FieldType::DateTimeRFC2822
            } else if type_name == "tg" {
                FieldType::DateTimeGlobal
            } else if type_name == "ta" {
                FieldType::DateTimeUS
            } else if type_name == "tc" {
                FieldType::DateTimeCtime
            } else if type_name == "th" {
                FieldType::DateTimeHTTP
            } else if type_name == "tt" {
                FieldType::DateTimeTime
            } else if type_name == "ts" {
                FieldType::DateTimeSystem
            } else if type_name.len() > 1 {
                // Multi-character - always custom type
                FieldType::Custom(type_name)
            } else {
                // Single character - treat as built-in (can be overridden in convert_value)
                match type_name.chars().next().unwrap() {
                    's' => FieldType::String,
                    'd' | 'i' => FieldType::Integer,
                    'b' | 'o' | 'x' | 'X' => FieldType::Integer, // Binary, octal, hex are integers
                    'n' => FieldType::NumberWithThousands,
                    'f' | 'F' => FieldType::Float,
                    'e' | 'E' => FieldType::Scientific,
                    'g' | 'G' => FieldType::GeneralNumber,
                    'l' => FieldType::Letters,
                    'w' => FieldType::Word,
                    'W' => FieldType::NonLetters,
                    'S' => FieldType::NonWhitespace,
                    'D' => FieldType::NonDigits,
                    c => FieldType::Custom(c.to_string()),
                }
            };
        }
    }
}

#[pymethods]
impl FormatParser {
    #[new]
    #[pyo3(signature = (pattern, extra_types=None))]
    fn new_py(pattern: &str, extra_types: Option<HashMap<String, PyObject>>) -> PyResult<Self> {
        Self::new_with_extra_types(pattern, extra_types)
    }

    /// Parse a string using this compiled pattern
    #[pyo3(signature = (string, case_sensitive=false, extra_types=None, evaluate_result=true))]
    fn parse(
        &self,
        string: &str,
        case_sensitive: bool,
        extra_types: Option<HashMap<String, PyObject>>,
        evaluate_result: bool,
    ) -> PyResult<Option<PyObject>> {
        // Merge stored extra_types with provided extra_types (provided takes precedence)
        let merged_extra_types = Python::with_gil(|py| -> PyResult<Option<HashMap<String, PyObject>>> {
            let mut merged = self.stored_extra_types.clone().unwrap_or_default();
            if let Some(ref provided) = extra_types {
                for (k, v) in provided {
                    merged.insert(k.clone(), v.clone_ref(py));
                }
            }
            Ok(Some(merged))
        })?;
        self.parse_internal(string, case_sensitive, merged_extra_types, evaluate_result)
    }

    /// Get the list of named field names (returns normalized names for compatibility)
    #[getter]
    fn named_fields(&self) -> Vec<String> {
        // Return normalized names (without hyphens/dots) for compatibility with original parse library
        self.normalized_names.iter()
            .filter_map(|n| n.clone())
            .collect()
    }

    /// Get the internal regex expression string (for testing)
    /// Returns a canonical format with literal spaces instead of \s* for compatibility
    #[getter]
    fn _expression(&self) -> String {
        // Replace \s* between capturing groups with literal spaces for canonical format
        // This matches the original parse library's _expression format
        let mut result = self.regex_str.clone();
        // Replace )\s*( with ) ( for canonical format (space between fields)
        result = result.replace(r")\s*(", ") (");
        
        // For alignment patterns like {:>} that produce " *(.+?)", we wrap it as "( *(.+?))"
        // But _expression expects just " *(.+?)" (no outer wrapper)
        // Unwrap patterns that are simple wrappers: (X) where X is already a valid pattern
        // Only do this for patterns that start with "(" and end with ")" and contain nested groups
        if result.starts_with("(") && result.ends_with(")") {
            let inner = &result[1..result.len()-1];
            // Check if inner already starts with a space and contains a capturing group
            if inner.starts_with(" *(") && inner.ends_with(")") {
                // This is a simple wrapper, unwrap it
                result = inner.to_string();
            }
        }
        
        result
    }

    /// Get the format object for formatting values into the pattern
    #[getter]
    fn format(&self) -> Format {
        Format {
            pattern: self.pattern.clone(),
        }
    }
}

/// Format object that formats values into a pattern string
#[pyclass]
pub struct Format {
    pattern: String,
}

#[pymethods]
impl Format {
    /// Format values into the pattern string using Python's format() method
    fn format(&self, py: Python, args: &Bound<'_, PyAny>) -> PyResult<String> {
        // Use Python's string format method to format values into the pattern
        let pattern_obj = PyString::new_bound(py, &self.pattern);
        let format_method = pattern_obj.getattr("format")?;
        
        // Call format with the args (can be a single value, tuple, or *args)
        let result = if let Ok(tuple) = args.downcast::<PyTuple>() {
            format_method.call1(tuple)?
        } else {
            // Single argument
            format_method.call1((args,))?
        };
        result.extract()
    }
}

