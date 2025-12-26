use crate::types::{FieldSpec, FieldType};
use crate::error;
use pyo3::prelude::*;
use regex;
use std::collections::HashMap;

/// Parse a format pattern string into regex parts, field specs, and names
pub fn parse_pattern(
    pattern: &str,
    extra_types: Option<&HashMap<String, PyObject>>,
    custom_patterns: &HashMap<String, String>,
) -> PyResult<(String, String, Vec<FieldSpec>, Vec<Option<String>>, Vec<Option<String>>, HashMap<String, String>)> {
    let mut regex_parts = Vec::new();
    let mut field_specs = Vec::new();
    let mut field_names = Vec::new();  // Original names
    let mut normalized_names = Vec::new();  // Normalized for regex
    let mut name_mapping = HashMap::new();  // normalized -> original
    let mut field_name_types = HashMap::new();  // Track field name -> FieldType for validation
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
                    // But use \s+ (one or more) instead of \s* (zero or more) to ensure we consume the space
                    let escaped = if literal.trim_end() != literal {
                        // Literal ends with whitespace - replace trailing whitespace with \s+
                        // to allow one or more spaces (ensures we consume at least one space)
                        let trimmed = literal.trim_end();
                        format!("{}\\s+", regex::escape(trimmed))
                    } else {
                        regex::escape(&literal)
                    };
                    regex_parts.push(escaped);
                    literal.clear();
                }

                // Parse field specification
                let (spec, name) = parse_field(&mut chars, extra_types)?;
                
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
                        // Found a field - check if it's empty {} or has precision
                        if peek_chars.peek() == Some(&'}') {
                            // Empty field {} - non-greedy, use exact width
                            break Some(false);
                        } else {
                            // Check if the field has precision (like {:.4})
                            let mut field_chars = peek_chars.clone();
                            let mut has_precision = false;
                            while let Some(&ch) = field_chars.peek() {
                                if ch == '}' {
                                    break;
                                }
                                if ch == ':' {
                                    field_chars.next();
                                    // Check for precision after colon
                                    while let Some(&next_ch) = field_chars.peek() {
                                        if next_ch == '}' {
                                            break;
                                        }
                                        if next_ch == '.' {
                                            has_precision = true;
                                            break;
                                        }
                                        field_chars.next();
                                    }
                                    break;
                                }
                                field_chars.next();
                            }
                            // If next field has precision, it's greedy (so current should be greedy too)
                            // If next field is empty {}, it's non-greedy (so current should be exact)
                            break Some(has_precision);
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
                        if !field_types_match(existing_type, &spec.field_type) {
                            return Err(error::repeated_name_error(original_name));
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
                        let normalized = normalize_field_name(original_name, &mut name_mapping, &normalized_names);
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
                    return Err(error::pattern_error("Expected '}' after field specification"));
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
pub fn normalize_field_name(name: &str, _name_mapping: &mut HashMap<String, String>, existing_normalized: &[Option<String>]) -> String {
    // Normalize: replace hyphens and dots with underscores
    let base_normalized: String = name.chars().map(|c| if c == '-' || c == '.' { '_' } else { c }).collect();
    
    // Check for collisions with existing normalized names
    let mut normalized = base_normalized.clone();
    
    // Find the position of the first underscore to insert additional underscores there
    let underscore_pos = normalized.find('_');
    
    // Check if this exact normalized name already exists
    let mut collision_count = 0;
    while existing_normalized.iter().any(|n| n.as_ref().map(|s| s == &normalized).unwrap_or(false)) {
        collision_count += 1;
        // Insert additional underscores at the first underscore position
        // For "a_b", collisions become "a__b", "a___b", etc.
        if let Some(pos) = underscore_pos {
            let before = &base_normalized[..pos];
            let after = &base_normalized[pos + 1..];
            // Total underscores = 1 (base) + collision_count
            normalized = format!("{}{}{}", before, "_".repeat(1 + collision_count), after);
        } else {
            // No underscore found, append underscores (shouldn't happen in practice)
            normalized = format!("{}{}", base_normalized, "_".repeat(collision_count));
        }
    }
    
    normalized
}

/// Check if two field types match (for repeated name validation)
pub fn field_types_match(t1: &FieldType, t2: &FieldType) -> bool {
    use std::mem::discriminant;
    discriminant(t1) == discriminant(t2)
}

/// Parse a field name into a path (for dict-style names like "hello[world]" -> ["hello", "world"])
pub fn parse_field_path(field_name: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = String::new();
    let mut in_brackets = false;
    
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
                    if !current.is_empty() {
                        path.push(current.clone());
                        current.clear();
                    }
                    in_brackets = false;
                } else {
                    current.push(ch);
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    
    if !current.is_empty() {
        path.push(current);
    }
    
    path
}

/// Parse a single field specification from the pattern
pub fn parse_field(chars: &mut std::iter::Peekable<std::str::Chars>, extra_types: Option<&HashMap<String, PyObject>>) -> PyResult<(FieldSpec, Option<String>)> {
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
                    return Err(error::not_implemented_error("Quoted keys in field names"));
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
    parse_format_spec(&format_spec, &mut spec, extra_types);

    let name = if field_name.is_empty() {
        None
    } else {
        Some(field_name)
    };

    Ok((spec, name))
}

/// Parse format specifier string into FieldSpec
pub fn parse_format_spec(format_spec: &str, spec: &mut FieldSpec, _extra_types: Option<&HashMap<String, PyObject>>) {
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
            let type_char = type_name.chars().next().unwrap();
            spec.original_type_char = Some(type_char); // Store original type character
            match type_char {
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

