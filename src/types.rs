use pyo3::prelude::*;
use regex;
use std::collections::HashMap;

/// Convert strftime format string to regex pattern
pub fn strftime_to_regex(format_str: &str) -> String {
    let mut regex_parts = Vec::new();
    let mut chars = format_str.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let Some(next_ch) = chars.next() {
                let regex_part = match next_ch {
                    'Y' => r"\d{4}",           // Year with century
                    'y' => r"\d{2}",           // Year without century
                    'm' => r"\d{1,2}",         // Month (1-12 or 01-12) - flexible
                    'd' => r"\d{1,2}",         // Day (1-31 or 01-31) - flexible
                    'H' => r"\d{1,2}",         // Hour (0-23 or 00-23) - flexible
                    'M' => r"\d{1,2}",         // Minute (0-59 or 00-59) - flexible
                    'S' => r"\d{1,2}",         // Second (0-59 or 00-59) - flexible
                    'b' | 'h' => r"[A-Za-z]{3}", // Abbreviated month name
                    'B' => r"[A-Za-z]+",       // Full month name
                    'a' => r"[A-Za-z]{3}",     // Abbreviated weekday
                    'A' => r"[A-Za-z]+",       // Full weekday
                    'w' => r"\d",              // Weekday as decimal (0=Sunday)
                    'j' => r"\d{1,3}",         // Day of year (1-366, flexible padding)
                    'U' | 'W' => r"\d{2}",     // Week number
                    'c' => r".+",              // Date and time representation (locale dependent)
                    'x' => r".+",              // Date representation (locale dependent)
                    'X' => r".+",              // Time representation (locale dependent)
                    '%' => "%",                // Literal %
                    _ => ".+?",                // Unknown directive - match anything
                };
                regex_parts.push(regex_part.to_string());
            }
        } else {
            // Escape special regex characters for literal text
            regex_parts.push(regex::escape(&ch.to_string()));
        }
    }
    
    regex_parts.join("")
}

#[derive(Debug, Clone)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Letters,      // 'l' - matches only letters
    Word,         // 'w' - matches word characters (letters, digits, underscore)
    NonLetters,   // 'W' - matches non-letter characters
    NonWhitespace,// 'S' - matches non-whitespace characters
    NonDigits,    // 'D' - matches non-digit characters
    NumberWithThousands, // 'n' - numbers with thousands separators
    Scientific,   // 'e' - scientific notation
    GeneralNumber,// 'g' - general number (int or float)
    Percentage,   // '%' - percentage
    DateTimeISO,  // 'ti' - ISO 8601 datetime format
    DateTimeRFC2822, // 'te' - RFC2822 email format
    DateTimeGlobal, // 'tg' - Global (day/month) format
    DateTimeUS,   // 'ta' - US (month/day) format
    DateTimeCtime, // 'tc' - ctime() format
    DateTimeHTTP, // 'th' - HTTP log format
    DateTimeTime, // 'tt' - Time format
    DateTimeSystem, // 'ts' - Linux system log format
    DateTimeStrftime, // For %Y-%m-%d style patterns
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub name: Option<String>,
    pub field_type: FieldType,
    pub width: Option<usize>,
    pub precision: Option<usize>,
    pub alignment: Option<char>, // '<', '>', '^', '='
    pub sign: Option<char>,      // '+', '-', ' '
    pub fill: Option<char>,
    pub zero_pad: bool,
    pub strftime_format: Option<String>, // For strftime-style patterns
    pub original_type_char: Option<char>, // Original type character (e.g., 'b', 'o', 'x' for binary/octal/hex)
}

impl Default for FieldSpec {
    fn default() -> Self {
        Self {
            name: None,
            field_type: FieldType::String,
            width: None,
            precision: None,
            alignment: None,
            sign: None,
            fill: None,
            zero_pad: false,
            strftime_format: None,
            original_type_char: None,
        }
    }
}

impl FieldSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_regex_pattern(&self, custom_patterns: &HashMap<String, String>, next_field_is_greedy: Option<bool>) -> String {
        let base_pattern = match &self.field_type {
            FieldType::String => {
                // Handle alignment and width for strings
                if let Some(prec) = self.precision {
                    // Precision specified: match exactly 'precision' characters
                    format!(".{{{}}}", prec)
                } else if let Some(width) = self.width {
                    // Width only (no precision): 
                    // - If there's a next field with precision (like {:.4}), use greedy (at least width)
                    // - If there's a next field without precision (like {}), use exact width
                    // - If it's the last field, use greedy (at least width)
                    match next_field_is_greedy {
                        Some(false) => format!(".{{{}}}", width),  // Exact when followed by non-greedy field
                        _ => format!(".{{{},}}", width),  // Greedy when followed by greedy field or last field
                    }
                } else if self.alignment.is_some() {
                    // Alignment specified but no width - match with optional surrounding whitespace
                    // For alignment, we want to capture only the text value (without padding spaces)
                    // The padding spaces are part of the alignment formatting, not the value
                    match self.alignment {
                        // Left: capture text, then allow trailing spaces (non-capturing)
                        Some('<') => r"([^\{\}\s]+(?:\s+[^\{\}\s]+)*?)(?:\s*)".to_string(),
                        // Right: allow leading spaces (non-capturing), then capture text
                        // For _expression compatibility, use " *(.+?)" format (leading spaces, then capture)
                        Some('>') => r" *(.+?)".to_string(),
                        // Center: allow spaces on both sides (non-capturing), capture text in middle
                        Some('^') => r"(?:\s*)([^\{\}\s]+(?:\s+[^\{\}\s]+)*?)(?:\s*)".to_string(),
                        _ => r"[^\{\}]+?".to_string(),
                    }
                } else {
                    // For empty {} fields, match any characters including newlines (non-greedy)
                    // Use .+? to match the original parse library behavior
                    r".+?".to_string()
                }
            }
            FieldType::Integer => {
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?",  // Default: allow optional + or -
                }).unwrap_or(r"[+-]?");  // Default: allow optional + or -
                
                // Handle fill character with alignment (e.g., {:x=5d})
                // For '=' alignment, fill goes between sign and digits
                // Pattern should match: [sign][fill*][digits]
                let (fill_prefix, fill_suffix) = if let (Some(fill_ch), Some('=')) = (self.fill, self.alignment) {
                    // For '=' alignment with fill, match fill characters between sign and number
                    let fill_escaped = regex::escape(&fill_ch.to_string());
                    (format!("{}*", fill_escaped), String::new())
                } else {
                    (String::new(), String::new())
                };
                
                let base_pattern = if self.zero_pad {
                    // Zero-padded: if width is specified, match exactly that many digits
                    if let Some(width) = self.width {
                        format!("{}{}{}[0-9]{{{}}}", sign, fill_prefix, fill_suffix, width)
                    } else {
                        format!("{}{}{}[0-9]+", sign, fill_prefix, fill_suffix)
                    }
                } else {
                    // Check original type to determine what digits to match
                    match self.original_type_char {
                        Some('x') | Some('X') => {
                            // Hex: match hex digits with or without 0x prefix
                            format!("{}{}{}(?:0[xX][0-9a-fA-F]+|[0-9a-fA-F]+)", sign, fill_prefix, fill_suffix)
                        },
                        Some('o') => {
                            // Octal: match octal digits with or without 0o prefix
                            format!("{}{}{}(?:0[oO][0-7]+|[0-7]+)", sign, fill_prefix, fill_suffix)
                        },
                        Some('b') => {
                            // Binary: match binary digits with or without 0b prefix
                            format!("{}{}{}(?:0[bB][01]+|[01]+)", sign, fill_prefix, fill_suffix)
                        },
                        _ => {
                            // Decimal: match decimal digits, or hex/octal/binary with prefix
                            format!("{}{}{}(?:0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][01]+|[0-9]+)", sign, fill_prefix, fill_suffix)
                        }
                    }
                };
                
                // Always allow optional leading spaces (the pattern matching handles this naturally
                // because the surrounding literal text will consume spaces, but we need to be explicit
                // for cases where spaces appear between the literal and the number)
                // Actually, the spaces are part of the literal match between "a " and "12", so we don't need \s* here
                // The issue is that "a  12 b" has two spaces, and our literal "a " only matches one
                // So the regex needs to match "a " + optional spaces + number + " b"
                // But this is handled at the pattern level, not here. The width specification adds \s* explicitly.
                // For now, keep the original behavior - width adds \s*, no width doesn't.
                base_pattern
            }
            FieldType::Float => {
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?",  // Default: allow optional + or -
                }).unwrap_or(r"[+-]?");  // Default: allow optional + or -
                
                // For floats, precision affects how we match
                // Width is mainly for formatting, but we need to handle it in parsing
                // When width is specified, there may be leading/trailing spaces
                if let Some(prec) = self.precision {
                    // Precision specified - must match exact precision after decimal
                    // Allow no leading zero before decimal (e.g., ".31415")
                    // Also allow negative sign
                    if self.width.is_some() {
                        // Width specified - allow optional leading spaces
                        format!(r"\s*{}(?:\d*\.\d{{{}}}|\.\d{{{}}})(?:[eE][+-]?\d+)?", sign, prec, prec)
                    } else {
                        format!(r"{}(?:\d*\.\d{{{}}}|\.\d{{{}}})(?:[eE][+-]?\d+)?", sign, prec, prec)
                    }
                } else {
                    // Float must have a decimal point (not just an integer)
                    // Allow: 12.34, .34, 12., or scientific notation with decimal
                    format!(r"{}(?:\d+\.\d+|\.\d+|\d+\.)(?:[eE][+-]?\d+)?", sign)
                }
            }
            FieldType::Letters => r"[a-zA-Z]+".to_string(),
            FieldType::Word => r"\w+".to_string(),
            FieldType::NonLetters => r"[^a-zA-Z]+".to_string(),
            FieldType::NonWhitespace => r"\S+".to_string(),
            FieldType::NonDigits => r"[^0-9]+".to_string(),
            FieldType::NumberWithThousands => {
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?",  // Default: allow optional + or -
                }).unwrap_or(r"[+-]?");  // Default: allow optional + or -
                // Match numbers with thousands separators (comma or dot)
                // Pattern: either number with valid thousands separators (1,234,567 or 1.234.567)
                // or plain number without separators
                // The regex matches the pattern, validation happens in conversion
                format!(r"{}(?:\d{{1,3}}(?:[.,]\d{{3}})*|\d+)", sign)
            },
            FieldType::Scientific => {
                // Scientific notation: matches floats with e/E exponent, or nan/inf
                // Pattern matches original parse library exactly: \d*\.\d+[eE][-+]?\d+|nan|NAN|[-+]?inf|[-+]?INF
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                }).unwrap_or("-?");
                // Sign applies to numeric part; nan/inf have their own optional signs in the pattern
                format!(r"{}\d*\.\d+[eE][-+]?\d+|nan|NAN|[-+]?inf|[-+]?INF", sign)
            },
            FieldType::GeneralNumber => {
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                }).unwrap_or("-?");
                // General number: can be int or float or scientific, or nan/inf
                format!(r"{}(?:\d+\.\d+|\.\d+|\d+\.|\d+)(?:[eE][+-]?\d+)?|nan|NAN|[-+]?inf|[-+]?INF", sign)
            },
            FieldType::Percentage => {
                let sign = self.sign.as_ref().map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                }).unwrap_or("-?");
                // Percentage: number followed by %
                format!(r"{}(?:\d+\.\d+|\.\d+|\d+)%", sign)
            },
            FieldType::DateTimeISO => {
                // ISO 8601 format: YYYY-MM-DD, YYYY-MM-DDTHH:MM, YYYY-MM-DDTHH:MM:SS, etc.
                // Supports various separators and timezone formats (with optional space before timezone)
                r"\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?)?(?:\s*[Zz]|\s*[+-]\d{2}:?\d{2}|\s*[+-]\d{4})?".to_string()
            },
            FieldType::DateTimeRFC2822 => {
                // RFC2822: Mon, 21 Nov 2011 10:21:36 +1000 or +10:00 (optional weekday)
                r"(?:(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+)?\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{4}\s+\d{2}:\d{2}:\d{2}\s+[+-]\d{2}:?\d{2,4}".to_string()
            },
            FieldType::DateTimeGlobal => {
                // Global format: 21/11/2011 10:21:36 AM +1000 or 21-Nov-2011 10:21:36 AM +1:00
                r"\d{1,2}[-/](?:\d{1,2}|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|June|July|August|September|October|November|December)[-/]\d{4}(?:\s+\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{1,2}:?\d{2,4})?)?".to_string()
            },
            FieldType::DateTimeUS => {
                // US format: 11/21/2011 10:21:36 AM +1000 or 11-Nov-2011 10:21:36 AM +1000 or Nov-21-2011 10:21:36 AM +1000
                r"(?:\d{1,2}|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|June|July|August|September|October|November|December)[-/]\d{1,2}[-/]\d{4}(?:\s+\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{2}:?\d{2,4})?)?".to_string()
            },
            FieldType::DateTimeCtime => {
                // ctime format: Mon Nov 21 10:21:36 2011
                r"(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\d{4}".to_string()
            },
            FieldType::DateTimeHTTP => {
                // HTTP log format: 21/Nov/2011:00:07:11 +0000
                r"\d{2}/[A-Za-z]{3}/\d{4}:\d{2}:\d{2}:\d{2}\s+[+-]\d{2}:?\d{2,4}".to_string()
            },
            FieldType::DateTimeTime => {
                // Time format: 10:21:36 PM -5:30
                r"\d{1,2}:\d{2}(?::\d{2})?(?:\s+[AP]M)?(?:\s+[+-]\d{1,2}:?\d{2,4})?".to_string()
            },
            FieldType::DateTimeSystem => {
                // Linux system log format: Nov 21 10:21:36
                r"[A-Za-z]{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}".to_string()
            },
            FieldType::DateTimeStrftime => {
                // Convert strftime format to regex pattern
                if let Some(ref fmt) = self.strftime_format {
                    strftime_to_regex(fmt)
                } else {
                    r".+?".to_string()
                }
            },
            FieldType::Boolean => "true|false|True|False|TRUE|FALSE|1|0|yes|no|Yes|No|YES|NO|on|off|On|Off|ON|OFF".to_string(),
            FieldType::Custom(name) => {
                custom_patterns.get(name)
                    .cloned()
                    .unwrap_or_else(|| r"\S+".to_string())  // Default to non-whitespace for custom types without patterns
            }
        };

        base_pattern
    }

    pub fn convert_value(&self, value: &str, py: Python, custom_converters: &HashMap<String, PyObject>) -> PyResult<PyObject> {
        // Check if this type has a custom converter (even if it's a built-in type name)
        let type_name = match &self.field_type {
            FieldType::Custom(name) => name.clone(),
            FieldType::String => "s".to_string(),
            FieldType::Integer => "d".to_string(),  // Use 'd' as the canonical integer type name
            FieldType::Float => "f".to_string(),
            FieldType::Boolean => "b".to_string(),
            FieldType::Letters => "l".to_string(),
            FieldType::Word => "w".to_string(),
            FieldType::NonLetters => "W".to_string(),
            FieldType::NonWhitespace => "S".to_string(),
            FieldType::NonDigits => "D".to_string(),
            FieldType::NumberWithThousands => "n".to_string(),
            FieldType::Scientific => "e".to_string(),
            FieldType::GeneralNumber => "g".to_string(),
            FieldType::Percentage => "%".to_string(),
            FieldType::DateTimeISO => "ti".to_string(),
            FieldType::DateTimeRFC2822 => "te".to_string(),
            FieldType::DateTimeGlobal => "tg".to_string(),
            FieldType::DateTimeUS => "ta".to_string(),
            FieldType::DateTimeCtime => "tc".to_string(),
            FieldType::DateTimeHTTP => "th".to_string(),
            FieldType::DateTimeTime => "tt".to_string(),
            FieldType::DateTimeSystem => "ts".to_string(),
            FieldType::DateTimeStrftime => "strftime".to_string(),
        };
        
        // If there's a custom converter for this type name, use it instead of built-in
        if custom_converters.contains_key(&type_name) {
            if let Some(converter) = custom_converters.get(&type_name) {
                let args = (value,);
                return converter.call1(py, args);
            }
        }
        
        // Use built-in conversion
        match &self.field_type {
            FieldType::String => {
                // Strip whitespace based on alignment
                let trimmed = match self.alignment {
                    Some('<') => value.trim_end(),  // Left-aligned: strip trailing spaces
                    Some('>') => value.trim_start(), // Right-aligned: strip leading spaces
                    Some('^') => value.trim(),      // Center-aligned: strip both
                    _ => value,                     // No alignment: keep as-is
                };
                Ok(trimmed.to_object(py))
            },
            FieldType::Integer => {
                // Strip whitespace before parsing (width may include spaces)
                let mut trimmed_str = value.trim().to_string();
                
                // Strip fill characters if alignment is '=' with fill
                // Fill characters appear between sign and digits (e.g., "-xxx12" or "+xxx12")
                // But NOT between sign and prefix (e.g., "-0o10" should not strip '0')
                if let (Some(fill_ch), Some('=')) = (self.fill, self.alignment) {
                    // Check if there's a sign first
                    if trimmed_str.starts_with('-') || trimmed_str.starts_with('+') {
                        // Keep the sign, strip fill chars after it but before the number part
                        let sign_char = &trimmed_str[..1];
                        let rest = &trimmed_str[1..];
                        // Only strip fill if it's not part of a prefix (0x, 0o, 0b)
                        if rest.starts_with("0x") || rest.starts_with("0X") || 
                           rest.starts_with("0o") || rest.starts_with("0O") ||
                           rest.starts_with("0b") || rest.starts_with("0B") {
                            // Has prefix, don't strip (fill shouldn't appear here)
                            // Actually, fill can appear: "-xxx0o10" -> strip xxx
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        } else {
                            // No prefix, strip fill chars
                            let rest_trimmed = rest.trim_start_matches(fill_ch);
                            trimmed_str = format!("{}{}", sign_char, rest_trimmed);
                        }
                    } else {
                        // No sign, just strip leading fill chars
                        trimmed_str = trimmed_str.trim_start_matches(fill_ch).to_string();
                    }
                }
                
                let trimmed = trimmed_str.as_str();
                // Handle negative numbers with prefixes (e.g., "-0o10")
                let (is_negative, num_str) = if trimmed.starts_with('-') {
                    (true, &trimmed[1..])
                } else if trimmed.starts_with('+') {
                    (false, &trimmed[1..])
                } else {
                    (false, trimmed)
                };
                
                let v = if num_str.starts_with("0x") || num_str.starts_with("0X") {
                    i64::from_str_radix(&num_str[2..], 16).map(|n| if is_negative { -n } else { n })
                } else if num_str.starts_with("0o") || num_str.starts_with("0O") {
                    i64::from_str_radix(&num_str[2..], 8).map(|n| if is_negative { -n } else { n })
                } else if num_str.starts_with("0b") || num_str.starts_with("0B") {
                    // Check if type is 'x' - if so, "0B" should be parsed as hex (0xB)
                    let result = if self.original_type_char == Some('x') || self.original_type_char == Some('X') {
                        // For hex type, "0B" means 0xB (hex), not binary
                        if num_str == "0B" || num_str == "0b" {
                            i64::from_str_radix("B", 16)
                        } else if num_str.len() > 2 {
                            // "0B1" should be parsed as "B1" in hex
                            i64::from_str_radix(&num_str[1..], 16)
                        } else {
                            i64::from_str_radix(&num_str[2..], 2)
                        }
                    } else {
                        i64::from_str_radix(&num_str[2..], 2)
                    };
                    result.map(|n| if is_negative { -n } else { n })
                } else {
                    // Check original type character to determine base if no prefix
                    let result = match self.original_type_char {
                        Some('b') => i64::from_str_radix(num_str, 2), // Binary without 0b prefix
                        Some('o') => i64::from_str_radix(num_str, 8), // Octal without 0o prefix
                        Some('x') | Some('X') => i64::from_str_radix(num_str, 16), // Hex without 0x prefix
                        _ => num_str.parse::<i64>(), // Decimal
                    };
                    result.map(|n| if is_negative { -n } else { n })
                };
                match v {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid integer: {}", value))),
                }
            }
            FieldType::Float => {
                // Strip whitespace before parsing (width may include spaces)
                let trimmed = value.trim();
                match trimmed.parse::<f64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid float: {}", value))),
                }
            }
            FieldType::Boolean => {
                let lower = value.to_lowercase();
                let b = matches!(lower.as_str(), "true" | "1" | "yes" | "on");
                Ok(b.to_object(py))
            }
            FieldType::Letters => Ok(value.to_object(py)),  // Letters are just strings
            FieldType::Word => Ok(value.to_object(py)),     // Words are just strings
            FieldType::NonLetters => Ok(value.to_object(py)), // Non-letters are just strings
            FieldType::NonWhitespace => Ok(value.to_object(py)), // Non-whitespace are just strings
            FieldType::NonDigits => Ok(value.to_object(py)), // Non-digits are just strings
            FieldType::NumberWithThousands => {
                // Strip thousands separators (comma or dot) and parse as integer
                let trimmed = value.trim();
                let cleaned = trimmed.replace(",", "").replace(".", "");
                match cleaned.parse::<i64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid number with thousands: {}", value))),
                }
            },
            FieldType::Scientific => {
                // Parse as float (supports scientific notation)
                let trimmed = value.trim();
                match trimmed.parse::<f64>() {
                    Ok(n) => Ok(n.to_object(py)),
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid scientific notation: {}", value))),
                }
            },
            FieldType::GeneralNumber => {
                // Parse as int if possible, otherwise float, or nan/inf
                let trimmed = value.trim();
                let lower = trimmed.to_lowercase();
                // Check for nan/inf first
                if lower == "nan" {
                    Ok(f64::NAN.to_object(py))
                } else if lower == "inf" || lower == "+inf" {
                    Ok(f64::INFINITY.to_object(py))
                } else if lower == "-inf" {
                    Ok(f64::NEG_INFINITY.to_object(py))
                } else {
                    // Try int first
                    if let Ok(n) = trimmed.parse::<i64>() {
                        Ok(n.to_object(py))
                    } else if let Ok(n) = trimmed.parse::<f64>() {
                        Ok(n.to_object(py))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid number: {}", value)))
                    }
                }
            },
            FieldType::Percentage => {
                // Parse number, remove %, divide by 100
                let trimmed = value.trim();
                let num_str = trimmed.trim_end_matches('%');
                match num_str.parse::<f64>() {
                    Ok(n) => Ok((n / 100.0).to_object(py)),
                    Err(_) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid percentage: {}", value))),
                }
            },
            FieldType::DateTimeISO => {
                crate::datetime_parse::parse_iso_datetime(py, value)
            },
            FieldType::DateTimeRFC2822 => {
                crate::datetime_parse::parse_rfc2822_datetime(py, value)
            },
            FieldType::DateTimeGlobal => {
                crate::datetime_parse::parse_global_datetime(py, value)
            },
            FieldType::DateTimeUS => {
                crate::datetime_parse::parse_us_datetime(py, value)
            },
            FieldType::DateTimeCtime => {
                crate::datetime_parse::parse_ctime_datetime(py, value)
            },
            FieldType::DateTimeHTTP => {
                crate::datetime_parse::parse_http_datetime(py, value)
            },
            FieldType::DateTimeTime => {
                crate::datetime_parse::parse_time(py, value)
            },
            FieldType::DateTimeSystem => {
                crate::datetime_parse::parse_system_datetime(py, value)
            },
            FieldType::DateTimeStrftime => {
                if let Some(ref fmt) = self.strftime_format {
                    crate::datetime_parse::parse_strftime_datetime(py, value, fmt)
                } else {
                    Ok(value.to_object(py))
                }
            },
            FieldType::Custom(_) => {
                // Already handled above
                Ok(value.to_object(py))
            }
        }
    }
}


