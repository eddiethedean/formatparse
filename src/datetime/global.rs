use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::{get_month_map, parse_timezone};

/// Parse Global (day/month) datetime format
/// Formats: 21/11/2011, 21-11-2011, 21-Nov-2011, 21-November-2011
pub fn parse_global_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map = get_month_map();
    
    // Helper to parse timezone - use common function
    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
        parse_timezone(py, tz_str)
    };
    
    // Helper to parse AM/PM - use common function
    let parse_time_with_ampm = |time_str: &str| -> Result<(u8, u8, u8), PyErr> {
        crate::datetime::common::parse_time_with_ampm(time_str)
    };
    
    // Try numeric format: 21/11/2011 or 21-11-2011 with optional time/timezone
    if let Ok(re) = Regex::new(r"^(\d{1,2})[-/](\d{1,2})[-/](\d{4})(?:\s+(.+))?$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                
                let (hour, minute, second) = if let Some(time_part) = caps.get(4) {
                    let time_str = time_part.as_str().trim();
                    // Check if there's a timezone
                    if let Some(tz_match) = Regex::new(r"\s+([+-]\d{2}:?\d{2})$").ok().and_then(|re| re.captures(time_str)).and_then(|c| c.get(1)) {
                        let tz_str = tz_match.as_str();
                        let time_only = time_str[..time_str.len() - tz_str.len()].trim();
                        let (h, m, s) = parse_time_with_ampm(time_only)?;
                        let tzinfo = parse_tz(tz_str)?;
                        let dt = datetime_class.call1((year, month, day, h, m, s, 0, tzinfo))?;
                        return Ok(dt.to_object(py));
                    } else {
                        parse_time_with_ampm(time_str)?
                    }
                } else {
                    (0, 0, 0)
                };
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try named month format: 21-Nov-2011 or 21-November-2011
    if let Ok(re) = Regex::new(r"^(\d{1,2})[-/](Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|May|June|July|August|September|October|November|December)[-/](\d{4})(?:\s+(.+))?$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                
                let (hour, minute, second, tzinfo) = if let Some(time_part) = caps.get(4) {
                    let time_str = time_part.as_str().trim();
                    if let Some(tz_match) = Regex::new(r"\s+([+-]\d{2}:?\d{2})$").ok().and_then(|re| re.captures(time_str)).and_then(|c| c.get(1)) {
                        let tz_str = tz_match.as_str();
                        let time_only = time_str[..time_str.len() - tz_str.len()].trim();
                        let (h, m, s) = parse_time_with_ampm(time_only)?;
                        let tz = parse_tz(tz_str)?;
                        (h, m, s, tz)
                    } else {
                        let (h, m, s) = parse_time_with_ampm(time_str)?;
                        (h, m, s, py.None())
                    }
                } else {
                    (0, 0, 0, py.None())
                };
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid Global datetime: {}", value)))
}

