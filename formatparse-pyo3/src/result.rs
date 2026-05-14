use pyo3::prelude::*;
use pyo3::types::{PySlice, PyTuple};
use std::collections::HashMap;

#[pyclass]
pub struct ParseResult {
    fixed: Vec<PyObject>,
    #[pyo3(get)]
    pub named: HashMap<String, PyObject>,
    pub span: (usize, usize),
    pub field_spans: HashMap<String, (usize, usize)>, // Maps field index/name to (start, end)
}

impl Clone for ParseResult {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Self {
            fixed: self.fixed.iter().map(|obj| obj.clone_ref(py)).collect(),
            named: self
                .named
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect(),
            span: self.span,
            field_spans: self.field_spans.clone(),
        })
    }
}

/// Truncate by Unicode scalar values so we never split inside a codepoint.
fn repr_trunc(s: &str, max_chars: usize) -> String {
    if max_chars < 3 {
        return "...".to_string();
    }
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    let take = max_chars.saturating_sub(3);
    s.chars().take(take).collect::<String>() + "..."
}

impl ParseResult {
    pub fn new(
        fixed: Vec<PyObject>,
        named: HashMap<String, PyObject>,
        span: (usize, usize),
    ) -> Self {
        Self {
            fixed,
            named,
            span,
            field_spans: HashMap::new(),
        }
    }

    pub fn new_with_spans(
        fixed: Vec<PyObject>,
        named: HashMap<String, PyObject>,
        span: (usize, usize),
        field_spans: HashMap<String, (usize, usize)>,
    ) -> Self {
        Self {
            fixed,
            named,
            span,
            field_spans,
        }
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.span = (self.span.0 + offset, self.span.1 + offset);
        // Adjust all field spans by offset
        self.field_spans = self
            .field_spans
            .into_iter()
            .map(|(k, (start, end))| (k, (start + offset, end + offset)))
            .collect();
        self
    }

    /// Rich but bounded string for `__repr__` / `__str__` (sorted `named` keys for stability).
    fn format_display(&self, py: Python<'_>) -> PyResult<String> {
        const MAX_KEYS: usize = 12;
        const MAX_VAL_CHARS: usize = 120;
        const MAX_FIXED: usize = 8;

        let mut keys: Vec<_> = self.named.keys().cloned().collect();
        keys.sort();

        let mut named_parts = Vec::new();
        for k in keys.iter().take(MAX_KEYS) {
            let v = self.named.get(k).expect("key from sorted vec");
            let r: String = v.bind(py).repr()?.extract()?;
            named_parts.push(format!("{k:?}: {}", repr_trunc(&r, MAX_VAL_CHARS)));
        }
        let mut named_body = named_parts.join(", ");
        if keys.len() > MAX_KEYS {
            named_body.push_str(&format!(", ... (+{} more)", keys.len() - MAX_KEYS));
        }
        let named_display = format!("{{{}}}", named_body);

        let fixed_display = if self.fixed.is_empty() {
            "()".to_string()
        } else {
            let mut fp = Vec::new();
            for obj in self.fixed.iter().take(MAX_FIXED) {
                let r: String = obj.bind(py).repr()?.extract()?;
                fp.push(repr_trunc(&r, MAX_VAL_CHARS));
            }
            if self.fixed.len() > MAX_FIXED {
                format!(
                    "({}, ... (+{} more))",
                    fp.join(", "),
                    self.fixed.len() - MAX_FIXED
                )
            } else {
                format!("({})", fp.join(", "))
            }
        };

        Ok(format!(
            "<ParseResult span={:?} named={} fixed={}>",
            self.span, named_display, fixed_display
        ))
    }
}

#[pymethods]
impl ParseResult {
    #[new]
    #[pyo3(signature = (fixed, named, span=None))]
    fn new_py(
        fixed: Vec<PyObject>,
        named: HashMap<String, PyObject>,
        span: Option<(usize, usize)>,
    ) -> Self {
        Self::new(fixed, named, span.unwrap_or((0, 0)))
    }

    #[getter]
    fn fixed(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let items: Vec<_> = self.fixed.iter().map(|obj| obj.bind(py)).collect();
            let tuple = PyTuple::new(py, items)?;
            Ok(tuple.into())
        })
    }

    #[getter]
    fn span(&self) -> (usize, usize) {
        self.span
    }

    #[getter]
    fn start(&self) -> usize {
        self.span.0
    }

    #[getter]
    fn end(&self) -> usize {
        self.span.1
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        self.format_display(py)
    }

    fn __str__(&self, py: Python<'_>) -> PyResult<String> {
        self.format_display(py)
    }

    fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            // Try to extract as slice first
            if let Ok(slice) = key.downcast::<PySlice>() {
                let len = self.fixed.len() as isize;
                let indices = slice.indices(len)?;

                let mut result = Vec::new();
                let mut idx = indices.start;
                for _ in 0..indices.slicelength {
                    if idx >= 0 && (idx as usize) < self.fixed.len() {
                        result.push(self.fixed[idx as usize].bind(py));
                    }
                    idx += indices.step;
                }

                let tuple = PyTuple::new(py, result)?;
                Ok(tuple.into())
            } else if let Ok(idx) = key.extract::<usize>() {
                self.fixed
                    .get(idx)
                    .map(|obj| obj.clone_ref(py))
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyIndexError, _>("Index out of range")
                    })
            } else if let Ok(name) = key.extract::<String>() {
                self.named
                    .get(&name)
                    .map(|obj| obj.clone_ref(py))
                    .ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                            "Key '{}' not found",
                            name
                        ))
                    })
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "Key must be int, str, or slice",
                ))
            }
        })
    }

    fn __contains__(&self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        Python::with_gil(|_py| {
            if let Ok(idx) = key.extract::<usize>() {
                Ok(idx < self.fixed.len())
            } else if let Ok(name) = key.extract::<String>() {
                Ok(self.named.contains_key(&name))
            } else {
                Ok(false)
            }
        })
    }

    #[getter]
    fn spans(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            for (key, value) in &self.field_spans {
                let py_key: PyObject = if let Ok(idx) = key.parse::<usize>() {
                    idx.into_py(py)
                } else {
                    key.clone().into_py(py)
                };
                let py_value = PyTuple::new(py, [value.0, value.1])?;
                dict.set_item(py_key.bind(py), py_value)?;
            }
            Ok(dict.into_py(py))
        })
    }
}
