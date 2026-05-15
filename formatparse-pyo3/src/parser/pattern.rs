use crate::error;
use formatparse_core::{FieldSpec, FieldType};
use pyo3::prelude::*;
use regex;
use std::collections::HashMap;

/// Maximum recursive depth when compiling nested format patterns (GitHub issue #12).
pub const MAX_NESTED_FORMAT_DEPTH: usize = 10;

/// Maximum brace nesting **within** one field's format specification (safety cap).
const MAX_BRACE_DEPTH_IN_FORMAT_SPEC: i32 = 10;

/// Result tuple from [`parse_pattern`]: compiled pattern string, search regex string, field
/// specs, original and normalized field names, normalized-to-original name map, and whether
/// `""` may match when every field is a default unconstrained string.
pub type ParsedPatternParts = (
    String,
    String,
    Vec<FieldSpec>,
    Vec<Option<String>>,
    Vec<Option<String>>,
    HashMap<String, String>,
    bool,
);

/// True when `s` contains at least one non-whitespace character (trim is non-empty).
fn literal_delimits_empty_field(s: &str) -> bool {
    !s.trim().is_empty()
}

/// Collect the format-spec substring after `:` until the matching `}` that closes this
/// field, honoring nested `{`…`}` and doubled `{{` / `}}` escapes (formatparse#12).
fn collect_balanced_format_spec(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> PyResult<String> {
    let mut out = String::new();
    let mut depth = 0i32;
    loop {
        let Some(&ch) = chars.peek() else {
            return Err(error::pattern_error(
                "Unclosed '{' in pattern: expected '}' to close the field",
            ));
        };
        if ch == '}' && depth == 0 {
            break;
        }
        let c = chars
            .next()
            .expect("peek matched a char so next() must succeed");
        match c {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    out.push('{');
                    out.push('{');
                } else {
                    depth += 1;
                    if depth > MAX_BRACE_DEPTH_IN_FORMAT_SPEC {
                        return Err(error::pattern_error(
                            "Format specification has too many nested '{' (max 10)",
                        ));
                    }
                    out.push('{');
                }
            }
            // A lone `}` closes one `{…}` nesting level inside the spec. Do **not** merge two
            // consecutive `}` into the `}}` escape here: in `{outer:{inner:d}}` the first `}`
            // closes the inner field and the second closes the outer field (formatparse#12).
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return Err(error::pattern_error(
                        "Unexpected '}' in format specification",
                    ));
                }
                out.push('}');
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

fn brace_balance_valid_for_nested_candidate(s: &str) -> bool {
    let mut depth = 0i32;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' => {
                if it.peek() == Some(&'{') {
                    it.next();
                    continue;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// True when `trimmed` should be compiled as a nested brace pattern (not a classic
/// ``[[fill]align]…[type]]`` format spec).
fn is_nested_format_spec_candidate(trimmed: &str) -> bool {
    if trimmed.len() < 2 {
        return false;
    }
    if !trimmed.starts_with('{') || trimmed.starts_with("{{") {
        return false;
    }
    if !trimmed.ends_with('}') {
        return false;
    }
    brace_balance_valid_for_nested_candidate(trimmed)
}

/// Strip a leading ``^`` and trailing ``$`` from an anchored full-pattern regex string.
fn strip_regex_anchors(anchored: &str) -> String {
    let s = anchored.strip_prefix('^').unwrap_or(anchored);
    let s = s.strip_suffix('$').unwrap_or(s);
    s.to_string()
}

/// After [`parse_field`], `chars` is at optional whitespace then the closing `}`.
/// True when there is a non-whitespace literal run after that `}` and before the next unescaped `{`
/// or end of pattern (formatparse#83). Whitespace-only gaps do not count so ``{} {}`` keeps
/// non-empty captures for both fields.
fn has_trailing_literal_before_next_field(mut chars: std::iter::Peekable<std::str::Chars>) -> bool {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
    if chars.next() != Some('}') {
        return false;
    }
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
    let mut literal = String::new();
    loop {
        match chars.next() {
            None => return literal_delimits_empty_field(&literal),
            Some('{') => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    literal.push('{');
                } else {
                    return literal_delimits_empty_field(&literal);
                }
            }
            Some('}') => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    literal.push('}');
                } else {
                    literal.push('}');
                }
            }
            Some(c) => literal.push(c),
        }
    }
}

/// Parse a format pattern string into regex parts, field specs, and names
/// `allow_empty_delimited_default_string`: when false, default string fields always use `.+?`
/// (used for the unanchored search regex so search/findall do not stop early).
pub fn parse_pattern(
    pattern: &str,
    extra_types: Option<&HashMap<String, PyObject>>,
    custom_patterns: &HashMap<String, String>,
    allow_empty_delimited_default_string: bool,
    nesting_depth: usize,
) -> PyResult<ParsedPatternParts> {
    // Pre-allocate with estimated capacity based on pattern length
    let estimated_fields = pattern.matches('{').count();
    let mut regex_parts = Vec::with_capacity(estimated_fields * 2);
    let mut field_specs = Vec::with_capacity(estimated_fields);
    let mut field_names = Vec::with_capacity(estimated_fields); // Original names
    let mut normalized_names = Vec::with_capacity(estimated_fields); // Normalized for regex
    let mut name_mapping = HashMap::with_capacity(estimated_fields); // normalized -> original
    let mut field_name_types = HashMap::with_capacity(estimated_fields); // Track field name -> FieldType for validation
    let mut chars: std::iter::Peekable<std::str::Chars> = pattern.chars().peekable();
    let mut literal = String::new();
    let mut allows_empty_default_string_match = true;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                // Check for escaped brace
                if chars.peek() == Some(&'{') {
                    chars.next();
                    literal.push('{');
                    continue;
                }

                let had_leading_literal = !literal.trim().is_empty();

                // Flush literal part
                if !literal.is_empty() {
                    allows_empty_default_string_match = false;
                    // If literal ends with whitespace, make it flexible to allow multiple spaces
                    // But use \s+ (one or more) instead of \s* (zero or more) to ensure we consume the space
                    let escaped = if literal.trim_end() != literal {
                        // Literal ends with whitespace - replace trailing whitespace with \s+
                        // to allow one or more spaces (ensures we consume at least one space)
                        let trimmed = literal.trim_end();
                        let mut escaped_str = String::with_capacity(trimmed.len() + 4);
                        escaped_str.push_str(&regex::escape(trimmed));
                        escaped_str.push_str("\\s+");
                        escaped_str
                    } else {
                        regex::escape(&literal)
                    };
                    regex_parts.push(escaped);
                    literal.clear();
                }

                // Parse field specification
                let (mut spec, name) = parse_field(&mut chars, extra_types, nesting_depth)?;

                if matches!(spec.field_type, FieldType::Nested) {
                    if nesting_depth >= MAX_NESTED_FORMAT_DEPTH {
                        return Err(error::pattern_error(
                            "Nested format patterns exceed max depth (10)",
                        ));
                    }
                    let inner = spec.nested_subpattern.as_ref().ok_or_else(|| {
                        error::pattern_error("Internal error: nested field missing subpattern")
                    })?;
                    let (inner_anchored, _, _, _, _, _, _) = parse_pattern(
                        inner,
                        extra_types,
                        custom_patterns,
                        allow_empty_delimited_default_string,
                        nesting_depth + 1,
                    )?;
                    spec.nested_regex_body = Some(strip_regex_anchors(&inner_anchored));
                }

                if !spec.is_default_unconstrained_string() {
                    allows_empty_default_string_match = false;
                }

                let has_trailing_literal = has_trailing_literal_before_next_field(chars.clone());

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
                            peek_chars.next(); // Consume the closing brace
                            found_closing = true;
                            break;
                        } else {
                            break;
                        }
                    }
                    if !found_closing {
                        break None; // No more fields
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
                            continue; // Escaped brace, continue
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

                let allow_empty_delimited = allow_empty_delimited_default_string
                    && spec.is_default_unconstrained_string()
                    && (had_leading_literal || has_trailing_literal);
                let pattern = spec.to_regex_pattern(
                    custom_patterns,
                    next_field_is_greedy,
                    allow_empty_delimited,
                );
                let la_raw = spec.regex_lookahead.as_deref().unwrap_or("");
                let (lb_prefix, body, la_emit) =
                    formatparse_core::rewrite_field_fragments_for_engine_anchor(&pattern, la_raw);

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

                // Issue #15 / parse#146: `{name:brace}` — capture text inside `{`…`}` in the input
                // (non-greedy `.*?`; later pattern literals may force a later `}`). Supports empty `{}`.
                // Requires a non-numbered name.
                let group_pattern = if matches!(spec.field_type, FieldType::BracedContent) {
                    let Some(ref original_name) = name else {
                        return Err(error::pattern_error(
                            "The :brace format requires a named field (e.g. {content:brace})",
                        ));
                    };
                    if original_name.chars().all(|c| c.is_ascii_digit()) {
                        return Err(error::pattern_error(
                            "The :brace format cannot be used with numbered fields",
                        ));
                    }
                    let normalized =
                        normalize_field_name(original_name, &mut name_mapping, &normalized_names);
                    format!("\\{{(?P<{}>.*?)\\}}", normalized)
                } else if let Some(ref original_name) = name {
                    // Check if field name is numeric (numbered field like {0}, {1}) - these should be positional
                    let is_numeric = original_name.chars().all(|c| c.is_ascii_digit());

                    if is_numeric {
                        // Numbered fields are positional (unnamed groups), not named groups
                        format!("{}{}({}){}", lb_prefix, "", body, la_emit)
                    } else {
                        // Normalize name: replace hyphens/dots with underscores, handle collisions
                        let normalized = normalize_field_name(
                            original_name,
                            &mut name_mapping,
                            &normalized_names,
                        );
                        format!("{}{}(?P<{}>{}){}", lb_prefix, "", normalized, body, la_emit)
                    }
                } else {
                    format!("{}{}({}){}", lb_prefix, "", body, la_emit)
                };

                regex_parts.push(group_pattern);

                // Handle name normalization for regex groups
                if let Some(ref original_name) = name {
                    // Check if field name is numeric (numbered field like {0}, {1}) - these should be positional
                    let is_numeric = original_name.chars().all(|c| c.is_ascii_digit());

                    if is_numeric {
                        field_names.push(None); // Store as None (positional)
                        normalized_names.push(None);
                    } else {
                        let normalized = normalize_field_name(
                            original_name,
                            &mut name_mapping,
                            &normalized_names,
                        );
                        field_names.push(Some(original_name.clone())); // Store original
                        normalized_names.push(Some(normalized.clone())); // Store normalized
                        name_mapping.insert(normalized, original_name.clone()); // Map normalized -> original
                    }
                } else {
                    field_names.push(None);
                    normalized_names.push(None);
                }
                field_specs.push(spec);

                // Expect closing brace
                if chars.next() != Some('}') {
                    return Err(error::pattern_error_parse_mismatch(
                        "Expected '}' after field specification",
                    ));
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
        allows_empty_default_string_match = false;
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
    Ok((
        regex_str_with_anchors,
        regex_str,
        field_specs,
        field_names,
        normalized_names,
        name_mapping,
        allows_empty_default_string_match,
    ))
}

/// Normalize field name for use inside `(?P<name>...)` capture groups.
///
/// Hyphens and dots become underscores (legacy parse compatibility). Dict-style paths use
/// `[` / `]` (`person[name]`); only `[` maps to `_`, and closing `]` is omitted so we do not
/// add a trailing separator (e.g. `hello[world]` → `hello_world`). `[` / `]` are not valid
/// in Rust/fancy-regex capture group identifiers.
pub fn normalize_field_name(
    name: &str,
    _name_mapping: &mut HashMap<String, String>,
    existing_normalized: &[Option<String>],
) -> String {
    let mut base_normalized = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '-' | '.' | '[' => base_normalized.push('_'),
            ']' => {}
            _ => base_normalized.push(c),
        }
    }

    // Check for collisions with existing normalized names
    let mut normalized = base_normalized.clone();

    // Find the position of the first underscore to insert additional underscores there
    let underscore_pos = normalized.find('_');

    // Check if this exact normalized name already exists
    let mut collision_count = 0;
    while existing_normalized
        .iter()
        .any(|n| n.as_ref().map(|s| s == &normalized).unwrap_or(false))
    {
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

/// Reject `:ml` / `:blk` combined with numeric-only format specifiers (GitHub issues #8, #69, #70).
///
/// Width, precision, alignment, and fill are supported for multiline and indent-block fields;
/// ``sign``, ``zero_pad``, and ``=`` alignment remain unsupported.
pub fn validate_multiline_mvp(spec: &FieldSpec) -> PyResult<()> {
    if !matches!(
        spec.field_type,
        FieldType::Multiline | FieldType::IndentBlock
    ) {
        return Ok(());
    }
    if spec.sign.is_some() || spec.zero_pad {
        return Err(error::pattern_error(
            "Multiline types :ml and :blk do not support sign or zero-padding",
        ));
    }
    if spec.alignment == Some('=') {
        return Err(error::pattern_error(
            "Multiline types :ml and :blk do not support '=' alignment",
        ));
    }
    Ok(())
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
pub fn parse_field(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    extra_types: Option<&HashMap<String, PyObject>>,
    nesting_depth: usize,
) -> PyResult<(FieldSpec, Option<String>)> {
    let mut spec = FieldSpec::new();
    let mut field_name = String::new();
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

    // Parse format spec (after colon until closing `}` that ends this field)
    if !in_name {
        let format_spec = collect_balanced_format_spec(chars)?;
        let trimmed = format_spec.trim();
        if is_nested_format_spec_candidate(trimmed) {
            if nesting_depth >= MAX_NESTED_FORMAT_DEPTH {
                return Err(error::pattern_error(
                    "Nested format patterns exceed max depth (10)",
                ));
            }
            spec.field_type = FieldType::Nested;
            spec.nested_subpattern = Some(trimmed.to_string());
        } else {
            parse_format_spec(&format_spec, &mut spec, extra_types)?;
        }
        validate_multiline_mvp(&spec)?;
    }

    let name = if field_name.is_empty() {
        None
    } else {
        Some(field_name)
    };

    Ok((spec, name))
}

/// Parse format specifier string into FieldSpec
pub fn parse_format_spec(
    format_spec: &str,
    spec: &mut FieldSpec,
    _extra_types: Option<&HashMap<String, PyObject>>,
) -> PyResult<()> {
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

    // Remaining characters: type token(s), optional trailing lookarounds (issue #9)
    let mut type_str = String::new();
    for ch in chars {
        type_str.push(ch);
    }

    if type_str == "%" {
        spec.field_type = FieldType::Percentage;
        return Ok(());
    }
    if type_str.starts_with('%') {
        formatparse_core::reject_lookaround_in_strftime(&type_str)
            .map_err(|e| error::pattern_error(&e))?;
        spec.field_type = FieldType::DateTimeStrftime;
        spec.strftime_format = Some(type_str.clone());
        return Ok(());
    }

    let (type_base, lookaround_tail) =
        formatparse_core::split_type_base_and_lookaround_tail(&type_str);
    if type_base.is_empty() && !lookaround_tail.is_empty() {
        return Err(error::pattern_error(
            "Type specification must precede lookaround assertions",
        ));
    }

    // Extract type name (alphabetic characters only) from the type base (not from lookarounds)
    let type_name: String = type_base.chars().filter(|c| c.is_alphabetic()).collect();

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
    } else if type_name == "brace" {
        FieldType::BracedContent
    } else if type_name == "ml" {
        FieldType::Multiline
    } else if type_name == "blk" {
        FieldType::IndentBlock
    } else if type_name.len() > 1 {
        FieldType::Custom(type_name)
    } else {
        let type_char = type_name.chars().next().unwrap();
        spec.original_type_char = Some(type_char);
        match type_char {
            's' => FieldType::String,
            'd' | 'i' => FieldType::Integer,
            'b' | 'o' | 'x' | 'X' => FieldType::Integer,
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

    if !lookaround_tail.is_empty() {
        let (lb, la) = formatparse_core::parse_lookaround_tail(lookaround_tail)
            .map_err(|e| error::pattern_error(&e))?;
        match &spec.field_type {
            FieldType::Integer | FieldType::Float => {
                spec.regex_lookbehind = if lb.is_empty() { None } else { Some(lb) };
                spec.regex_lookahead = if la.is_empty() { None } else { Some(la) };
            }
            _ => {
                return Err(error::pattern_error(
                    "Lookaround assertions are only supported for integer and float format types (d, i, b, o, x, X, f, F)",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod normalize_field_name_tests {
    use super::normalize_field_name;
    use std::collections::HashMap;

    #[test]
    fn dict_style_brackets_map_to_underscores() {
        let mut m = HashMap::new();
        let existing: Vec<Option<String>> = vec![];
        assert_eq!(
            normalize_field_name("hello[world]", &mut m, &existing),
            "hello_world"
        );
        assert_eq!(
            normalize_field_name("hello[foo][baz]", &mut m, &existing),
            "hello_foo_baz"
        );
    }

    #[test]
    fn deep_nested_brackets_normalize() {
        let mut m = HashMap::new();
        assert_eq!(normalize_field_name("a[b[c[d]]]", &mut m, &[]), "a_b_c_d");
    }
}

#[cfg(test)]
mod nested_candidate_tests {
    use super::*;
    use pyo3::Python;

    #[test]
    fn nested_candidate_accepts_inner_field_pattern() {
        let s = "{inner:d}";
        assert!(is_nested_format_spec_candidate(s.trim()));
    }

    #[test]
    fn parse_nested_outer_field_type() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|_| {
            let r = parse_pattern("{outer:{inner:d}}", None, &HashMap::new(), true, 0)
                .expect("parse_pattern");
            let specs = &r.2;
            assert_eq!(specs.len(), 1);
            assert!(
                matches!(specs[0].field_type, FieldType::Nested),
                "expected Nested, got {:?}",
                specs[0].field_type
            );
        });
    }
}
