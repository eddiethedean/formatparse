//! formatparse-pyo3: PyO3 bindings for formatparse
//!
//! formatparse-pyo3 provides Python bindings for the formatparse-core library.

use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

mod datetime;
mod error;
mod match_rs;
mod parser;
mod pattern_cache;
mod pattern_normalize;
mod result;
mod results;
mod types;
mod unicode_offsets;

pub(crate) use pattern_cache::extract_extra_types_identity;
use pattern_cache::get_or_create_parser;
use unicode_offsets::search_byte_range;

pub use datetime::FixedTzOffset;
pub use parser::{FindallIter, Format, FormatParser};
pub use result::*;
pub use results::Results;
pub use types::conversion::*;
// Core types come from formatparse-core
pub use formatparse_core::strftime_to_regex;
pub use formatparse_core::{FieldSpec, FieldType};
pub use match_rs::Match;

pub use error::PatternParseMismatch;

/// Parse a string using a format specification
#[pyfunction]
#[pyo3(signature = (pattern, string, extra_types=None, case_sensitive=false, evaluate_result=true))]
fn parse(
    pattern: &str,
    string: &str,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<Option<Py<PyAny>>> {
    // Validate input lengths
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    // Use cached parser if available
    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    match get_or_create_parser(pattern, extra_types_cloned) {
        Ok(parser) => parser.parse_internal(
            string,
            case_sensitive,
            extra_types.as_ref(),
            evaluate_result,
        ),
        Err(e) => Python::attach(|py| {
            if e.is_instance_of::<PyNotImplementedError>(py) {
                return Err(e);
            }
            if e.is_instance_of::<crate::error::PatternParseMismatch>(py) {
                return Ok(None);
            }
            Err(e)
        }),
    }
}

/// Parse many strings with the same pattern, compiling the pattern once.
///
/// Each input string uses the same semantics as `parse` (including
/// `extra_types`, `case_sensitive`, and `evaluate_result`). Non-matches
/// become Python `None` at that index in the returned list.
#[pyfunction]
#[pyo3(signature = (pattern, strings, extra_types=None, case_sensitive=false, evaluate_result=true))]
fn parse_batch(
    pattern: &str,
    strings: Vec<String>,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<Py<PyAny>> {
    for s in &strings {
        formatparse_core::validate_input_length(s)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        if s.contains('\0') {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Input string contains null byte",
            ));
        }
    }

    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });

    let parser = match get_or_create_parser(pattern, extra_types_cloned) {
        Ok(p) => p,
        Err(e) => {
            return Python::attach(|py| -> PyResult<Py<PyAny>> {
                if e.is_instance_of::<PyNotImplementedError>(py) {
                    return Err(e);
                }
                if e.is_instance_of::<crate::error::PatternParseMismatch>(py) {
                    let none_obj = py.None().into_py_any(py)?;
                    let mut out: Vec<Py<PyAny>> = Vec::with_capacity(strings.len());
                    for _ in 0..strings.len() {
                        out.push(none_obj.clone_ref(py));
                    }
                    let items: Vec<_> = out.iter().map(|o| o.bind(py)).collect();
                    return PyList::new(py, items)?.into_py_any(py);
                }
                Err(e)
            });
        }
    };

    Python::attach(|py| -> PyResult<Py<PyAny>> {
        let mut out: Vec<Py<PyAny>> = Vec::with_capacity(strings.len());
        for s in &strings {
            match parser.parse_internal(s, case_sensitive, extra_types.as_ref(), evaluate_result)? {
                Some(obj) => out.push(obj),
                None => out.push(py.None().into_py_any(py)?),
            }
        }
        let items: Vec<_> = out.iter().map(|o| o.bind(py)).collect();
        PyList::new(py, items)?.into_py_any(py)
    })
}

/// Search for a pattern in a string
#[pyfunction]
#[pyo3(signature = (pattern, string, pos=0, endpos=None, extra_types=None, case_sensitive=true, evaluate_result=true))]
fn search(
    pattern: &str,
    string: &str,
    pos: usize,
    endpos: Option<usize>,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<Option<Py<PyAny>>> {
    // `pos` / `endpos` are Python character indices (like str slicing), not byte offsets.
    let Some((byte_start, byte_end)) = search_byte_range(string, pos, endpos) else {
        return Ok(None);
    };

    // Validate input lengths
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let parser = get_or_create_parser(pattern, extra_types_cloned)?;
    let search_string = &string[byte_start..byte_end];

    if let Some(result) =
        parser.search_pattern(search_string, case_sensitive, extra_types, evaluate_result)?
    {
        // Adjust byte spans to absolute offsets, then expose character indices to Python.
        Python::attach(|py| {
            if let Ok(parse_result) = result.bind(py).cast::<ParseResult>() {
                let result_value = parse_result.borrow();
                let adjusted = result_value
                    .clone()
                    .with_offset(byte_start)
                    .spans_as_char_indices(string);
                Ok(Some(Py::new(py, adjusted)?.into_py_any(py)?))
            } else if let Ok(match_obj) = result.bind(py).cast::<crate::match_rs::Match>() {
                let adjusted = match_obj
                    .borrow()
                    .clone()
                    .with_offset(byte_start)
                    .spans_as_char_indices(string);
                Ok(Some(Py::new(py, adjusted)?.into_py_any(py)?))
            } else {
                Ok(Some(result))
            }
        })
    } else {
        Ok(None)
    }
}

/// Find all matches of a pattern in a string
#[pyfunction]
#[pyo3(signature = (pattern, string, extra_types=None, case_sensitive=false, evaluate_result=true, max_matches=None))]
fn findall(
    pattern: &str,
    string: &str,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
    case_sensitive: bool,
    evaluate_result: bool,
    max_matches: Option<usize>,
) -> PyResult<Py<PyAny>> {
    // Validate input lengths
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    // Check for null bytes in inputs
    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let parser = get_or_create_parser(pattern, extra_types_cloned)?;
    crate::parser::findall_engine::findall_matches(
        parser,
        string,
        extra_types.as_ref(),
        case_sensitive,
        evaluate_result,
        max_matches,
    )
}

/// Iterator over non-overlapping matches (same scan as :func:`findall`, one item per step).
///
/// See :class:`FindallIter` for memory semantics and limitations (issue #13 MVP).
#[pyfunction]
#[pyo3(signature = (pattern, string, extra_types=None, case_sensitive=false, evaluate_result=true, max_matches=None))]
fn findall_iter(
    py: Python<'_>,
    pattern: &str,
    string: &str,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
    case_sensitive: bool,
    evaluate_result: bool,
    max_matches: Option<usize>,
) -> PyResult<Py<FindallIter>> {
    formatparse_core::validate_input_length(string)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    if string.contains('\0') {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input string contains null byte",
        ));
    }

    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
        extra_types.as_ref().map(|et| {
            et.iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect()
        })
    });
    let parser = get_or_create_parser(pattern, extra_types_cloned)?;

    let et_map = Python::attach(|py| -> HashMap<String, Py<PyAny>> {
        extra_types
            .as_ref()
            .map(|et| {
                et.iter()
                    .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                    .collect()
            })
            .unwrap_or_default()
    });

    Py::new(
        py,
        FindallIter::new(
            parser,
            string.to_string(),
            case_sensitive,
            evaluate_result,
            et_map,
            max_matches,
        ),
    )
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
    extra_types: Option<HashMap<String, Py<PyAny>>>,
) -> PyResult<FormatParser> {
    let extra_types_cloned = Python::attach(|py| -> Option<HashMap<String, Py<PyAny>>> {
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
) -> PyResult<Py<PyAny>> {
    use crate::types::FieldSpec;

    // Parse the format spec string
    let mut spec = FieldSpec::new();
    formatparse_core::parser::pattern::parse_format_spec(format_string, &mut spec)
        .map_err(crate::parser::pattern::pattern_compile_error_to_py)?;
    formatparse_core::parser::pattern::validate_multiline_mvp(&spec)
        .map_err(crate::parser::pattern::pattern_compile_error_to_py)?;

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
    Python::attach(|py| {
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
fn _formatparse(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add(
        "PatternParseMismatch",
        py.get_type::<crate::error::PatternParseMismatch>(),
    )?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_batch, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(findall, m)?)?;
    m.add_function(wrap_pyfunction!(findall_iter, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(extract_format, m)?)?;
    m.add_class::<ParseResult>()?;
    m.add_class::<FormatParser>()?;
    m.add_class::<Format>()?;
    m.add_class::<FixedTzOffset>()?;
    m.add_class::<Match>()?;
    m.add_class::<Results>()?;
    m.add_class::<FindallIter>()?;
    Ok(())
}
