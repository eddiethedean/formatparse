use crate::parser::format_parser::FormatParser;
use crate::parser::matching::{match_with_captures, match_with_captures_raw, CapturedMatchContext};
use formatparse_core::FieldType;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Incremental iterator over ``findall``-style matches (issue #13 MVP).
///
/// Yields one match at a time using the same non-overlapping scan as :func:`findall`,
/// without building a full :class:`Results` or list first. This lowers peak memory when
/// you only consume matches sequentially. It does **not** implement arbitrary chunked
/// file I/O or cross-chunk backtracking; use line-by-line reads only when your pattern
/// cannot span physical line breaks.
#[pyclass(module = "_formatparse", name = "FindallIter")]
pub struct FindallIter {
    parser: Arc<FormatParser>,
    haystack: String,
    case_sensitive: bool,
    evaluate_result: bool,
    fast_path: bool,
    extra_types: HashMap<String, PyObject>,
    last_end: usize,
    search_pos: usize,
}

impl FindallIter {
    pub fn new(
        parser: Arc<FormatParser>,
        haystack: String,
        case_sensitive: bool,
        evaluate_result: bool,
        extra_types: HashMap<String, PyObject>,
    ) -> Self {
        let has_custom_converters = !extra_types.is_empty();
        let has_nested_dicts = parser.fields.has_nested_dict_fields.iter().any(|&b| b);
        let has_nested_format_fields = parser
            .fields
            .field_specs
            .iter()
            .any(|s| matches!(s.field_type, FieldType::Nested));
        let fast_path = !has_custom_converters
            && evaluate_result
            && !has_nested_dicts
            && !has_nested_format_fields;
        Self {
            parser,
            haystack,
            case_sensitive,
            evaluate_result,
            fast_path,
            extra_types,
            last_end: 0,
            search_pos: 0,
        }
    }
}

#[pymethods]
impl FindallIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> PyResult<Option<PyObject>> {
        if slf.fast_path {
            loop {
                if slf.search_pos > slf.haystack.len() {
                    return Ok(None);
                }
                let search_regex = slf.parser.get_search_regex(slf.case_sensitive);
                let Some(caps) = search_regex
                    .captures_from_pos(&slf.haystack, slf.search_pos)
                    .map_err(crate::error::fancy_regex_match_error)?
                else {
                    return Ok(None);
                };
                let Some(m0) = caps.get(0) else {
                    return Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "regex match missing capture group 0",
                    ));
                };
                let match_start = m0.start();
                let match_end = m0.end();

                if match_start < slf.last_end {
                    slf.search_pos = slf.last_end.max(match_start.saturating_add(1));
                    continue;
                }

                let slices = slf.parser.fields.capture_slices();

                match match_with_captures_raw(&caps, &slf.haystack, match_start, &slices) {
                    Ok(Some(raw_data)) => {
                        slf.last_end = match_end;
                        if match_start == match_end {
                            slf.last_end += 1;
                        }
                        slf.search_pos = slf.last_end;
                        let pr = raw_data.to_parse_result(py)?;
                        return Ok(Some(pr.into_py_any(py)?));
                    }
                    Ok(None) => {
                        slf.search_pos = match_start.saturating_add(1);
                        continue;
                    }
                    Err(_) => {
                        slf.fast_path = false;
                        if slf.last_end == 0 {
                            slf.search_pos = 0;
                        }
                        break;
                    }
                }
            }
        }

        loop {
            if slf.search_pos > slf.haystack.len() {
                return Ok(None);
            }
            let search_regex = slf.parser.get_search_regex(slf.case_sensitive);
            let Some(caps) = search_regex
                .captures_from_pos(&slf.haystack, slf.search_pos)
                .map_err(crate::error::fancy_regex_match_error)?
            else {
                return Ok(None);
            };
            let Some(m0) = caps.get(0) else {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "regex match missing capture group 0",
                ));
            };
            let match_start = m0.start();
            let match_end = m0.end();

            if match_start < slf.last_end {
                slf.search_pos = slf.last_end.max(match_start.saturating_add(1));
                continue;
            }

            let ctx = CapturedMatchContext {
                pattern: &slf.parser.pattern,
                fields: slf.parser.fields.capture_slices(),
                py,
                custom_converters: &slf.extra_types,
                evaluate_result: slf.evaluate_result,
            };

            match match_with_captures(&caps, &ctx)? {
                Some(result) => {
                    slf.last_end = match_end;
                    if match_start == match_end {
                        slf.last_end += 1;
                    }
                    slf.search_pos = slf.last_end;
                    return Ok(Some(result));
                }
                None => {
                    slf.search_pos = match_start.saturating_add(1);
                    continue;
                }
            }
        }
    }

    fn __repr__(&self) -> String {
        "<FindallIter>".to_string()
    }
}
