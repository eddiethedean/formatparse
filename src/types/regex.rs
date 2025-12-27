use super::definitions::{FieldSpec, FieldType};
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

impl FieldSpec {
    pub fn to_regex_pattern(&self, custom_patterns: &HashMap<String, String>, next_field_is_greedy: Option<bool>) -> String {
        let base_pattern = match &self.field_type {
            FieldType::String => {
                // Handle alignment and width for strings
                if let Some(prec) = self.precision {
                    // Precision specified: match exactly 'precision' characters
                    // If alignment is also specified, allow fill characters in appropriate positions
                    if let Some(align) = self.alignment {
                        let fill_ch = self.fill.unwrap_or(' ');
                        let fill_escaped = regex::escape(&fill_ch.to_string());
                        match align {
                            '<' => {
                                // Left-aligned: content (precision chars) + optional trailing fill chars
                                format!(".{{{}}}(?:{}*)", prec, fill_escaped)
                            },
                            '>' => {
                                // Right-aligned: optional leading fill chars + content (precision chars)
                                format!("(?:{}*).{{{}}}", fill_escaped, prec)
                            },
                            '^' => {
                                // Center-aligned: optional leading fill + content + optional trailing fill
                                format!("(?:{}*).{{{}}}(?:{}*)", fill_escaped, prec, fill_escaped)
                            },
                            _ => format!(".{{{}}}", prec),
                        }
                    } else {
                        // Precision only, no alignment: match exactly 'precision' characters
                        format!(".{{{}}}", prec)
                    }
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
                    // Zero-padded: if width is specified, match 1 to width digits
                    // This allows unpadded values (e.g., '9' for {c:02d}) but rejects values exceeding width
                    if let Some(width) = self.width {
                        format!("{}{}{}[0-9]{{1,{}}}", sign, fill_prefix, fill_suffix, width)
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
}

