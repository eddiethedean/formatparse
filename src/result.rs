use pyo3::prelude::*;
use pyo3::types::{PyTuple, PySlice};
use std::collections::HashMap;

#[pyclass]
#[derive(Clone)]
pub struct ParseResult {
    fixed: Vec<PyObject>,
    #[pyo3(get)]
    pub named: HashMap<String, PyObject>,
    pub span: (usize, usize),
    pub field_spans: HashMap<String, (usize, usize)>,  // Maps field index/name to (start, end)
}

impl ParseResult {
    pub fn new(fixed: Vec<PyObject>, named: HashMap<String, PyObject>, span: (usize, usize)) -> Self {
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
        field_spans: HashMap<String, (usize, usize)>
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
        self.field_spans = self.field_spans.into_iter()
            .map(|(k, (start, end))| (k, (start + offset, end + offset)))
            .collect();
        self
    }

}

#[pymethods]
impl ParseResult {
    #[new]
    #[pyo3(signature = (fixed, named, span=None))]
    fn new_py(fixed: Vec<PyObject>, named: HashMap<String, PyObject>, span: Option<(usize, usize)>) -> Self {
        Self::new(fixed, named, span.unwrap_or((0, 0)))
    }

    #[getter]
    fn fixed(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let tuple = PyTuple::new_bound(py, &self.fixed);
            Ok(tuple.to_object(py))
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

    fn __repr__(&self) -> String {
        format!("<Result {} {}>", self.fixed.len(), self.named.len())
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            // Try to extract as slice first
            if let Ok(slice) = key.downcast::<PySlice>() {
                let len = self.fixed.len() as std::os::raw::c_long;
                let indices = slice.indices(len)?;
                
                let mut result = Vec::new();
                let mut idx = indices.start;
                for _ in 0..indices.slicelength {
                    if idx >= 0 && (idx as usize) < self.fixed.len() {
                        result.push(self.fixed[idx as usize].clone_ref(py));
                    }
                    idx += indices.step;
                }
                
                let tuple = PyTuple::new_bound(py, &result);
                Ok(tuple.to_object(py))
            } else if let Ok(idx) = key.extract::<usize>() {
                self.fixed
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyIndexError, _>("Index out of range"))
            } else if let Ok(name) = key.extract::<String>() {
                self.named
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key '{}' not found", name)))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>("Key must be int, str, or slice"))
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
            let dict = pyo3::types::PyDict::new_bound(py);
            for (key, value) in &self.field_spans {
                let py_key: PyObject = if let Ok(idx) = key.parse::<usize>() {
                    idx.to_object(py)
                } else {
                    key.to_object(py)
                };
                let py_value = PyTuple::new_bound(py, &[value.0, value.1]);
                dict.set_item(py_key, py_value)?;
            }
            Ok(dict.to_object(py))
        })
    }
}

