use pyo3::prelude::*;

/// Fixed timezone offset for datetime parsing
#[pyclass]
pub struct FixedTzOffset {
    offset_seconds: i32,
    name: String,
}

#[pymethods]
impl FixedTzOffset {
    #[new]
    fn new(offset_minutes: i32, name: String) -> Self {
        Self {
            offset_seconds: offset_minutes * 60,
            name,
        }
    }

    fn __repr__(&self) -> String {
        let hours = self.offset_seconds.abs() / 3600;
        let minutes = (self.offset_seconds.abs() % 3600) / 60;
        let sign = if self.offset_seconds >= 0 { "" } else { "-" };
        format!("<FixedTzOffset {} {}{}:{:02}:00>", self.name, sign, hours, minutes)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_tz) = other.downcast::<Self>() {
            Ok(self.offset_seconds == other_tz.borrow().offset_seconds && 
               self.name == other_tz.borrow().name)
        } else {
            Ok(false)
        }
    }

    fn __ne__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(!self.__eq__(other)?)
    }

    /// Get the offset in seconds
    #[getter]
    fn offset(&self) -> i32 {
        self.offset_seconds
    }

    /// Get the timezone name
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// tzinfo.utcoffset() - returns timedelta for offset
    fn utcoffset(&self, py: Python, _dt: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let datetime_module = py.import_bound("datetime")?;
        let timedelta_class = datetime_module.getattr("timedelta")?;
        // timedelta(seconds=offset_seconds)
        let delta = timedelta_class.call1((0, 0, self.offset_seconds,))?;
        Ok(delta.to_object(py))
    }

    /// tzinfo.dst() - returns None (no DST)
    fn dst(&self, py: Python, _dt: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        Ok(py.None())
    }

    /// tzinfo.tzname() - returns timezone name
    fn tzname(&self, _dt: Option<&Bound<'_, PyAny>>) -> String {
        self.name.clone()
    }
}

