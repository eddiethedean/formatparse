use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

mod datetime;
mod datetime_parse;
mod parser;
mod result;
mod types;
mod match_rs;

pub use datetime::FixedTzOffset;
pub use parser::{FormatParser, Format};
pub use result::*;
pub use types::*;
pub use match_rs::Match;

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
    // Check for unmatched braces - if pattern compilation fails due to unmatched brace, return None
    // But propagate NotImplementedError for unsupported features
    match FormatParser::new_with_extra_types(pattern, extra_types.clone()) {
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
    let parser = FormatParser::new_with_extra_types(pattern, extra_types.clone())?;
    let end = endpos.unwrap_or(string.len());
    let search_string = &string[pos..end];
    
    if let Some(result) = parser.search_pattern(search_string, case_sensitive, extra_types, evaluate_result)? {
        // Adjust positions if it's a ParseResult (not Match)
        Python::with_gil(|py| {
            if let Ok(parse_result) = result.bind(py).downcast::<ParseResult>() {
                let result_value = parse_result.borrow();
                let adjusted = result_value.clone().with_offset(pos);
                Ok(Some(Py::new(py, adjusted)?.to_object(py)))
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
) -> PyResult<Vec<PyObject>> {
    let parser = FormatParser::new_with_extra_types(pattern, extra_types.clone())?;
    let mut results = Vec::new();
    let mut pos = 0;
    
    while pos < string.len() {
        if let Some(result) = parser.search_pattern(&string[pos..], case_sensitive, extra_types.clone(), evaluate_result)? {
            Python::with_gil(|py| {
                // Get span from result
                let span_end = if let Ok(parse_result) = result.bind(py).downcast::<ParseResult>() {
                    parse_result.borrow().span.1
                } else if let Ok(match_obj) = result.bind(py).downcast::<Match>() {
                    match_obj.borrow().span.1
                } else {
                    0
                };
                
                // Adjust offset for ParseResult
                let adjusted_result = if evaluate_result {
                    if let Ok(parse_result) = result.bind(py).downcast::<ParseResult>() {
                        let result_value = parse_result.borrow();
                        Py::new(py, result_value.clone().with_offset(pos))?.to_object(py)
                    } else {
                        result
                    }
                } else {
                    result
                };
                
                results.push(adjusted_result);
                pos = pos + span_end;
                if span_end == 0 {
                    pos += 1; // Avoid infinite loop
                }
                Ok::<(), PyErr>(())
            })?;
        } else {
            pos += 1; // Try next position
        }
    }
    
    Ok(results)
}

/// Compile a pattern into a FormatParser for reuse
#[pyfunction]
#[pyo3(signature = (pattern, extra_types=None))]
fn compile(
    pattern: &str,
    extra_types: Option<HashMap<String, PyObject>>,
) -> PyResult<FormatParser> {
    FormatParser::new_with_extra_types(pattern, extra_types)
}

/// Extract format specification components from a format string
#[pyfunction]
#[pyo3(signature = (format_string, _match_dict=None))]
fn extract_format(
    format_string: &str,
    _match_dict: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    use crate::parser::FormatParser;
    use crate::types::FieldSpec;
    
    // Parse the format spec string
    let mut spec = FieldSpec::new();
    FormatParser::parse_format_spec(format_string, &mut spec, None);
    
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
            if (next_ch == '<' || next_ch == '>' || next_ch == '^' || next_ch == '=') && ch != next_ch {
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
        let result = PyDict::new_bound(py);
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
        
        Ok(result.into())
    })
}


/// Python module definition
#[pymodule]
fn _structparse(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
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
    Ok(())
}

