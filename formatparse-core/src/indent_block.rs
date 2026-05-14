//! Strip common leading indentation from a captured block (GitHub issue #69).
//!
//! Used for ``{name:blk}`` after the same regex boundaries as ``:ml``. Blank lines and
//! lines that contain only spaces or tabs do not contribute to the computed margin.
//! Tabs count as single characters (not expanded). Output lines are joined with ``\\n``.

/// Remove the largest common prefix of spaces/tabs from each line (see module docs).
pub fn strip_common_indent(s: &str) -> String {
    let lines: Vec<String> = s
        .split('\n')
        .map(|p| p.strip_suffix('\r').unwrap_or(p).to_string())
        .collect();

    let mut margin: Option<usize> = None;
    for line in &lines {
        if line.is_empty() {
            continue;
        }
        if line.chars().all(|c| c == ' ' || c == '\t') {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        margin = Some(match margin {
            None => indent,
            Some(m) => m.min(indent),
        });
    }
    let m = margin.unwrap_or(0);
    if m == 0 {
        return s.to_string();
    }

    let mut out = String::with_capacity(s.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let lead = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let rest: String = if lead >= m {
            line.chars().skip(m).collect()
        } else {
            line.trim_start_matches([' ', '\t']).to_string()
        };
        out.push_str(&rest);
    }
    out.trim_start_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string() {
        assert_eq!(strip_common_indent(""), "");
    }

    #[test]
    fn no_leading_indent_unchanged() {
        assert_eq!(strip_common_indent("a\nb"), "a\nb");
    }

    #[test]
    fn strips_common_spaces() {
        assert_eq!(strip_common_indent("  a\n  b"), "a\nb");
    }

    #[test]
    fn blank_lines_do_not_set_margin() {
        assert_eq!(strip_common_indent("  a\n\n  b"), "a\n\nb");
    }

    #[test]
    fn crlf_segments() {
        assert_eq!(strip_common_indent("  a\r\n  b"), "a\nb");
    }

    #[test]
    fn leading_newline_after_dedent_trimmed() {
        assert_eq!(strip_common_indent("\n  a\n  b"), "a\nb");
    }

    #[test]
    fn tabs_count_as_single_indent_chars() {
        assert_eq!(strip_common_indent("\tx\n\ty"), "x\ny");
    }
}
