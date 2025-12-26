use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::get_abbreviated_month_map;

/// Parse Linux system log format: Nov 21 10:21:36 (year is current year)
pub fn parse_system_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let today = datetime_class.call_method0("today")?;
    let current_year: i32 = today.getattr("year")?.extract()?;
    
    let month_map = get_abbreviated_month_map();
    
    // Nov 21 10:21:36 or Nov  1 10:21:36 (note the double space)
    if let Ok(re) = Regex::new(r"^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(month_match), Some(day_match), Some(hour_match), Some(minute_match), Some(second_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5)) {
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let day: u8 = day_match.as_str().trim().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                
                let dt = datetime_class.call1((current_year, month, day, hour, minute, second, 0, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid system datetime: {}", value)))
}

