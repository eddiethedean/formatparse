use pyo3::prelude::*;
use pyo3::exceptions::{PyIndexError, PyTypeError};
use pyo3::types::PyList;
use crate::parser::raw_match::RawMatchData;

/// Results container that stores raw match data and lazily converts to ParseResult
/// This avoids creating all ParseResult objects upfront, improving performance
/// The struct itself is lightweight - just a Vec of raw data
#[pyclass]
pub struct Results {
    raw_data: Vec<RawMatchData>,
    // Cache for converted ParseResult objects (lazy evaluation)
    cached_results: Option<PyObject>,
}

impl Results {
    pub fn new(raw_data: Vec<RawMatchData>) -> Self {
        Self {
            raw_data,
            cached_results: None,
        }
    }
    
    /// Convert all raw data to ParseResult objects (called lazily)
    fn convert_all(&mut self, py: Python) -> PyResult<PyObject> {
        if let Some(ref cached) = self.cached_results {
            return Ok(cached.clone_ref(py));
        }
        
        let mut py_results: Vec<PyObject> = Vec::with_capacity(self.raw_data.len());
        for raw_data in &self.raw_data {
            let parse_result = raw_data.to_parse_result(py)?;
            py_results.push(parse_result.to_object(py));
        }
        
        let items: Vec<_> = py_results.iter()
            .map(|obj| obj.bind(py))
            .collect();
        let results_list = PyList::new_bound(py, items);
        let list_obj = results_list.to_object(py);
        
        // Cache the result
        self.cached_results = Some(list_obj.clone_ref(py));
        Ok(list_obj)
    }
    
    /// Convert a single raw data item to ParseResult (for lazy indexing)
    pub fn convert_item(&self, index: usize, py: Python) -> PyResult<PyObject> {
        if index >= self.raw_data.len() {
            return Err(PyIndexError::new_err("list index out of range"));
        }
        
        let raw_data = &self.raw_data[index];
        let parse_result = raw_data.to_parse_result(py)?;
        Ok(parse_result.to_object(py))
    }
}

#[pymethods]
impl Results {
    /// Get the length (no conversion needed)
    fn __len__(&self) -> usize {
        self.raw_data.len()
    }
    
    /// Get an item by index (lazy conversion - only converts the requested item)
    fn __getitem__(&self, key: &Bound<'_, PyAny>, py: Python) -> PyResult<PyObject> {
        // Try to extract as usize first (positive index)
        if let Ok(index) = key.extract::<usize>() {
            // Single item access - convert only this item
            self.convert_item(index, py)
        } else if let Ok(index) = key.extract::<isize>() {
            // Handle negative indices (Python-style)
            let len = self.raw_data.len() as isize;
            let actual_index = if index < 0 {
                (len + index) as usize
            } else {
                index as usize
            };
            self.convert_item(actual_index, py)
        } else if key.is_instance_of::<pyo3::types::PySlice>() {
            // Slice access - convert all items to a list and let Python handle slicing
            // This is less optimal but necessary for slice support
            let mut results = Results::new(self.raw_data.clone());
            let list = results.convert_all(py)?;
            // Use Python's __getitem__ to handle the slice
            let list_bound = list.bind(py);
            let slice_result = list_bound.get_item(key)?;
            Ok(slice_result.to_object(py))
        } else {
            Err(PyTypeError::new_err("list indices must be integers or slices"))
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
    
    /// Convert to list (forces conversion of all items)
    fn to_list(&mut self, py: Python) -> PyResult<PyObject> {
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
    cached_list: Option<PyObject>,  // Cached list of converted items
    index: usize,
}

#[pymethods]
impl ResultsIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    
    fn __next__(&mut self, py: Python) -> PyResult<Option<PyObject>> {
        // On first iteration, batch convert all items at once
        if self.cached_list.is_none() {
            let results = self.results.bind(py);
            // Convert all items in a single batch (one GIL block)
            let list = results.call_method0("to_list")?;
            self.cached_list = Some(list.to_object(py));
        }
        
        // Now iterate over the cached list (no FFI overhead)
        let list_bound = self.cached_list.as_ref().unwrap().bind(py).downcast::<pyo3::types::PyList>()?;
        let len = list_bound.len();
        
        if self.index >= len {
            return Ok(None);
        }
        
        let item = list_bound.get_item(self.index)?;
        self.index += 1;
        Ok(Some(item.to_object(py)))
    }
}

