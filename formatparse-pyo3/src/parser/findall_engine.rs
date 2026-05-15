//! Non-overlapping findall scan (raw fast path and Python fallback).

use crate::parser::format_parser::FormatParser;
use crate::parser::matching::{
    match_with_captures, match_with_captures_raw, CapturedMatchContext, FieldCaptureSlices,
};
use crate::results::Results;
use formatparse_core::FieldType;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Find all non-overlapping matches; returns `Results` or a `list` of matches.
pub(crate) fn findall_matches(
    parser: Arc<FormatParser>,
    string: &str,
    extra_types: Option<&HashMap<String, PyObject>>,
    case_sensitive: bool,
    evaluate_result: bool,
) -> PyResult<PyObject> {
    let has_custom_converters = extra_types
        .as_ref()
        .map(|et| !et.is_empty())
        .unwrap_or(false);
    let has_nested_dicts = parser.fields.has_nested_dict_fields.iter().any(|&b| b);
    let has_nested_format_fields = parser
        .fields
        .field_specs
        .iter()
        .any(|s| matches!(s.field_type, FieldType::Nested));

    if !has_custom_converters && evaluate_result && !has_nested_dicts && !has_nested_format_fields {
        let mut raw_results = Vec::new();
        let search_regex = parser.get_search_regex(case_sensitive);
        let mut last_end = 0;
        let mut raw_path_failed = false;
        let fields = FieldCaptureSlices::from_parser(&parser);

        for cap_result in search_regex.captures_iter(string) {
            let captures = cap_result.map_err(crate::error::fancy_regex_match_error)?;
            let Some(full_match) = captures.get(0) else {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "regex match missing capture group 0",
                ));
            };
            let match_start = full_match.start();
            let match_end = full_match.end();

            if match_start < last_end {
                continue;
            }

            match match_with_captures_raw(&captures, string, match_start, &fields) {
                Ok(Some(raw_data)) => {
                    raw_results.push(raw_data);
                    last_end = match_end;
                    if match_start == match_end {
                        last_end += 1;
                    }
                }
                Ok(None) => {}
                Err(_) => {
                    raw_path_failed = true;
                    break;
                }
            }
        }

        if !raw_path_failed {
            return Python::with_gil(|py| -> PyResult<PyObject> {
                let results = Results::new(raw_results);
                Py::new(py, results)?.into_py_any(py)
            });
        }
    }

    Python::with_gil(|py| -> PyResult<PyObject> {
        let search_regex = parser.get_search_regex(case_sensitive);
        let mut results = Vec::new();
        let mut last_end = 0;
        let empty_extra_types = HashMap::new();
        let extra_types_for_matching = extra_types.unwrap_or(&empty_extra_types);

        for cap_result in search_regex.captures_iter(string) {
            let captures = cap_result.map_err(crate::error::fancy_regex_match_error)?;
            let Some(full_match) = captures.get(0) else {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "regex match missing capture group 0",
                ));
            };
            let match_start = full_match.start();
            let match_end = full_match.end();

            if match_start < last_end {
                continue;
            }

            if let Some(result) = match_with_captures(
                &captures,
                &CapturedMatchContext {
                    pattern: &parser.pattern,
                    fields: FieldCaptureSlices::from_parser(&parser),
                    py,
                    custom_converters: extra_types_for_matching,
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

        let items: Vec<_> = results.iter().map(|obj| obj.bind(py)).collect();
        let results_list = PyList::new(py, items)?;
        Ok(results_list.into())
    })
}
