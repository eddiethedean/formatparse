use crate::parser::raw_match::RawMatchData;
use pyo3::exceptions::{PyIndexError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::IntoPyObjectExt;

/// Results container that stores raw match data and lazily converts to ParseResult
/// This avoids creating all ParseResult objects upfront, improving performance
/// The struct itself is lightweight - just a Vec of raw data
#[pyclass]
pub struct Results {
    raw_data: Vec<RawMatchData>,
    // Cache for converted ParseResult objects (lazy evaluation)
    cached_results: Option<Py<PyAny>>,
}

impl Results {
    pub fn new(raw_data: Vec<RawMatchData>) -> Self {
        Self {
            raw_data,
            cached_results: None,
        }
    }

    /// Convert all raw data to ParseResult objects (called lazily)
    fn convert_all(&mut self, py: Python) -> PyResult<Py<PyAny>> {
        if let Some(ref cached) = self.cached_results {
            return Ok(cached.clone_ref(py));
        }

        let mut py_results: Vec<Py<PyAny>> = Vec::with_capacity(self.raw_data.len());
        for raw_data in &self.raw_data {
            let parse_result = raw_data.to_parse_result(py)?;
            py_results.push(parse_result.into_py_any(py)?);
        }

        let items: Vec<_> = py_results.iter().map(|obj| obj.bind(py)).collect();
        let results_list = PyList::new(py, items)?;
        let list_obj = results_list.into_py_any(py)?;

        // Cache the result
        self.cached_results = Some(list_obj.clone_ref(py));
        Ok(list_obj)
    }

    /// Convert a single raw data item to ParseResult (for lazy indexing)
    pub fn convert_item(&self, index: usize, py: Python) -> PyResult<Py<PyAny>> {
        if index >= self.raw_data.len() {
            return Err(PyIndexError::new_err("list index out of range"));
        }

        let raw_data = &self.raw_data[index];
        let parse_result = raw_data.to_parse_result(py)?;
        parse_result.into_py_any(py)
    }
}

#[pymethods]
impl Results {
    /// Get the length (no conversion needed)
    fn __len__(&self) -> usize {
        self.raw_data.len()
    }

    /// Get an item by index (lazy conversion - only converts the requested item)
    fn __getitem__(&self, key: &Bound<'_, PyAny>, py: Python) -> PyResult<Py<PyAny>> {
        // Try to extract as usize first (positive index)
        if let Ok(index) = key.extract::<usize>() {
            // Single item access - convert only this item
            self.convert_item(index, py)
        } else if let Ok(index) = key.extract::<isize>() {
            let len = self.raw_data.len();
            let actual_index = if index < 0 {
                match (len as i128).checked_add(index as i128) {
                    Some(sum) if sum >= 0 && (sum as usize) < len => sum as usize,
                    _ => {
                        return Err(PyIndexError::new_err("list index out of range"));
                    }
                }
            } else {
                let u = index as usize;
                if u >= len {
                    return Err(PyIndexError::new_err("list index out of range"));
                }
                u
            };
            self.convert_item(actual_index, py)
        } else if let Ok(slice) = key.cast::<pyo3::types::PySlice>() {
            let len = self.raw_data.len() as isize;
            let indices = slice.indices(len)?;
            let mut items: Vec<Py<PyAny>> = Vec::new();
            let mut i = indices.start;
            while (indices.step > 0 && i < indices.stop) || (indices.step < 0 && i > indices.stop) {
                if i >= 0 && (i as usize) < self.raw_data.len() {
                    items.push(self.convert_item(i as usize, py)?);
                }
                i += indices.step;
            }
            let py_items: Vec<_> = items.iter().map(|obj| obj.bind(py)).collect();
            Ok(PyList::new(py, py_items)?.into_py_any(py)?)
        } else {
            Err(PyTypeError::new_err(
                "list indices must be integers or slices",
            ))
        }
    }

    /// Iterator support (batch converts on first iteration)
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<ResultsIterator> {
        Ok(ResultsIterator {
            results: slf.into(),
            cached_list: None,
            index: 0,
        })
    }

    /// Convert to list (forces conversion of all items).
    /// Name matches the `parse` library Python API (`results.to_list()`).
    #[allow(clippy::wrong_self_convention)] // `to_*` + `&mut self` is required by pymethods / API parity
    fn to_list(&mut self, py: Python) -> PyResult<Py<PyAny>> {
        self.convert_all(py)
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("<Results {} matches>", self.raw_data.len())
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Iterator for Results (batch conversion on first iteration)
/// This avoids FFI overhead by converting all items at once when iteration starts
#[pyclass]
pub struct ResultsIterator {
    results: Py<Results>,
    cached_list: Option<Py<PyAny>>, // Cached list of converted items
    index: usize,
}

#[pymethods]
impl ResultsIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python) -> PyResult<Option<Py<PyAny>>> {
        // On first iteration, batch convert all items at once
        if self.cached_list.is_none() {
            let results = self.results.bind(py);
            // Convert all items in a single batch (one GIL block)
            let list = results.call_method0("to_list")?;
            self.cached_list = Some(list.into_py_any(py)?);
        }

        // Now iterate over the cached list (no FFI overhead)
        let list_bound = self
            .cached_list
            .as_ref()
            .expect("cached_list set immediately above when None")
            .bind(py)
            .cast::<pyo3::types::PyList>()?;
        let len = list_bound.len();

        if self.index >= len {
            return Ok(None);
        }

        let item = list_bound.get_item(self.index)?;
        self.index += 1;
        Ok(Some(item.into_py_any(py)?))
    }
}
