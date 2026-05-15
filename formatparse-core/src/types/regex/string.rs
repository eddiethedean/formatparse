//! Regex patterns for string field types.

use super::helpers::STRING_WIDTH_PRECISION_CHAR;
use crate::types::definitions::{FieldSpec, FieldType};
use regex;

pub(crate) fn pattern(
    spec: &FieldSpec,
    next_field_is_greedy: Option<bool>,
    allow_empty_delimited: bool,
) -> String {
    let fmt_char = STRING_WIDTH_PRECISION_CHAR;
    match &spec.field_type {
        FieldType::String => {
            // Handle alignment and width for strings
            if let Some(prec) = spec.precision {
                // Precision specified: match exactly 'precision' characters
                // If alignment is also specified, allow fill characters in appropriate positions
                if let Some(align) = spec.alignment {
                    let fill_ch = spec.fill.unwrap_or(' ');
                    let fill_escaped = regex::escape(&fill_ch.to_string());
                    // When width == precision, the formatted field is exactly `prec` characters
                    // wide; optional fill segments would let the capture extend past that width
                    // and consume text meant for a following literal (GitHub issue #97).
                    let width_equals_precision = spec.width == Some(prec);
                    match align {
                        '<' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", fmt_char, prec)
                            } else {
                                // Left-aligned: content (precision chars) + optional trailing fill
                                format!("{}{{{}}}(?:{}*)", fmt_char, prec, fill_escaped)
                            }
                        }
                        '>' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", fmt_char, prec)
                            } else {
                                // Right-aligned: optional leading fill + exactly `prec` characters.
                                // Leading fill must be non-greedy so we do not consume the entire slice
                                // before `.{{prec}}` when another field follows (issue #88 / parse#218).
                                format!("(?:{}*?){}{{{}}}", fill_escaped, fmt_char, prec)
                            }
                        }
                        '^' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", fmt_char, prec)
                            } else {
                                // Center-aligned: optional leading fill + content + optional trailing fill.
                                // Non-greedy leading fill for the same boundary reason as `>`.
                                format!(
                                    "(?:{}*?){}{{{}}}(?:{}*)",
                                    fill_escaped, fmt_char, prec, fill_escaped
                                )
                            }
                        }
                        _ => format!("{}{{{}}}", fmt_char, prec),
                    }
                } else {
                    // Precision only, no alignment: match exactly 'precision' characters
                    format!("{}{{{}}}", fmt_char, prec)
                }
            } else if let Some(width) = spec.width {
                // Width only (no precision):
                // - If there's a next field with precision (like {:.4}), use greedy (at least width)
                // - If there's a next field without precision (like {}), use exact width
                // - If it's the last field, use greedy (at least width)
                match next_field_is_greedy {
                    Some(false) => format!("{}{{{}}}", fmt_char, width), // Exact when followed by non-greedy field
                    _ => format!("{}{{{},}}", fmt_char, width), // Greedy when followed by greedy field or last field
                }
            } else if spec.alignment.is_some() {
                // Alignment specified but no width - match with optional surrounding whitespace
                // For alignment, we want to capture only the text value (without padding spaces)
                // The padding spaces are part of the alignment formatting, not the value
                match spec.alignment {
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
                // For empty {} fields, match any characters including newlines (non-greedy).
                // Delimited default strings may match empty (issue #83); otherwise require
                // at least one character like the original parse library.
                if allow_empty_delimited {
                    r".*?".to_string()
                } else {
                    r".+?".to_string()
                }
            }
        }
        FieldType::Multiline | FieldType::IndentBlock => {
            // Same layout as strings, but ``.`` cannot span newlines in Rust regex; use ``[\s\S]``.
            // ``IndentBlock`` (:blk) uses the same fragment as ``Multiline``; dedent is applied after capture.
            const ML: &str = "[\\s\\S]";
            if let Some(prec) = spec.precision {
                if let Some(align) = spec.alignment {
                    let fill_ch = spec.fill.unwrap_or(' ');
                    let fill_escaped = regex::escape(&fill_ch.to_string());
                    let width_equals_precision = spec.width == Some(prec);
                    match align {
                        '<' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", ML, prec)
                            } else {
                                format!("{}{{{}}}(?:{}*)", ML, prec, fill_escaped)
                            }
                        }
                        '>' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", ML, prec)
                            } else {
                                format!("(?:{}*?){}{{{}}}", fill_escaped, ML, prec)
                            }
                        }
                        '^' => {
                            if width_equals_precision {
                                format!("{}{{{}}}", ML, prec)
                            } else {
                                format!(
                                    "(?:{}*?){}{{{}}}(?:{}*)",
                                    fill_escaped, ML, prec, fill_escaped
                                )
                            }
                        }
                        _ => format!("{}{{{}}}", ML, prec),
                    }
                } else {
                    format!("{}{{{}}}", ML, prec)
                }
            } else if let Some(width) = spec.width {
                match next_field_is_greedy {
                    Some(false) => format!("{}{{{}}}", ML, width),
                    _ => format!("{}{{{},}}", ML, width),
                }
            } else if spec.alignment.is_some() {
                match spec.alignment {
                    Some('<') => format!("({}+?)(?:\\s*)", ML),
                    Some('>') => format!(" *({}+?)", ML),
                    Some('^') => format!("(?:\\s*)({}+?)(?:\\s*)", ML),
                    _ => format!("{}+?", ML),
                }
            } else {
                r"[\s\S]+?".to_string()
            }
        }
        _ => unreachable!("pattern() called with wrong field type"),
    }
}
