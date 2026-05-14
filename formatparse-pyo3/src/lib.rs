//! formatparse-pyo3: PyO3 bindings for formatparse
//!
//! formatparse-pyo3 provides Python bindings for the formatparse-core library.

use lru::LruCache;
use once_cell::sync::Lazy;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

// Use formatparse-core for pure Rust types (imported below via pub use)

mod datetime;
mod error;
mod match_rs;
mod parser;
mod result;
mod results;
mod types;

pub use datetime::FixedTzOffset;
pub use parser::{Format, FormatParser};
pub use result::*;
pub use results::Results;
pub use types::conversion::*;
// Core types come from formatparse-core
pub use formatparse_core::strftime_to_regex;
pub use formatparse_core::{FieldSpec, FieldType};
pub use match_rs::Match;

// Pattern cache for compiled FormatParser instances
// Cache size: 1000 patterns
// Using u64 hash as key for faster lookups
// Using Arc to avoid expensive clones
static PATTERN_CACHE: Lazy<Mutex<LruCache<u64, Arc<FormatParser>>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

/// Create a cache key hash from pattern and `extra_types`.
///
/// Must match what affects compilation in [`FormatParser::new_with_extra_types`]:
/// each converter's `pattern` string and `regex_group_count` (via
/// `validate_custom_type_pattern`). Keys alone are insufficient (same key, different
/// `with_pattern` / group count would incorrectly share a cached parser).
fn create_cache_key_hash(
    py: Python<'_>,
    pattern: &str,
    extra_types: &Option<HashMap<String, PyObject>>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pattern.hash(&mut hasher);
    if let Some(extra_types) = extra_types {
        let mut entries: Vec<(&String, &PyObject)> = extra_types.iter().collect();
        entries.sort_by_key(|(k, _)| *k);
        for (name, converter_obj) in entries {
            name.hash(&mut hasher);
            let converter_ref = converter_obj.bind(py);
            let pat = converter_ref
                .getattr("pattern")
                .ok()
                .and_then(|a| a.extract::<String>().ok())
                .unwrap_or_default();
            pat.hash(&mut hasher);
            // Distinct from any valid regex_group_count (non-negative).
            const GC_MISSING: i64 = -1;
            const GC_NONE: i64 = -2;
            let gc_tag = match converter_ref.getattr("regex_group_count") {
                Ok(v) => {
                    if v.is_none() {
                        GC_NONE
                    } else if let Ok(n) = v.extract::<i64>() {
                        n
                    } else {
                        GC_MISSING
                    }
                }
                Err(_) => GC_MISSING,
            };
            gc_tag.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Get or create a FormatParser from cache
fn get_or_create_parser(
    pattern: &str,
    extra_types: Option<HashMap<String, PyObject>>,
) -> PyResult<Arc<FormatParser>> {
    let cache_key = Python::with_gil(|py| create_cache_key_hash(py, pattern, &extra_types));

    // Try to get from cache (minimize lock scope)
    let cached = {
        let mut cache = PATTERN_CACHE.lock().unwrap();
        cache.get(&cache_key).cloned()
    };

    if let Some(cached_parser) = cached {
        return Ok(cached_parser);
    }

    // Not in cache, create new parser
    let parser = Arc::new(FormatParser::new_with_extra_types(pattern, extra_types)?);

    // Store in cache (minimize lock scope)
    {
        let mut cache = PATTERN_CACHE.lock().unwrap();
        cache.put(cache_key, parser.clone());
    }

    Ok(parser)
}

/// Parse a string using a format specification
#[pyfunction]
#[pyo3(signature = (pattern, string, extra_types=None, case_sensitive=false, evaluate_result=true))]
fn parse(
    pattern: &str,
    string: &str,
    extra_types: Option<HashMap<String, PyObject>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    // Validate input lengths
    formatparse_core::validate_pattern_length(pattern)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if pattern.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Pattern contains null byte",
        ));
    }
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    // Use cached parser if available
    let extra_types_cloned = Python::with_gil(|py| -> Option<HashMap<String, PyObject>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    match get_or_create_parser(pattern, extra_types_cloned) {
        Ok(parser) => parser.parse_internal(string, case_sensitive, extra_types, evaluate_result),
        Err(e) => {
            let err_msg = e.to_string();
            // Propagate NotImplementedError (for unsupported features like quoted keys)
            if err_msg.contains("not supported") {
                return Err(e);
            }
            // If it's an "Expected '}'" error, return None instead of raising
            if err_msg.contains("Expected '}'") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Search for a pattern in a string
#[pyfunction]
#[pyo3(signature = (pattern, string, pos=0, endpos=None, extra_types=None, case_sensitive=true, evaluate_result=true))]
fn search(
    pattern: &str,
    string: &str,
    pos: usize,
    endpos: Option<usize>,
    extra_types: Option<HashMap<String, PyObject>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<Option<PyObject>> {
    // Validate pos parameter
    if pos > string.len() {
        return Ok(None);
    }

    // Validate endpos parameter
    let end = endpos.unwrap_or(string.len());
    if end > string.len() {
        return Ok(None);
    }
    if end < pos {
        return Ok(None);
    }

    // Validate input lengths
    formatparse_core::validate_pattern_length(pattern)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if pattern.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Pattern contains null byte",
        ));
    }
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    let extra_types_cloned = Python::with_gil(|py| -> Option<HashMap<String, PyObject>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let parser = get_or_create_parser(pattern, extra_types_cloned)?;
    let search_string = &string[pos..end];

    if let Some(result) =
        parser.search_pattern(search_string, case_sensitive, extra_types, evaluate_result)?
    {
        // Adjust positions if it's a ParseResult (not Match)
        Python::with_gil(|py| {
            if let Ok(parse_result) = result.bind(py).downcast::<ParseResult>() {
                let result_value = parse_result.borrow();
                let adjusted = result_value.clone().with_offset(pos);
                // Py::new() is already optimized when GIL is held
                Ok(Some(Py::new(py, adjusted)?.into_py_any(py)?))
            } else {
                // It's a Match object - we need to adjust its span
                // For now, just return it as-is (Match spans are relative to search start)
                Ok(Some(result))
            }
        })
    } else {
        Ok(None)
    }
}

/// Find all matches of a pattern in a string
#[pyfunction]
#[pyo3(signature = (pattern, string, extra_types=None, case_sensitive=false, evaluate_result=true))]
fn findall(
    pattern: &str,
    string: &str,
    extra_types: Option<HashMap<String, PyObject>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<PyObject> {
    // Validate input lengths
    formatparse_core::validate_pattern_length(pattern)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if pattern.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Pattern contains null byte",
        ));
    }
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    let extra_types_cloned = Python::with_gil(|py| -> Option<HashMap<String, PyObject>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let parser = get_or_create_parser(pattern, extra_types_cloned)?;

    // Fast path: if no custom converters and evaluate_result=True, use raw matching
    // This defers all Python object creation until the end (batch conversion)
    // CRITICAL: Do ALL regex matching OUTSIDE GIL, then batch convert inside GIL
    let has_custom_converters = extra_types
        .as_ref()
        .map(|et| !et.is_empty())
        .unwrap_or(false);
    let has_nested_dicts = parser.has_nested_dict_fields.iter().any(|&b| b);

    if !has_custom_converters && evaluate_result && !has_nested_dicts {
        // Use raw matching path: collect all raw data first (NO GIL), then batch convert
        let mut raw_results = Vec::new();
        let search_regex = parser.get_search_regex(case_sensitive);
        let mut last_end = 0;

        // Collect all raw matches OUTSIDE GIL (no Python objects created yet)
        // This is the key optimization: all CPU work happens without GIL
        for captures in search_regex.captures_iter(string) {
            let full_match = captures.get(0).unwrap();
            let match_start = full_match.start();
            let match_end = full_match.end();

            if match_start < last_end {
                continue;
            }

            // Try raw matching (no Python objects, no GIL needed)
            if let Ok(Some(raw_data)) = crate::parser::matching::match_with_captures_raw(
                &captures,
                string,
                match_start,
                &crate::parser::matching::FieldCaptureSlices {
                    field_specs: &parser.field_specs,
                    field_names: &parser.field_names,
                    normalized_names: &parser.normalized_names,
                    custom_type_groups: &parser.custom_type_groups,
                    has_nested_dict_fields: &parser.has_nested_dict_fields,
                },
            ) {
                raw_results.push(raw_data);
                last_end = match_end;

                if match_start == match_end {
                    last_end += 1;
                }
            }
        }

        // Return Results object with raw data (lazy conversion)
        // This avoids creating all ParseResult objects upfront
        // The Results object is lightweight - just stores raw data
        return Python::with_gil(|py| -> PyResult<PyObject> {
            let results = Results::new(raw_results);
            Py::new(py, results)?.into_py_any(py)
        });
    }

    // Fallback: use Python path (for custom converters or evaluate_result=False)
    // Optimized: Collect results first, then create PyList with items directly
    Python::with_gil(|py| -> PyResult<PyObject> {
        let search_regex = parser.get_search_regex(case_sensitive);
        let mut results = Vec::new();
        let mut last_end = 0;
        let extra_types_for_matching = if let Some(ref et) = extra_types {
            et
        } else {
            &HashMap::new()
        };

        for captures in search_regex.captures_iter(string) {
            let full_match = captures.get(0).unwrap();
            let match_start = full_match.start();
            let match_end = full_match.end();

            if match_start < last_end {
                continue;
            }

            let extra_types_ref = &extra_types_for_matching;

            if let Some(result) = crate::parser::matching::match_with_captures(
                &captures,
                &crate::parser::matching::CapturedMatchContext {
                    pattern: &parser.pattern,
                    fields: crate::parser::matching::FieldCaptureSlices {
                        field_specs: &parser.field_specs,
                        field_names: &parser.field_names,
                        normalized_names: &parser.normalized_names,
                        custom_type_groups: &parser.custom_type_groups,
                        has_nested_dict_fields: &parser.has_nested_dict_fields,
                    },
                    py,
                    custom_converters: extra_types_ref,
                    evaluate_result,
                },
            )? {
                results.push(result);
                last_end = match_end;

                if match_start == match_end {
                    last_end += 1;
                }
            }
        }

        // Create PyList with items directly (more efficient than empty + append)
        // Convert PyObject to Bound<PyAny> for PyList::new
        let items: Vec<_> = results.iter().map(|obj| obj.bind(py)).collect();
        let results_list = PyList::new(py, items)?;
        Ok(results_list.into())
    })
}

/// Compile a pattern into a FormatParser for reuse.
///
/// Uses the same LRU cache as the `parse`, `search`, and `findall` bindings:
/// `compile` with the same pattern and equivalent `extra_types` keys avoids
/// rebuilding compiled regexes (see GitHub issue #29).
#[pyfunction]
#[pyo3(signature = (pattern, extra_types=None))]
fn compile(
    pattern: &str,
    extra_types: Option<HashMap<String, PyObject>>,
) -> PyResult<FormatParser> {
    // Validate pattern length
    formatparse_core::validate_pattern_length(pattern).map_err(PyValueError::new_err)?;

    // Check for null bytes in pattern
    if pattern.contains('\0') {
        return Err(PyValueError::new_err("Pattern contains null byte"));
    }

    let extra_types_cloned = Python::with_gil(|py| -> Option<HashMap<String, PyObject>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let arc = get_or_create_parser(pattern, extra_types_cloned)?;
    Ok((*arc).clone())
}

/// Extract format specification components from a format string
#[pyfunction]
#[pyo3(signature = (format_string, _match_dict=None))]
fn extract_format(
    format_string: &str,
    _match_dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    use crate::types::FieldSpec;

    // Parse the format spec string
    let mut spec = FieldSpec::new();
    crate::parser::pattern::parse_format_spec(format_string, &mut spec, None);

    // Extract type from the original format_string (preserve original type chars like 'o', 'x', 'b')
    // Parse the format spec to extract the type characters that come after width/precision/alignment
    let type_str: String = if format_string == "%" {
        "%".to_string()
    } else {
        // Parse format spec to find where type starts
        // Format: [[fill]align][sign][#][0][width][,][.precision][type]
        let chars: Vec<char> = format_string.chars().collect();
        let mut i = 0;
        let len = chars.len();

        // Skip fill and align
        if i < len && (chars[i] == '<' || chars[i] == '>' || chars[i] == '^' || chars[i] == '=') {
            i += 1;
        } else if i + 1 < len {
            let ch = chars[i];
            let next_ch = chars[i + 1];
            if (next_ch == '<' || next_ch == '>' || next_ch == '^' || next_ch == '=')
                && ch != next_ch
            {
                i += 2; // Skip fill + align
            }
        }

        // Skip sign
        if i < len && (chars[i] == '+' || chars[i] == '-' || chars[i] == ' ') {
            i += 1;
        }

        // Skip #
        if i < len && chars[i] == '#' {
            i += 1;
        }

        // Skip 0
        if i < len && chars[i] == '0' {
            i += 1;
        }

        // Skip width (digits)
        while i < len && chars[i].is_ascii_digit() {
            i += 1;
        }

        // Skip comma
        if i < len && chars[i] == ',' {
            i += 1;
        }

        // Skip precision (.digits)
        if i < len && chars[i] == '.' {
            i += 1;
            while i < len && chars[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Type is the rest
        if i < len {
            format_string[i..].to_string()
        } else {
            "s".to_string() // Default
        }
    };

    // Build result dictionary
    Python::with_gil(|py| {
        let result = PyDict::new(py);
        result.set_item("type", type_str)?;

        // Extract width
        if let Some(width) = spec.width {
            result.set_item("width", width.to_string())?;
        }

        // Extract precision
        if let Some(precision) = spec.precision {
            result.set_item("precision", precision.to_string())?;
        }

        // Extract alignment
        if let Some(align) = spec.alignment {
            result.set_item("align", align.to_string())?;
        }

        // Extract fill
        if let Some(fill) = spec.fill {
            result.set_item("fill", fill.to_string())?;
        }

        // Extract zero padding
        if spec.zero_pad {
            result.set_item("zero", true)?;
        }

        result.into_py_any(py)
    })
}

/// Python module definition
#[pymodule]
fn _formatparse(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(findall, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(extract_format, m)?)?;
    m.add_class::<ParseResult>()?;
    m.add_class::<FormatParser>()?;
    m.add_class::<Format>()?;
    m.add_class::<FixedTzOffset>()?;
    m.add_class::<Match>()?;
    m.add_class::<Results>()?;
    Ok(())
}
