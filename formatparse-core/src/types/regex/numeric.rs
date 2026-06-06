//! Regex patterns for numeric field types.

use super::helpers::{int_width_precision_bounds, wrap_field_lookbehind};
use crate::types::definitions::{FieldSpec, FieldType};
use regex;

pub(crate) fn pattern(spec: &FieldSpec) -> String {
    match &spec.field_type {
        FieldType::Integer => {
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?", // Default: allow optional + or -
                })
                .unwrap_or(r"[+-]?"); // Default: allow optional + or -

            // Handle fill character with alignment (e.g., {:x=5d})
            // For '=' alignment, fill goes between sign and digits
            // Pattern should match: [sign][fill*][digits]
            let (fill_prefix, fill_suffix) =
                if let (Some(fill_ch), Some('=')) = (spec.fill, spec.alignment) {
                    // For '=' alignment with fill, match fill characters between sign and number
                    let fill_escaped = regex::escape(&fill_ch.to_string());
                    (format!("{}*", fill_escaped), String::new())
                } else {
                    (String::new(), String::new())
                };

            let width_precision =
                int_width_precision_bounds(spec.width, spec.precision, spec.zero_pad);

            let base_pattern = if spec.zero_pad {
                if let Some((lo, hi)) = width_precision {
                    // Both width and precision: bounded digit count (formatparse#82 / parse#107).
                    format!(
                        "{}{}{}[0-9]{{{},{}}}",
                        sign, fill_prefix, fill_suffix, lo, hi
                    )
                } else if let Some(width) = spec.width {
                    // Zero-padded: if width is specified, match 1 to width digits
                    // This allows unpadded values (e.g., '9' for {c:02d}) but rejects values exceeding width
                    format!("{}{}{}[0-9]{{1,{}}}", sign, fill_prefix, fill_suffix, width)
                } else {
                    format!("{}{}{}[0-9]+", sign, fill_prefix, fill_suffix)
                }
            } else if let Some((lo, hi)) = width_precision {
                let q_hex = format!("[0-9a-fA-F]{{{},{}}}", lo, hi);
                let q_oct = format!("[0-7]{{{},{}}}", lo, hi);
                let q_bin = format!("[01]{{{},{}}}", lo, hi);
                let q_dec = format!("[0-9]{{{},{}}}", lo, hi);
                match spec.original_type_char {
                    Some('x') | Some('X') => {
                        // Hex: bounded digit run after optional 0x (parse#107 / #82).
                        format!(
                            "{}{}{}(?:0[xX]{}|{})",
                            sign, fill_prefix, fill_suffix, q_hex, q_hex
                        )
                    }
                    Some('o') => {
                        format!(
                            "{}{}{}(?:0[oO]{}|{})",
                            sign, fill_prefix, fill_suffix, q_oct, q_oct
                        )
                    }
                    Some('b') => {
                        format!(
                            "{}{}{}(?:0[bB]{}|{})",
                            sign, fill_prefix, fill_suffix, q_bin, q_bin
                        )
                    }
                    _ => {
                        // Decimal `d` / `i`: optional leading whitespace before digits (#81);
                        // each radix branch uses the same inclusive min/max digit count.
                        format!(
                            "{}{}{}(?:0[xX]{}|0[oO]{}|0[bB]{}|[ \\t]*{})",
                            sign, fill_prefix, fill_suffix, q_hex, q_oct, q_bin, q_dec
                        )
                    }
                }
            } else {
                // Check original type to determine what digits to match
                match spec.original_type_char {
                    Some('x') | Some('X') => {
                        // Hex: match hex digits with or without 0x prefix
                        format!(
                            "{}{}{}(?:0[xX][0-9a-fA-F]+|[0-9a-fA-F]+)",
                            sign, fill_prefix, fill_suffix
                        )
                    }
                    Some('o') => {
                        // Octal: match octal digits with or without 0o prefix
                        format!(
                            "{}{}{}(?:0[oO][0-7]+|[0-7]+)",
                            sign, fill_prefix, fill_suffix
                        )
                    }
                    Some('b') => {
                        // Binary: match binary digits with or without 0b prefix
                        format!("{}{}{}(?:0[bB][01]+|[01]+)", sign, fill_prefix, fill_suffix)
                    }
                    _ => {
                        // Decimal: optional leading whitespace before digits (parse#133 / #81)
                        // so values like "    0" match like str.format padding, without widening
                        // the 0x/0o/0b branches.
                        format!(
                            "{}{}{}(?:0[xX][0-9a-fA-F]+|0[oO][0-7]+|0[bB][01]+|[ \\t]*[0-9]+)",
                            sign, fill_prefix, fill_suffix
                        )
                    }
                }
            };

            wrap_field_lookbehind(spec, base_pattern)
        }
        FieldType::Float => {
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?", // Default: allow optional + or -
                })
                .unwrap_or(r"[+-]?"); // Default: allow optional + or -

            // For floats, precision affects how we match
            // Width is mainly for formatting, but we need to handle it in parsing
            // When width is specified, there may be leading/trailing spaces
            let inner = if let Some(prec) = spec.precision {
                // Precision specified — digits after the decimal when a decimal point appears.
                // For prec == 0, str.format often emits no '.' (e.g. "{:02.0f}" → "20"); accept
                // integer-looking text as well as "12." / ".5" forms (issue #84 / parse#159).
                if prec == 0 {
                    let width_prefix = if spec.width.is_some() { r"\s*" } else { "" };
                    format!(
                        r"{}{}(?:\d+(?:\.0*)?|\d*\.\d{{{}}}|\.\d{{{}}})(?:[eE][+-]?\d+)?",
                        width_prefix, sign, prec, prec
                    )
                } else if spec.width.is_some() {
                    // Width specified - allow optional leading spaces
                    format!(
                        r"\s*{}(?:\d*\.\d{{{}}}|\.\d{{{}}})(?:[eE][+-]?\d+)?",
                        sign, prec, prec
                    )
                } else {
                    format!(
                        r"{}(?:\d*\.\d{{{}}}|\.\d{{{}}})(?:[eE][+-]?\d+)?",
                        sign, prec, prec
                    )
                }
            } else {
                // Float must have a decimal point (not just an integer)
                // Allow: 12.34, .34, 12., or scientific notation with decimal
                format!(r"{}(?:\d+\.\d+|\.\d+|\d+\.)(?:[eE][+-]?\d+)?", sign)
            };
            wrap_field_lookbehind(spec, inner)
        }
        FieldType::Letters => r"[a-zA-Z]+".to_string(),
        FieldType::Word => r"\w+".to_string(),
        FieldType::NonLetters => r"[^a-zA-Z]+".to_string(),
        FieldType::NonWhitespace => r"\S+".to_string(),
        FieldType::NonDigits => r"[^0-9]+".to_string(),
        FieldType::NumberWithThousands => {
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => r"[+-]?", // Default: allow optional + or -
                })
                .unwrap_or(r"[+-]?"); // Default: allow optional + or -
                                      // Match numbers with thousands separators (comma or dot)
                                      // Pattern: either number with valid thousands separators (1,234,567 or 1.234.567)
                                      // or plain number without separators
                                      // The regex matches the pattern, validation happens in conversion
            format!(r"{}(?:\d{{1,3}}(?:[.,]\d{{3}})*|\d+)", sign)
        }
        FieldType::Scientific => {
            // Scientific notation: matches floats with e/E exponent, or nan/inf
            // Pattern matches original parse library exactly: \d*\.\d+[eE][-+]?\d+|nan|NAN|[-+]?inf|[-+]?INF
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                })
                .unwrap_or("-?");
            // Sign applies to numeric part; nan/inf have their own optional signs in the pattern
            format!(r"{}\d*\.\d+[eE][-+]?\d+|nan|NAN|[-+]?inf|[-+]?INF", sign)
        }
        FieldType::GeneralNumber => {
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                })
                .unwrap_or("-?");
            // General number: can be int or float or scientific, or nan/inf
            format!(
                r"{}(?:\d+\.\d+|\.\d+|\d+\.|\d+)(?:[eE][+-]?\d+)?|nan|NAN|[-+]?inf|[-+]?INF",
                sign
            )
        }
        FieldType::Percentage => {
            let sign = spec
                .sign
                .as_ref()
                .map(|s| match s {
                    '+' => r"\+?",
                    '-' => "-?",
                    ' ' => r"[- ]?",
                    _ => "-?",
                })
                .unwrap_or("-?");
            // Percentage: number followed by %
            format!(r"{}(?:\d+\.\d+|\.\d+|\d+)%", sign)
        }
        _ => unreachable!("pattern() called with wrong field type"),
    }
}
