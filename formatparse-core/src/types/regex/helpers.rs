use crate::types::definitions::FieldSpec;
use regex;

/// When [`FieldSpec::width`] and [`FieldSpec::precision`] are both set on integer fields
/// (`d` / `i` / `x` / `X` / `o` / `b`), the significant digit run uses a bounded repetition
/// `{min_digits,max_digits}` (inclusive).
pub(crate) fn int_width_precision_bounds(
    width: Option<usize>,
    precision: Option<usize>,
    zero_pad: bool,
) -> Option<(usize, usize)> {
    let w = width?;
    let p = precision?;
    if w <= p {
        Some((w, p))
    } else if zero_pad {
        None
    } else {
        Some((p, p))
    }
}

/// Convert strftime format string to regex pattern
pub fn strftime_to_regex(format_str: &str) -> String {
    let mut regex_parts = Vec::new();
    let mut chars = format_str.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let Some(next_ch) = chars.next() {
                let regex_part = match next_ch {
                    'Y' => r"\d{4}",
                    'y' => r"\d{2}",
                    'm' => r"\d{1,2}",
                    'd' => r"\d{1,2}",
                    'H' => r"\d{1,2}",
                    'M' => r"\d{1,2}",
                    'S' => r"\d{1,2}",
                    'b' | 'h' => r"[A-Za-z]{3}",
                    'B' => r"[A-Za-z]+",
                    'a' => r"[A-Za-z]{3}",
                    'A' => r"[A-Za-z]+",
                    'w' => r"\d",
                    'j' => r"\d{1,3}",
                    'U' | 'W' => r"\d{2}",
                    'c' => r".+",
                    'x' => r".+",
                    'X' => r".+",
                    '%' => "%",
                    _ => ".+?",
                };
                regex_parts.push(regex_part.to_string());
            }
        } else {
            regex_parts.push(regex::escape(&ch.to_string()));
        }
    }

    regex_parts.join("")
}

/// Prefix the field body with optional zero-width lookbehind (issue #9).
#[inline]
pub(crate) fn wrap_field_lookbehind(spec: &FieldSpec, core: String) -> String {
    format!("{}{}", spec.regex_lookbehind.as_deref().unwrap_or(""), core)
}

/// Character class for one unit of width/precision in normal string fields (`s` / `{}`).
pub(crate) const STRING_WIDTH_PRECISION_CHAR: &str = concat!(
    "[^", "\r", "\n", "\u{000B}",
    "\u{000C}",
    "\u{0085}",
    "\u{2028}",
    "\u{2029}",
    "]"
);
