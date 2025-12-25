use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;

/// Parse ISO 8601 datetime string and return Python datetime object
pub fn parse_iso_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let date_class = datetime_module.getattr("date")?;
    
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
                let microsecond: u32 = caps.get(7).map(|m| {
                    let s = m.as_str();
                    let padded = format!("{:0<6}", s);
                    padded[..6.min(padded.len())].parse().unwrap_or(0)
                }).unwrap_or(0);
                
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
                let microsecond: u32 = caps.get(7).map(|m| {
                    let s = m.as_str();
                    let padded = format!("{:0<6}", s);
                    padded[..6.min(padded.len())].parse().unwrap_or(0)
                }).unwrap_or(0);
                
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
                let microsecond: u32 = caps.get(7).map(|m| {
                    let s = m.as_str();
                    let padded = format!("{:0<6}", s);
                    padded[..6.min(padded.len())].parse().unwrap_or(0)
                }).unwrap_or(0);
                
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
                let microsecond: u32 = caps.get(7).map(|m| {
                    let s = m.as_str();
                    let padded = format!("{:0<6}", s);
                    padded[..6.min(padded.len())].parse().unwrap_or(0)
                }).unwrap_or(0);
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, microsecond, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid ISO 8601 datetime: {}", value)))
}

fn create_fixed_tz(py: Python, offset_minutes: i32, name: &str) -> PyResult<PyObject> {
    let fixed_tz_module = py.import_bound("structparse")?;
    let fixed_tz_class = fixed_tz_module.getattr("FixedTzOffset")?;
    let tz = fixed_tz_class.call1((offset_minutes, name.to_string(),))?;
    Ok(tz.to_object(py))
}

/// Parse RFC2822 datetime string and return Python datetime object
/// Format: Mon, 21 Nov 2011 10:21:36 +1000
pub fn parse_rfc2822_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    // Map month abbreviations to numbers
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ].iter().cloned().collect();
    
    // Try with optional weekday prefix: Mon, 21 Nov 2011 10:21:36 +1000
    if let Ok(re) = Regex::new(r"^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2})(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match), Some(hour_match), Some(minute_match), Some(second_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(6), caps.get(7), caps.get(8), caps.get(9)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute"))?;
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try with timezone +10:00 format
    if let Ok(re) = Regex::new(r"^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2}):(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match), Some(hour_match), Some(minute_match), Some(second_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(6), caps.get(7), caps.get(8), caps.get(9)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute"))?;
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    // Try without weekday prefix: 21 Nov 2011 10:21:36 +1000
    if let Ok(re) = Regex::new(r"^(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2})(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match), Some(hour_match), Some(minute_match), Some(second_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(6), caps.get(7), caps.get(8), caps.get(9)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute"))?;
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid RFC2822 datetime: {}", value)))
}

/// Parse Global (day/month) datetime format
/// Formats: 21/11/2011, 21-11-2011, 21-Nov-2011, 21-November-2011
pub fn parse_global_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
        ("January", 1), ("February", 2), ("March", 3), ("April", 4),
        ("June", 6), ("July", 7), ("August", 8),
        ("September", 9), ("October", 10), ("November", 11), ("December", 12),
    ].iter().cloned().collect();
    
    // Helper to parse timezone
    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
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
        Ok(py.None())
    };
    
    // Helper to parse AM/PM
    let parse_time_with_ampm = |time_str: &str| -> Result<(u8, u8, u8), PyErr> {
        let mut hour = 0u8;
        let mut minute = 0u8;
        let mut second = 0u8;
        let mut is_pm = false;
        
        if let Some(ampm_idx) = time_str.to_uppercase().find("AM") {
            let time_part = &time_str[..ampm_idx].trim();
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_part) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                    // 12 AM becomes 0 (midnight), other AM hours stay as-is
                    if hour == 12 {
                        hour = 0;
                    }
                }
            }
        } else if let Some(pm_idx) = time_str.to_uppercase().find("PM") {
            is_pm = true;
            let time_part = &time_str[..pm_idx].trim();
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_part) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                    // 12 PM stays as 12 (noon), other PM hours add 12
                    if hour != 12 {
                        hour += 12;
                    }
                }
            }
        } else {
            // 24-hour format
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_str) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                }
            }
        }
        
        Ok((hour, minute, second))
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

/// Parse US (month/day) datetime format - similar to global but different order
pub fn parse_us_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    // Reuse global parser logic but swap day/month parsing
    // Actually, we need to parse month/day instead of day/month
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
        ("January", 1), ("February", 2), ("March", 3), ("April", 4),
        ("June", 6), ("July", 7), ("August", 8),
        ("September", 9), ("October", 10), ("November", 11), ("December", 12),
    ].iter().cloned().collect();
    
    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
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
        Ok(py.None())
    };
    
    let parse_time_with_ampm = |time_str: &str| -> Result<(u8, u8, u8), PyErr> {
        let mut hour = 0u8;
        let mut minute = 0u8;
        let mut second = 0u8;
        
        if let Some(ampm_idx) = time_str.to_uppercase().find("AM") {
            let time_part = &time_str[..ampm_idx].trim();
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_part) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    if hour == 12 {
                        hour = 0;
                    }
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                }
            }
        } else if let Some(pm_idx) = time_str.to_uppercase().find("PM") {
            let time_part = &time_str[..pm_idx].trim();
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_part) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    if hour != 12 {
                        hour += 12;
                    }
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                }
            }
        } else {
            if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
                if let Some(caps) = re.captures(time_str) {
                    hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                    minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                    second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
                }
            }
        }
        
        Ok((hour, minute, second))
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
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid US datetime: {}", value)))
}

/// Parse ctime() format: Mon Nov 21 10:21:36 2011
pub fn parse_ctime_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ].iter().cloned().collect();
    
    if let Ok(re) = Regex::new(r"^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun)\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})\s+(\d{4})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(month_match), Some(day_match), Some(hour_match), Some(minute_match), Some(second_match), Some(year_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(6)) {
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, py.None()))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid ctime datetime: {}", value)))
}

/// Parse HTTP log format: 21/Nov/2011:10:21:36 +1000
pub fn parse_http_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ].iter().cloned().collect();
    
    // 21/Nov/2011:10:21:36 +1000 or +10:00
    if let Ok(re) = Regex::new(r"^(\d{2})/(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)/(\d{4}):(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2}):?(\d{2})$") {
        if let Some(caps) = re.captures(value) {
            if let (Some(day_match), Some(month_match), Some(year_match), Some(hour_match), Some(minute_match), Some(second_match), Some(tz_sign), Some(tz_hour_match), Some(tz_min_match)) = 
                (caps.get(1), caps.get(2), caps.get(3), caps.get(4), caps.get(5), caps.get(6), caps.get(7), caps.get(8), caps.get(9)) {
                let day: u8 = day_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                let month_name = month_match.as_str();
                let month = *month_map.get(month_name).ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid month: {}", month_name)))?;
                let year: i32 = year_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                let hour: u8 = hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                let minute: u8 = minute_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                let second: u8 = second_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;
                
                let sign_str = tz_sign.as_str();
                let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour"))?;
                let tz_min: i32 = tz_min_match.as_str().parse().unwrap_or(0);
                let sign = if sign_str == "+" { 1 } else { -1 };
                let offset_minutes = sign * (tz_hour * 60 + tz_min);
                let tzinfo = create_fixed_tz(py, offset_minutes, "")?;
                
                let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
                return Ok(dt.to_object(py));
            }
        }
    }
    
    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid HTTP datetime: {}", value)))
}

/// Parse Linux system log format: Nov 21 10:21:36 (year is current year)
pub fn parse_system_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let today = datetime_module.call_method0("today")?;
    let current_year: i32 = today.getattr("year")?.extract()?;
    
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ].iter().cloned().collect();
    
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

/// Parse time format: 10:21:36, 10:21:36 AM, 10:21:36 PM, 10:21 - returns time object
pub fn parse_time(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let time_class = datetime_module.getattr("time")?;
    
    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
        if let Ok(re) = Regex::new(r"([+-])(\d{1,2}):?(\d{2})") {
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
        Ok(py.None())
    };
    
    // Check for timezone at the end
    let (time_str, tzinfo) = if let Some(tz_match) = Regex::new(r"\s+([+-]\d{1,2}:?\d{2,4})$").ok().and_then(|re| re.captures(value)).and_then(|c| c.get(1)) {
        let tz_str = tz_match.as_str();
        let time_only = value[..value.len() - tz_str.len()].trim();
        (time_only, parse_tz(tz_str)?)
    } else {
        (value.trim(), py.None())
    };
    
    let mut hour = 0u8;
    let mut minute = 0u8;
    let mut second = 0u8;
    
    // Parse AM/PM
    if let Some(ampm_idx) = time_str.to_uppercase().find("AM") {
        let time_part = time_str[..ampm_idx].trim();
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_part) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                if hour == 12 {
                    hour = 0;
                }
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            }
        }
    } else if let Some(pm_idx) = time_str.to_uppercase().find("PM") {
        let time_part = time_str[..pm_idx].trim();
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_part) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                if hour != 12 {
                    hour += 12;
                }
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            }
        }
    } else {
        // 24-hour format
        if let Ok(re) = Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2}))?") {
            if let Some(caps) = re.captures(time_str) {
                hour = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
                minute = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
                second = caps.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
            }
        }
    }
    
    let time_obj = time_class.call1((hour, minute, second, 0, tzinfo))?;
    Ok(time_obj.to_object(py))
}

/// Parse strftime-style datetime using Python's strptime
pub fn parse_strftime_datetime(py: Python, value: &str, format_str: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let date_class = datetime_module.getattr("date")?;
    
    // Use datetime.strptime for parsing (it can parse dates too)
    let strptime = datetime_class.getattr("strptime")?;
    
    // Determine if format contains time components
    let has_time = format_str.contains("%H") || format_str.contains("%M") || format_str.contains("%S");
    let has_date = format_str.contains("%Y") || format_str.contains("%y") || format_str.contains("%m") || format_str.contains("%d");
    
    match strptime.call1((value, format_str)) {
        Ok(dt) => {
            if has_time && has_date {
                // Return full datetime
                Ok(dt.to_object(py))
            } else if has_date {
                // Convert datetime to date
                let year: i32 = dt.getattr("year")?.extract()?;
                let month: u8 = dt.getattr("month")?.extract()?;
                let day: u8 = dt.getattr("day")?.extract()?;
                let date = date_class.call1((year, month, day))?;
                Ok(date.to_object(py))
            } else {
                Ok(dt.to_object(py))
            }
        },
        Err(e) => Err(e)
    }
}

