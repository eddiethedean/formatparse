//! Trailing regex lookaround assertions in format type tails (issue #9 / parse#209).

use crate::parser::count_capturing_groups;
use fancy_regex::Regex;

/// Maximum bytes allowed for the concatenated lookaround tail (all groups).
const MAX_LOOKAROUND_TAIL_BYTES: usize = 4096;

/// Split `type_str` into the type token(s) and any trailing `(?=…)` / `(?!…)` / `(?<=…)` / `(?<!…)`.
///
/// Call only for non-strftime type tails (caller handles `%…` and rejects embedded lookarounds).
pub fn split_type_base_and_lookaround_tail(type_str: &str) -> (&str, &str) {
    let t = type_str.trim();
    if let Some(i) = find_first_lookaround_start(t) {
        let base = t[..i].trim_end();
        let tail = t[i..].trim_start();
        (base, tail)
    } else {
        (t, "")
    }
}

fn find_first_lookaround_start(s: &str) -> Option<usize> {
    for (i, _) in s.char_indices() {
        if starts_with_lookaround(s, i) {
            return Some(i);
        }
    }
    None
}

fn starts_with_lookaround(s: &str, i: usize) -> bool {
    let rest = &s[i..];
    rest.starts_with("(?<=")
        || rest.starts_with("(?<!")
        || rest.starts_with("(?=")
        || rest.starts_with("(?!")
}

/// Extract end byte index (exclusive) of the balanced `(...)` group starting at `open_idx`.
fn balanced_paren_group_end(s: &str, open_idx: usize) -> Option<usize> {
    if !s[open_idx..].starts_with('(') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_idx;
    while i < s.len() {
        let ch = s[i..].chars().next()?;
        if ch == '\\' {
            i += ch.len_utf8();
            if i < s.len() {
                i += s[i..].chars().next()?.len_utf8();
            }
            continue;
        }
        match ch {
            '(' => {
                depth += 1;
                i += ch.len_utf8();
            }
            ')' => {
                depth -= 1;
                i += ch.len_utf8();
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += ch.len_utf8(),
        }
    }
    None
}

/// Parse `tail` into concatenated lookbehind and lookahead fragments (order preserved).
/// Each segment must be a single non-capturing lookaround group with no capturing groups inside.
pub fn parse_lookaround_tail(tail: &str) -> Result<(String, String), String> {
    let tail = tail.trim();
    if tail.is_empty() {
        return Ok((String::new(), String::new()));
    }
    if tail.len() > MAX_LOOKAROUND_TAIL_BYTES {
        return Err(format!(
            "Lookaround tail exceeds maximum length of {} bytes",
            MAX_LOOKAROUND_TAIL_BYTES
        ));
    }

    let mut lookbehind = String::new();
    let mut lookahead = String::new();
    let mut pos = 0usize;
    let t = tail;

    while pos < t.len() {
        while let Some(c) = t[pos..].chars().next() {
            if c.is_whitespace() {
                pos += c.len_utf8();
            } else {
                break;
            }
        }
        if pos >= t.len() {
            break;
        }
        if t.as_bytes().get(pos) != Some(&b'(') {
            return Err(format!(
                "Unexpected text in lookaround tail at byte {}: expected '('",
                pos
            ));
        }
        let end = balanced_paren_group_end(t, pos).ok_or_else(|| {
            format!(
                "Unclosed parenthesis in lookaround tail starting at byte {}",
                pos
            )
        })?;
        let group = &t[pos..end];
        if !is_allowed_lookaround_prefix(group) {
            return Err(format!(
                "Invalid lookaround group (must start with (?=, (?!, (?<=, or (?<!): {:?}",
                truncate(group, 64)
            ));
        }
        if count_capturing_groups(group) != 0 {
            return Err("Lookaround groups must not contain capturing parentheses".to_string());
        }
        Regex::new(group).map_err(|e| format!("Invalid lookaround regex: {}", e))?;

        if group.starts_with("(?<=") || group.starts_with("(?<!") {
            lookbehind.push_str(group);
        } else {
            lookahead.push_str(group);
        }
        pos = end;
    }

    Ok((lookbehind, lookahead))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn is_allowed_lookaround_prefix(group: &str) -> bool {
    group.starts_with("(?<=")
        || group.starts_with("(?<!")
        || group.starts_with("(?=")
        || group.starts_with("(?!")
}

/// True if `body` contains only literal characters and `\\.` escapes, with no other regex operators.
fn is_literal_lookaround_body(body: &str) -> bool {
    let mut it = body.chars();
    while let Some(ch) = it.next() {
        if ch == '\\' {
            if it.next().is_none() {
                return false;
            }
            continue;
        }
        match ch {
            '|' | '(' | ')' | '[' | ']' | '.' | '*' | '+' | '?' | '{' | '}' | '^' | '$' => {
                return false;
            }
            _ => {}
        }
    }
    true
}

/// fancy-regex 0.14 does not match some anchored patterns that combine `^` / `$` with
/// **positive** lookbehind at the start or **positive** lookahead before the end anchor.
/// For literal-only bodies, rewrite those assertions to non-capturing groups so spans stay
/// on the field capture and full-string parse still works.
///
/// Returns `(prefix, field_body_rest, lookahead_suffix)` to be assembled as
/// `prefix + "(?P<name>" + field_body_rest + ")" + lookahead_suffix` (or the unnamed variant).
pub fn rewrite_field_fragments_for_engine_anchor(
    field_body: &str,
    trailing_lookahead: &str,
) -> (String, String, String) {
    let mut prefix = String::new();
    let mut rest = field_body;
    while rest.starts_with("(?<=") {
        let Some(end) = balanced_paren_group_end(rest, 0) else {
            break;
        };
        let group = &rest[..end];
        let inner = group
            .strip_prefix("(?<=")
            .and_then(|g| g.strip_suffix(')'))
            .unwrap_or("");
        if !is_literal_lookaround_body(inner) {
            break;
        }
        prefix.push_str("(?:");
        prefix.push_str(inner);
        prefix.push(')');
        rest = &rest[end..];
    }
    let la = lower_positive_lookahead_suffix(trailing_lookahead);
    (prefix, rest.to_string(), la)
}

fn lower_positive_lookahead_suffix(trailing_lookahead: &str) -> String {
    let t = trailing_lookahead.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut pos = 0usize;
    while pos < t.len() {
        while let Some(c) = t[pos..].chars().next() {
            if c.is_whitespace() {
                pos += c.len_utf8();
            } else {
                break;
            }
        }
        if pos >= t.len() {
            break;
        }
        if t.as_bytes().get(pos) != Some(&b'(') {
            out.push_str(&t[pos..]);
            break;
        }
        let Some(end) = balanced_paren_group_end(t, pos) else {
            out.push_str(&t[pos..]);
            break;
        };
        let group = &t[pos..end];
        if let Some(inner) = group.strip_prefix("(?=").and_then(|g| g.strip_suffix(')')) {
            if is_literal_lookaround_body(inner) {
                out.push_str("(?:");
                out.push_str(inner);
                out.push(')');
            } else {
                out.push_str(group);
            }
        } else {
            out.push_str(group);
        }
        pos = end;
    }
    out
}

/// If `type_str` is a strftime tail (`%…` but not exactly `%`), reject when lookarounds are present.
pub fn reject_lookaround_in_strftime(type_str: &str) -> Result<(), String> {
    let t = type_str.trim();
    if t == "%" {
        return Ok(());
    }
    if t.starts_with('%') && find_first_lookaround_start(t).is_some() {
        return Err(
            "Lookaround assertions are not supported with strftime (%…) format types".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_d_lookahead() {
        let (base, tail) = split_type_base_and_lookaround_tail("d(?=px)");
        assert_eq!(base, "d");
        assert_eq!(tail, "(?=px)");
    }

    #[test]
    fn split_custom_lookahead() {
        let (base, tail) = split_type_base_and_lookaround_tail("MyType(?=x)");
        assert_eq!(base, "MyType");
        assert_eq!(tail, "(?=x)");
    }

    #[test]
    fn strftime_rejects_embedded_lookaround() {
        let err = reject_lookaround_in_strftime("%Y(?=x)").unwrap_err();
        assert!(err.contains("strftime"), "{}", err);
    }

    #[test]
    fn parse_tail_orders_lb_then_la() {
        let (lb, la) = parse_lookaround_tail("(?<=\\$)(?=px)").unwrap();
        assert!(lb.starts_with("(?<="));
        assert!(la.starts_with("(?="));
    }

    #[test]
    fn regex_engine_accepts_issue_examples() {
        Regex::new(r"\d+(?=px)").expect("lookahead");
        Regex::new(r"(?<=\$)\d+").expect("lookbehind");
        Regex::new(r"(?<=\$)\d+(?=px)").expect("combined");
    }

    #[test]
    fn reject_capture_inside_lookaround() {
        let err = parse_lookaround_tail(r"(?=([0-9]))").unwrap_err();
        assert!(err.contains("capturing"));
    }

    #[test]
    fn rewrite_lowers_literal_positive_lb_and_la() {
        let (p, b, la) = rewrite_field_fragments_for_engine_anchor(r"(?<=\$)\d+", "(?=(?:px))");
        assert_eq!(p, r"(?:\$)");
        assert_eq!(b, r"\d+");
        // Non-simple lookahead body is preserved
        assert_eq!(la, "(?=(?:px))");

        let (p2, b2, la2) = rewrite_field_fragments_for_engine_anchor(r"(?<=\$)\d+", "(?=px)");
        assert_eq!(p2, r"(?:\$)");
        assert_eq!(b2, r"\d+");
        assert_eq!(la2, "(?:px)");
    }
}
