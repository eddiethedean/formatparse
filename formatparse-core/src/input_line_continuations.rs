//! Backslash–newline continuations in **matched input** for `:ml` / `:blk` fields (GitHub issue #80).
//!
//! Same rules as format **pattern** continuations (issue #68): a run of `k` backslashes immediately
//! before a line ending (`\n` or `\r\n`) — if `k` is odd, the line continues (the last backslash
//! plus the line break are removed, and `floor((k-1)/2)` literal backslashes are kept); if `k` is
//! even, `k/2` literal backslashes are kept and the line break is preserved. After a continuation,
//! ASCII spaces and tabs at the start of the next physical line are stripped.

/// Fold backslash line continuations in captured multiline / indent-block text.
pub fn normalize_input_line_continuations(input: &str) -> String {
    let b = input.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0usize;
    while i < b.len() {
        let start = i;
        let mut j = i;
        while j < b.len() && b[j] != b'\n' && b[j] != b'\r' {
            j += 1;
        }
        let seg = &b[start..j];
        if j == b.len() {
            out.extend_from_slice(seg);
            break;
        }
        let mut k = 0usize;
        let mut p = seg.len();
        while p > 0 && seg[p - 1] == b'\\' {
            k += 1;
            p -= 1;
        }
        let prefix_end = seg.len().saturating_sub(k);
        if k % 2 == 1 {
            out.extend_from_slice(&seg[..prefix_end]);
            let emit = (k - 1) / 2;
            out.extend(std::iter::repeat_n(b'\\', emit));
            if b[j] == b'\r' && j + 1 < b.len() && b[j + 1] == b'\n' {
                i = j + 2;
            } else {
                i = j + 1;
            }
            while i < b.len() && matches!(b[i], b' ' | b'\t') {
                i += 1;
            }
        } else {
            out.extend_from_slice(seg);
            if b[j] == b'\r' && j + 1 < b.len() && b[j + 1] == b'\n' {
                out.push(b'\r');
                out.push(b'\n');
                i = j + 2;
            } else {
                out.push(b[j]);
                i = j + 1;
            }
        }
    }
    // Output is built only from valid UTF-8 slices of `input` plus ASCII `\` / newlines / spaces.
    String::from_utf8(out).expect("normalize_input_line_continuations: UTF-8 invariant")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_joins_lines() {
        assert_eq!(normalize_input_line_continuations("foo\\\nbar"), "foobar");
    }

    #[test]
    fn continuation_crlf() {
        assert_eq!(normalize_input_line_continuations("foo\\\r\nbar"), "foobar");
    }

    #[test]
    fn even_backslashes_keep_newline() {
        assert_eq!(
            normalize_input_line_continuations("foo\\\\\nbar"),
            "foo\\\\\nbar"
        );
    }

    #[test]
    fn odd_three_backslashes() {
        assert_eq!(normalize_input_line_continuations("a\\\\\\\nb"), "a\\b");
    }

    #[test]
    fn literal_newline_unchanged() {
        assert_eq!(normalize_input_line_continuations("a\nb"), "a\nb");
    }

    #[test]
    fn empty() {
        assert_eq!(normalize_input_line_continuations(""), "");
    }

    #[test]
    fn continuation_strips_spaces_tabs_on_next_line() {
        assert_eq!(
            normalize_input_line_continuations("foo\\\n  \t  bar"),
            "foobar"
        );
    }
}
