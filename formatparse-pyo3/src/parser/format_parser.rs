use crate::parser::matching::FieldCaptureSlices;
use crate::result::ParseResult;
use fancy_regex::Regex;
use formatparse_core::count_capturing_groups;
use formatparse_core::parser::MAX_FIELDS;
use formatparse_core::{FieldSpec, FieldType};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Field layout produced at pattern-compile time (narrow interface for matchers).
pub(crate) struct CompiledFields {
    pub field_specs: Vec<FieldSpec>,
    pub field_names: Vec<Option<String>>,
    pub normalized_names: Vec<Option<String>>,
    pub custom_type_groups: Vec<usize>,
    pub has_nested_dict_fields: Vec<bool>,
    pub nested_parsers: Vec<Option<Arc<FormatParser>>>,
    pub field_count: usize,
}

impl CompiledFields {
    pub fn capture_slices(&self) -> FieldCaptureSlices<'_> {
        FieldCaptureSlices {
            field_specs: &self.field_specs,
            field_names: &self.field_names,
            normalized_names: &self.normalized_names,
            custom_type_groups: &self.custom_type_groups,
            has_nested_dict_fields: &self.has_nested_dict_fields,
            nested_parsers: &self.nested_parsers,
        }
    }
}

#[pyclass(module = "_formatparse", from_py_object)]
/// Compiled format pattern for parsing strings.
///
/// Construct with :func:`formatparse.compile` (or ``FormatParser(pattern, extra_types=...)`` in Python).
///
/// **Custom types:** converters passed as ``extra_types`` at compile time are stored
/// and merged with any ``extra_types`` passed per call to ``parse`` or ``search``
/// (per-call keys override stored keys).
///
/// **Pickling:** Only the pattern string is serialized. If the parser was built
/// with ``extra_types``, those converters are **not** restored after unpickling;
/// call ``compile(pattern, extra_types=...)`` again with the same mapping.
pub struct FormatParser {
    #[pyo3(get)]
    // Note: This field is actually used in __getstate__, format getter, and accessed from Python.
    // The dead_code warning is a false positive - the compiler doesn't recognize PyO3 getter usage.
    pub pattern: String,
    pub(crate) regex: Regex,
    pub(crate) regex_str: String, // Store the regex string for _expression property
    pub(crate) regex_case_insensitive: Option<Regex>,
    pub(crate) search_regex: Regex, // Pre-compiled search regex (case-sensitive, no anchors)
    pub(crate) search_regex_case_insensitive: Option<Regex>, // Pre-compiled search regex (case-insensitive, no anchors)
    pub(crate) fields: CompiledFields,
    #[allow(dead_code)]
    pub(crate) name_mapping: std::collections::HashMap<String, String>, // Map normalized -> original
    pub(crate) stored_extra_types: Option<HashMap<String, Py<PyAny>>>, // Store extra_types for use during conversion
    pub(crate) allows_empty_default_string_match: bool, // True iff parse("") can use empty-field fast path (issue #16)
}

impl FormatParser {
    /// Returns true when this parser matches a cache lookup: same normalized pattern and
    /// the same `extra_types` fingerprint as [`crate::extract_extra_types_identity`].
    /// Used after an LRU hit on the hash key to rule out collisions.
    pub(crate) fn matches_pattern_cache_request(
        &self,
        py: Python<'_>,
        normalized_pattern: &str,
        extra_types: &Option<HashMap<String, Py<PyAny>>>,
    ) -> bool {
        if self.pattern != normalized_pattern {
            return false;
        }
        let requested = crate::extract_extra_types_identity(py, extra_types);
        let stored = crate::extract_extra_types_identity(py, &self.stored_extra_types);
        requested == stored
    }

    pub fn new(pattern: &str) -> PyResult<Self> {
        Self::new_with_extra_types(pattern, None)
    }

    pub fn new_with_extra_types(
        pattern: &str,
        extra_types: Option<HashMap<String, Py<PyAny>>>,
    ) -> PyResult<Self> {
        let pattern_owned = crate::pattern_normalize::prepare_compiled_pattern(pattern)?;
        let custom_patterns = Python::attach(|py| -> PyResult<HashMap<String, String>> {
            let mut patterns = HashMap::new();
            if let Some(ref extra_types_map) = extra_types {
                for (name, converter_obj) in extra_types_map {
                    // Try to get the pattern attribute from the converter function
                    let converter_ref = converter_obj.bind(py);
                    if let Ok(pattern_attr) = converter_ref.getattr("pattern") {
                        if let Ok(pattern_str) = pattern_attr.extract::<String>() {
                            patterns.insert(name.clone(), pattern_str);
                        }
                    }
                }
            }
            Ok(patterns)
        })?;

        let (
            regex_str_with_anchors,
            regex_str,
            field_specs,
            field_names,
            normalized_names,
            name_mapping,
            allows_empty_default_string_match,
        ) = formatparse_core::parser::pattern::parse_pattern(
            &pattern_owned,
            &custom_patterns,
            true,
            0,
        )
        .map_err(crate::parser::pattern::pattern_compile_error_to_py)?;

        // Search/findall use a separate compile path without "empty delimited" `.*?` groups so
        // unanchored matching does not stop early (e.g. `{}, {}` on "Hello, World").
        let (regex_str_search_anchored, _, _, _, _, _, _) =
            formatparse_core::parser::pattern::parse_pattern(
                &pattern_owned,
                &custom_patterns,
                false,
                0,
            )
            .map_err(crate::parser::pattern::pattern_compile_error_to_py)?;

        // Validate field count
        if field_specs.len() > MAX_FIELDS {
            return Err(PyValueError::new_err(format!(
                "Pattern contains {} fields, which exceeds the maximum allowed count of {}",
                field_specs.len(),
                MAX_FIELDS
            )));
        }

        let nested_parsers: Vec<Option<Arc<FormatParser>>> = Python::attach(|py| -> PyResult<_> {
            let mut out = Vec::with_capacity(field_specs.len());
            for spec in &field_specs {
                if matches!(spec.field_type, FieldType::Nested) {
                    let sub = spec.nested_subpattern.as_ref().ok_or_else(|| {
                        PyValueError::new_err("internal error: nested field missing subpattern")
                    })?;
                    let cloned_et = extra_types.as_ref().map(|m| {
                        m.iter()
                            .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                            .collect::<HashMap<_, _>>()
                    });
                    out.push(Some(Arc::new(FormatParser::new_with_extra_types(
                        sub, cloned_et,
                    )?)));
                } else {
                    out.push(None);
                }
            }
            Ok(out)
        })?;
        // Pre-compute custom type validation results (pattern_groups per field)
        // This avoids calling validate_custom_type_pattern for every match
        let custom_type_groups = Python::attach(|py| -> PyResult<Vec<usize>> {
            let mut groups = Vec::with_capacity(field_specs.len());
            let empty_map = std::collections::HashMap::new();
            let custom_converters = extra_types
                .as_ref()
                .map(|et| et as &HashMap<String, Py<PyAny>>)
                .unwrap_or(&empty_map);

            for spec in &field_specs {
                let pattern_groups = if matches!(spec.field_type, FieldType::Nested) {
                    spec.nested_regex_body
                        .as_ref()
                        .map(|b| count_capturing_groups(b))
                        .unwrap_or(0)
                } else if !custom_converters.is_empty() {
                    crate::parser::matching::validate_custom_type_pattern(
                        spec,
                        custom_converters,
                        py,
                    )?
                } else {
                    0
                };
                groups.push(pattern_groups);
            }
            Ok(groups)
        })?;

        // Pre-compute which fields have nested dict names (contain '[')
        // This avoids checking original_name.contains('[') in the hot path
        let has_nested_dict_fields: Vec<bool> = field_names
            .iter()
            .map(|name_opt| name_opt.as_ref().map(|n| n.contains('[')).unwrap_or(false))
            .collect();

        // Build regex with DOTALL flag
        let regex = formatparse_core::build_regex(&regex_str_with_anchors)
            .map_err(crate::error::core_error_to_py_err)?;

        let regex_search_anchored = formatparse_core::build_regex(&regex_str_search_anchored)
            .map_err(crate::error::core_error_to_py_err)?;

        // Build case-insensitive regex
        let regex_case_insensitive =
            formatparse_core::build_case_insensitive_regex(&regex_str_with_anchors);

        // Pre-compile search regex variants (without anchors)
        let search_regex =
            formatparse_core::build_search_regex(regex_search_anchored.as_str(), true)
                .map_err(crate::error::core_error_to_py_err)?;
        let search_regex_case_insensitive =
            formatparse_core::build_search_regex(regex_search_anchored.as_str(), false).ok();

        let field_count = field_specs.len();
        Ok(Self {
            pattern: pattern_owned,
            regex,
            regex_str,
            regex_case_insensitive,
            search_regex,
            search_regex_case_insensitive,
            fields: CompiledFields {
                field_specs,
                field_names,
                normalized_names,
                custom_type_groups,
                has_nested_dict_fields,
                nested_parsers,
                field_count,
            },
            name_mapping,
            stored_extra_types: extra_types,
            allows_empty_default_string_match,
        })
    }

    pub fn search_pattern(
        &self,
        string: &str,
        case_sensitive: bool,
        extra_types: Option<HashMap<String, Py<PyAny>>>,
        evaluate_result: bool,
    ) -> PyResult<Option<Py<PyAny>>> {
        // Use pre-compiled search regex
        let search_regex = if case_sensitive {
            &self.search_regex
        } else {
            self.search_regex_case_insensitive
                .as_ref()
                .unwrap_or(&self.search_regex)
        };

        Python::attach(|py| {
            if search_regex
                .captures(string)
                .map_err(crate::error::fancy_regex_match_error)?
                .is_some()
            {
                let extra_types_ref = if let Some(ref et) = extra_types {
                    et
                } else {
                    &HashMap::new()
                };
                let f = &self.fields;
                return crate::parser::matching::match_with_regex(
                    search_regex,
                    &crate::parser::matching::RegexMatchContext {
                        string,
                        pattern: &self.pattern,
                        field_specs: &f.field_specs,
                        field_names: &f.field_names,
                        normalized_names: &f.normalized_names,
                        nested_parsers: &f.nested_parsers,
                        py,
                        custom_converters: extra_types_ref,
                        evaluate_result,
                    },
                );
            }
            Ok(None)
        })
    }

    pub(crate) fn parse_internal(
        &self,
        string: &str,
        case_sensitive: bool,
        extra_types: Option<&HashMap<String, Py<PyAny>>>,
        evaluate_result: bool,
    ) -> PyResult<Option<Py<PyAny>>> {
        Python::attach(|py| {
            let empty = HashMap::<String, Py<PyAny>>::new();
            let extra_types_ref = extra_types.unwrap_or(&empty);

            // Use existing regex (custom type handling is done in convert_value)
            let regex = if case_sensitive {
                &self.regex
            } else {
                self.regex_case_insensitive.as_ref().unwrap_or(&self.regex)
            };

            let f = &self.fields;
            if string.is_empty()
                && self.allows_empty_default_string_match
                && !f.field_specs.is_empty()
            {
                if let Some(obj) = crate::parser::matching::match_empty_default_string_parse(
                    &self.pattern,
                    &f.field_specs,
                    &f.field_names,
                    &f.normalized_names,
                    py,
                    extra_types_ref,
                    evaluate_result,
                )? {
                    return Ok(Some(obj));
                }
            }

            crate::parser::matching::match_with_regex(
                regex,
                &crate::parser::matching::RegexMatchContext {
                    string,
                    pattern: &self.pattern,
                    field_specs: &f.field_specs,
                    field_names: &f.field_names,
                    normalized_names: &f.normalized_names,
                    nested_parsers: &f.nested_parsers,
                    py,
                    custom_converters: extra_types_ref,
                    evaluate_result,
                },
            )
        })
    }

    /// Get the search regex for a given case sensitivity
    pub(crate) fn get_search_regex(&self, case_sensitive: bool) -> &Regex {
        if case_sensitive {
            &self.search_regex
        } else {
            self.search_regex_case_insensitive
                .as_ref()
                .unwrap_or(&self.search_regex)
        }
    }

    /// Parse one capture slice with this parser's pattern (nested fields, issue #12).
    pub(crate) fn parse_nested_capture(
        &self,
        py: Python<'_>,
        slice: &str,
        custom_converters: &HashMap<String, Py<PyAny>>,
    ) -> PyResult<Option<Py<ParseResult>>> {
        let mut merged = HashMap::new();
        if let Some(ref stored) = self.stored_extra_types {
            for (k, v) in stored {
                merged.insert(k.clone(), v.clone_ref(py));
            }
        }
        for (k, v) in custom_converters {
            merged.insert(k.clone(), v.clone_ref(py));
        }
        let opt = self.parse_internal(slice, true, Some(&merged), true)?;
        let Some(obj) = opt else {
            return Ok(None);
        };
        let bound = obj.bind(py);
        let pr = bound.cast::<ParseResult>().map_err(|_| {
            PyValueError::new_err("internal error: nested parse did not return ParseResult")
        })?;
        Ok(Some(pr.clone().unbind()))
    }
}

impl Clone for FormatParser {
    fn clone(&self) -> Self {
        Python::attach(|py| Self {
            pattern: self.pattern.clone(),
            regex: self.regex.clone(),
            regex_str: self.regex_str.clone(),
            regex_case_insensitive: self.regex_case_insensitive.clone(),
            search_regex: self.search_regex.clone(),
            search_regex_case_insensitive: self.search_regex_case_insensitive.clone(),
            fields: CompiledFields {
                field_specs: self.fields.field_specs.clone(),
                field_names: self.fields.field_names.clone(),
                normalized_names: self.fields.normalized_names.clone(),
                custom_type_groups: self.fields.custom_type_groups.clone(),
                has_nested_dict_fields: self.fields.has_nested_dict_fields.clone(),
                nested_parsers: self.fields.nested_parsers.clone(),
                field_count: self.fields.field_count,
            },
            name_mapping: self.name_mapping.clone(),
            stored_extra_types: self.stored_extra_types.as_ref().map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                    .collect()
            }),
            allows_empty_default_string_match: self.allows_empty_default_string_match,
        })
    }
}
