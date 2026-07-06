//! Map Python str character indices to UTF-8 byte offsets for search(pos/endpos).

/// Number of Unicode scalar values in `s` (same as Python `len(s)`).
pub(crate) fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Byte offset at the start of the character at `char_index`, or `s.len()` when
/// `char_index == char_len(s)`. Returns `None` when `char_index > char_len(s)`.
pub(crate) fn char_index_to_byte_start(s: &str, char_index: usize) -> Option<usize> {
    let n = char_len(s);
    if char_index > n {
        return None;
    }
    if char_index == n {
        return Some(s.len());
    }
    if char_index == 0 {
        return Some(0);
    }
    s.char_indices()
        .nth(char_index)
        .map(|(byte_idx, _)| byte_idx)
}

/// Exclusive end: byte offset of the character at `char_index`, or `s.len()` when
/// `char_index == char_len(s)`.
pub(crate) fn char_index_to_byte_end(s: &str, char_index: usize) -> Option<usize> {
    char_index_to_byte_start(s, char_index)
}

/// Resolve Python `pos` / `endpos` (character indices) to a UTF-8 byte range.
/// Returns `None` when the range is invalid or empty.
pub(crate) fn search_byte_range(
    s: &str,
    pos: usize,
    endpos: Option<usize>,
) -> Option<(usize, usize)> {
    let char_count = char_len(s);
    if pos > char_count {
        return None;
    }
    let end_char = endpos.unwrap_or(char_count);
    if end_char > char_count || end_char < pos {
        return None;
    }
    let byte_start = char_index_to_byte_start(s, pos)?;
    let byte_end = char_index_to_byte_end(s, end_char)?;
    if byte_end < byte_start {
        return None;
    }
    Some((byte_start, byte_end))
}

/// Convert a byte offset (on a char boundary) to a Python character index.
pub(crate) fn byte_to_char_index(s: &str, byte_offset: usize) -> usize {
    if byte_offset == 0 {
        return 0;
    }
    debug_assert!(byte_offset <= s.len());
    debug_assert!(s.is_char_boundary(byte_offset));
    s[..byte_offset].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rocket_ab_range() {
        let s = "🚀ab value=42";
        assert_eq!(char_len(s), 12);
        assert_eq!(char_index_to_byte_start(s, 0), Some(0));
        assert_eq!(char_index_to_byte_start(s, 1), Some(4)); // 'a'
        assert_eq!(search_byte_range(s, 1, None), Some((4, s.len())));
    }

    #[test]
    fn byte_to_char_roundtrip() {
        let s = "🚀ab";
        assert_eq!(byte_to_char_index(s, 4), 1);
        assert_eq!(byte_to_char_index(s, 0), 0);
    }
}
