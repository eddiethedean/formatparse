use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::{create_fixed_tz, extract_microseconds};

/// Parse ISO 8601 datetime string and return Python datetime object
pub fn parse_iso_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    // Try to parse various ISO 8601 formats
    // YYYY-MM-DD
    if let Ok(re) = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(year_match), Some(month_match), Some(day_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                // Return datetime with time 00:00:00
                let dt = datetime_class.call1((year, month, day, 0, 0, 0, 0, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // YYYY-MM-DDTHH:MM or YYYY-MM-DD HH:MM with optional timezone
    // First try with Z timezone
    if let Ok(re) = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?[Zz]$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(year_match), Some(month_match), Some(day_match), Some(hour_match), Some(minute_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5)) {
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = caps.get(6).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                let microsecond: u32 = extract_microseconds(caps.get(7));
                
                let tzinfo = create_fixed_tz(py, 0, "UTC")?;
                let dt = datetime_class.call1((year, month, day, hour, minute, second, microsecond, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try with timezone offset +0100 or -0530 (4 digits) - allow optional space
    if let Ok(re) = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?\s*([+-])(\d{2})(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(year_match), Some(month_match), Some(day_match), Some(hour_match), Some(minute_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(8), caps.get(9), caps.get(10)) {
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = caps.get(6).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                let microsecond: u32 = extract_microseconds(caps.get(7));
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute"))?;
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, microsecond, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try with timezone offset +01:00 or -05:30 (with colon) - allow optional space before timezone
    if let Ok(re) = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?\s*([+-])(\d{2}):(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(year_match), Some(month_match), Some(day_match), Some(hour_match), Some(minute_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(8), caps.get(9), caps.get(10)) {
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = caps.get(6).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                let microsecond: u32 = extract_microseconds(caps.get(7));
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute"))?;
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, microsecond, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try without timezone
    if let Ok(re) = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(year_match), Some(month_match), Some(day_match), Some(hour_match), Some(minute_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5)) {
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = caps.get(6).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                let microsecond: u32 = extract_microseconds(caps.get(7));
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, microsecond, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid ISO 8601 datetime: {}", value)))
}

