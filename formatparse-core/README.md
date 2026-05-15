# formatparse-core

Pure Rust library for parsing strings using Python `format()`-style patterns.

This crate holds the **language-agnostic** logic: field specifications, regex construction from those specs, datetime microsecond helpers, input normalization (line continuations, indent stripping), and safety limits. It has **no dependency on Python or PyO3**, so it is suitable for:

- Running `cargo test` without a Python install
- Embedding in other Rust projects
- Building non-Python bindings on top of the same engine

## Modules (public surface)

The crate root re-exports the main building blocks; see [`src/lib.rs`](src/lib.rs) for the full list. Highlights:

| Module | Role |
|--------|------|
| `types` | `FieldType`, `FieldSpec`, and regex fragments for each field kind |
| `types::regex` | Helpers such as `strftime_to_regex` |
| `parser` | Length/name validation, `build_regex` / search-regex helpers (`parser::regex`) |
| `datetime` | Microsecond digit parsing shared with bindings |
| `error` | `FormatParseError` and related messages |
| `indent_block` | Strip common leading indent from captured block text |
| `input_line_continuations` | Normalize backslash line continuations in input |

## Testing

```bash
cargo test --package formatparse-core
```

## Usage

The primary consumer is the [`formatparse-pyo3`](../formatparse-pyo3) crate (Python extension). You can also use `formatparse-core` directly:

```rust
use formatparse_core::{FieldSpec, FieldType};

let spec = FieldSpec {
    name: Some("age".to_string()),
    field_type: FieldType::Integer,
    width: None,
    precision: None,
    alignment: None,
    sign: None,
    fill: None,
    zero_pad: false,
    strftime_format: None,
    original_type_char: None,
    nested_subpattern: None,
    nested_regex_body: None,
    regex_lookbehind: None,
    regex_lookahead: None,
};
```

Or use `FieldSpec::default()` / `FieldSpec::new()` and mutate the fields you need.
