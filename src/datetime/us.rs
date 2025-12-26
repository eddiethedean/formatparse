use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::{get_month_map, create_fixed_tz};

/// Parse US (month/day) datetime format - similar to global but different order
pub fn parse_us_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map = get_month_map();
    
    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
        // Handle formats: +1000, +10:00, +10:30, etc.
        if let Ok(re) = Regex::new(r"([+-])(\d{2}):?(\d{2})") {
            if let Some(caps) = re.captures(tz_str) {
                if let (Some(sign_match), Some(hour_match), Some(min_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                    let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                    let hour: i32 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                    let min: i32 = min_match.as_str().parse().unwrap_or(0);
                    let offset_minutes = sign * (hour * 60 + min);
                    return create_fixed_tz(py, offset_minutes, "");
                }
            }
        }
        // Also handle 4-digit format: +1000 (10 hours, 00 minutes)
        if let Ok(re) = Regex::new(r"([+-])(\d{4})") {
            if let Some(caps) = re.captures(tz_str) {
                if let (Some(sign_match), Some(tz_match)) = (caps.get(1), caps.get(2)) {
                    let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                    let tz_str = tz_match.as_str();
                    // Regex ensures exactly 4 digits, but add defensive check
                    if tz_str.len() >= 4 {
                        let hour: i32 = tz_str[..2].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                        let min: i32 = tz_str[2..4].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                        let offset_minutes = sign * (hour * 60 + min);
                        return create_fixed_tz(py, offset_minutes, "");
                    }
                }
            }
        }
        Ok(py.None())
    };
    
    let parse_time_with_ampm = |time_str: &str| -> Result<(u8, u8, u8), PyErr> {
        crate::datetime::common::parse_time_with_ampm(time_str)
    };
    
    // Numeric format: 11/21/2011 or 11-21-2011 (month/day/year)
    if let Ok(re) = Regex::new(r"^(\d{1,2})[-/](\d{1,2})[-/](\d{4})(?:\s+(.+))?$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(month_match), Some(day_match), Some(year_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let month: u8 = month_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                
                let (hour, minute, second, tzinfo) = if let Some(time_part) = caps.get(4) {
                    let time_str = time_part.as_str().trim();
                    // Try to match timezone: +1000, +10:00, +10:30, etc.
                    if let Some(tz_match) = Regex::new(r"\s+([+-]\d{2}:?\d{2,4})$").ok().and_then(|re| re.captures(time_str)).and_then(|c| c.get(1)) {
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
    
    // Named month format: Nov-21-2011 or November-21-2011 (month-day-year)
    if let Ok(re) = Regex::new(r"^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|May|June|July|August|September|October|November|December)[-/](\d{1,2})[-/](\d{4})(?:\s+(.+))?$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(month_match), Some(day_match), Some(year_match)) = (caps.get(1), caps.get(2), caps.get(3)) {
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                
                let (hour, minute, second, tzinfo) = if let Some(time_part) = caps.get(4) {
                    let time_str = time_part.as_str().trim();
                    // Try to match timezone: +1000, +10:00, +10:30, etc.
                    if let Some(tz_match) = Regex::new(r"\s+([+-]\d{2}:?\d{2,4})$").ok().and_then(|re| re.captures(time_str)).and_then(|c| c.get(1)) {
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
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid US datetime: {}", value)))
}

