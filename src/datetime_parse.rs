use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use crate::types::strftime_to_regex;

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
        // Handle formats: +1:00, +10:00, +10:30, +1000, etc.
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
        // Also handle 4-digit format: +1000 (1 hour, 00 minutes)
        if let Ok(re) = Regex::new(r"([+-])(\d{4})") {
            if let Some(caps) = re.captures(tz_str) {
                if let (Some(sign_match), Some(tz_match)) = (caps.get(1), caps.get(2)) {
                    let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                    let tz_str = tz_match.as_str();
                    let hour: i32 = tz_str[..2].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                    let min: i32 = tz_str[2..].parse().unwrap_or(0);
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
                    let hour: i32 = tz_str[..2].parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone"))?;
                    let min: i32 = tz_str[2..].parse().unwrap_or(0);
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
    let today = datetime_class.call_method0("today")?;
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

/// Check if a PyErr is a regex group redefinition error from strptime
fn is_regex_group_redefinition_error(err: &PyErr) -> bool {
    let err_str = err.to_string();
    err_str.contains("redefinition of group name") || err_str.contains("re.error")
}

/// Fallback parser for strftime format strings when strptime fails due to regex group conflicts
/// This manually parses the format string and extracts datetime components
fn parse_strftime_fallback(py: Python, value: &str, format_str: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let date_class = datetime_module.getattr("date")?;
    let time_class = datetime_module.getattr("time")?;
    
    // Month name mapping
    let month_map: std::collections::HashMap<&str, u8> = [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4),
        ("May", 5), ("Jun", 6), ("Jul", 7), ("Aug", 8),
        ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
        ("January", 1), ("February", 2), ("March", 3), ("April", 4),
        ("June", 6), ("July", 7), ("August", 8),
        ("September", 9), ("October", 10), ("November", 11), ("December", 12),
    ].iter().cloned().collect();
    
    // Build a regex with capturing groups for format codes we need to extract
    let mut regex_parts = Vec::new();
    let mut format_code_groups: Vec<(char, usize)> = Vec::new();
    let mut group_index = 1;
    
    let mut chars = format_str.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let Some(next_ch) = chars.next() {
                match next_ch {
                    'Y' => {
                        regex_parts.push(r"(\d{4})".to_string());
                        format_code_groups.push(('Y', group_index));
                        group_index += 1;
                    },
                    'y' => {
                        regex_parts.push(r"(\d{2})".to_string());
                        format_code_groups.push(('y', group_index));
                        group_index += 1;
                    },
                    'm' => {
                        regex_parts.push(r"(\d{1,2})".to_string());
                        format_code_groups.push(('m', group_index));
                        group_index += 1;
                    },
                    'd' => {
                        regex_parts.push(r"(\d{1,2})".to_string());
                        format_code_groups.push(('d', group_index));
                        group_index += 1;
                    },
                    'H' => {
                        regex_parts.push(r"(\d{1,2})".to_string());
                        format_code_groups.push(('H', group_index));
                        group_index += 1;
                    },
                    'M' => {
                        regex_parts.push(r"(\d{1,2})".to_string());
                        format_code_groups.push(('M', group_index));
                        group_index += 1;
                    },
                    'S' => {
                        regex_parts.push(r"(\d{1,2})".to_string());
                        format_code_groups.push(('S', group_index));
                        group_index += 1;
                    },
                    'f' => {
                        regex_parts.push(r"(\d{1,6})".to_string());
                        format_code_groups.push(('f', group_index));
                        group_index += 1;
                    },
                    'b' | 'h' => {
                        regex_parts.push(r"([A-Za-z]{3})".to_string());
                        format_code_groups.push(('b', group_index));
                        group_index += 1;
                    },
                    'B' => {
                        regex_parts.push(r"([A-Za-z]+)".to_string());
                        format_code_groups.push(('B', group_index));
                        group_index += 1;
                    },
                    'a' | 'A' | 'w' | 'j' | 'U' | 'W' | 'c' | 'x' | 'X' | '%' => {
                        // These are matched but we don't need to extract them for datetime construction
                        let pattern = match next_ch {
                            'a' => r"[A-Za-z]{3}",
                            'A' => r"[A-Za-z]+",
                            'w' => r"\d",
                            'j' => r"\d{1,3}",
                            'U' | 'W' => r"\d{2}",
                            'c' | 'x' | 'X' => r".+",
                            '%' => "%",
                            _ => ".+?",
                        };
                        regex_parts.push(pattern.to_string());
                    },
                    _ => {
                        regex_parts.push(r".+?".to_string());
                    }
                }
            }
        } else {
            regex_parts.push(regex::escape(&ch.to_string()));
        }
    }
    
    let full_regex = format!("^{}$", regex_parts.join(""));
    let re = Regex::new(&full_regex)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid regex: {}", e)))?;
    
    let captures = re.captures(value)
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Value '{}' does not match format '{}'", value, format_str)))?;
    
    // Extract datetime components
    let mut year: Option<i32> = None;
    let mut month: Option<u8> = None;
    let mut day: Option<u8> = None;
    let mut hour: Option<u8> = None;
    let mut minute: Option<u8> = None;
    let mut second: Option<u8> = None;
    let mut microsecond: Option<u32> = None;
    
    for (code, group_idx) in format_code_groups {
        if let Some(cap) = captures.get(group_idx) {
            let val_str = cap.as_str();
            match code {
                'Y' => {
                    year = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?);
                },
                'y' => {
                    let yy: i32 = val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                    // Convert 2-digit year to 4-digit (assume 2000s for 00-68, 1900s for 69-99)
                    year = Some(if yy <= 68 { 2000 + yy } else { 1900 + yy });
                },
                'm' => {
                    month = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?);
                },
                'd' => {
                    day = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?);
                },
                'H' => {
                    hour = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?);
                },
                'M' => {
                    minute = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?);
                },
                'S' => {
                    second = Some(val_str.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?);
                },
                'f' => {
                    // Pad microseconds to 6 digits
                    let micros_str = if val_str.len() > 6 {
                        &val_str[..6]
                    } else {
                        val_str
                    };
                    let padded = format!("{:0<6}", micros_str);
                    microsecond = Some(padded.parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid microsecond"))?);
                },
                'b' | 'B' => {
                    month = month_map.get(val_str).copied();
                },
                _ => {}
            }
        }
    }
    
    // Determine what to return based on what components we have
    let has_time = hour.is_some() || minute.is_some() || second.is_some() || microsecond.is_some();
    let has_date = year.is_some() || month.is_some() || day.is_some();
    
    if has_time && !has_date {
        // Time only
        let time_obj = time_class.call1((
            hour.unwrap_or(0),
            minute.unwrap_or(0),
            second.unwrap_or(0),
            microsecond.unwrap_or(0)
        ))?;
        Ok(time_obj.to_object(py))
    } else if has_date && !has_time {
        // Date only
        let year_val = year.unwrap_or(1970);
        let month_val = month.unwrap_or(1);
        let day_val = day.unwrap_or(1);
        let date = date_class.call1((year_val, month_val, day_val))?;
        Ok(date.to_object(py))
    } else {
        // Both date and time (or neither - default to datetime)
        let year_val = year.unwrap_or(1970);
        let month_val = month.unwrap_or(1);
        let day_val = day.unwrap_or(1);
        let dt = datetime_class.call1((
            year_val,
            month_val,
            day_val,
            hour.unwrap_or(0),
            minute.unwrap_or(0),
            second.unwrap_or(0),
            microsecond.unwrap_or(0),
            py.None()
        ))?;
        Ok(dt.to_object(py))
    }
}

/// Parse strftime-style datetime using Python's strptime
pub fn parse_strftime_datetime(py: Python, value: &str, format_str: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    let date_class = datetime_module.getattr("date")?;
    let time_class = datetime_module.getattr("time")?;
    
    // Determine if format contains time components
    let has_time = format_str.contains("%H") || format_str.contains("%M") || format_str.contains("%S") || format_str.contains("%f");
    let has_date = format_str.contains("%Y") || format_str.contains("%y") || format_str.contains("%m") || format_str.contains("%d") || format_str.contains("%j");
    
    // Handle time-only patterns
    if has_time && !has_date {
        // Time-only: parse and return time object
        // strptime requires both date and time, so add a dummy date
        // Handle %f (microseconds) specially - it needs a dot, not colon
        let mut adjusted_format = format_str.to_string();
        let mut adjusted_value = value.to_string();
        
        // If format has %f but value uses colon separator, convert colon to dot before %f
        if format_str.contains("%f") && value.contains(':') {
            // Find the position of %f in format
            if let Some(f_pos) = adjusted_format.find("%f") {
                // Check if there's a colon before %f that should be a dot
                // Format like "%M:%S:%f" should become "%M:%S.%f" for value "23:27:123456"
                // But we need to check the value structure
                if adjusted_value.matches(':').count() >= 2 {
                    // Replace the last colon before microseconds with a dot
                    let mut last_colon_pos = 0;
                    for (i, ch) in adjusted_value.char_indices().rev() {
                        if ch == ':' {
                            last_colon_pos = i;
                            break;
                        }
                    }
                    if last_colon_pos > 0 {
                        adjusted_value.replace_range(last_colon_pos..last_colon_pos+1, ".");
                        // Also update format if needed
                        if let Some(format_colon_pos) = adjusted_format[..f_pos].rfind(':') {
                            adjusted_format.replace_range(format_colon_pos..format_colon_pos+1, ".");
                        }
                    }
                }
            }
        }
        
        let dummy_format = if adjusted_format.contains("%H") {
            adjusted_format.clone()
        } else {
            format!("%H:{}", adjusted_format) // Add hour if missing (defaults to 0)
        };
        let dummy_value = if adjusted_value.matches(':').count() < 2 && !adjusted_format.contains("%H") {
            format!("0:{}", adjusted_value) // Add hour 0 if missing
        } else {
            adjusted_value
        };
        
        // Add dummy date prefix for strptime
        let full_format = format!("1970-01-01 {}", dummy_format);
        let full_value = format!("1970-01-01 {}", dummy_value);
        
        let strptime = datetime_class.getattr("strptime")?;
        match strptime.call1((full_value.as_str(), full_format.as_str())) {
            Ok(dt) => {
                let hour: u8 = dt.getattr("hour")?.extract().unwrap_or(0);
                let minute: u8 = dt.getattr("minute")?.extract().unwrap_or(0);
                let second: u8 = dt.getattr("second")?.extract().unwrap_or(0);
                let microsecond: u32 = dt.getattr("microsecond")?.extract().unwrap_or(0);
                let time_obj = time_class.call1((hour, minute, second, microsecond))?;
                Ok(time_obj.to_object(py))
            },
            Err(e) => {
                // Check if this is a regex group redefinition error
                if is_regex_group_redefinition_error(&e) {
                    // Fall back to manual parsing
                    parse_strftime_fallback(py, value, format_str)
                } else {
                    Err(e)
                }
            }
        }
    } else if has_date && !has_time {
        // Date-only: parse and return date object
        // Handle %j (day of year) specially
        if format_str.contains("%j") {
            // Parse day of year format
            if let Ok(re) = Regex::new(r"^(\d{4})/(\d{1,3})$") {
                if let Some(caps) = re.captures(value) {
                    let year: i32 = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                    let day_of_year: u16 = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day of year"))?;
                    // Create date from year and day of year
                    let jan1 = date_class.call1((year, 1, 1))?;
                    let timedelta = datetime_module.getattr("timedelta")?;
                    let days = timedelta.call1((day_of_year as i32 - 1,))?;
                    let add_method = jan1.getattr("__add__")?;
                    let result_date = add_method.call1((days,))?;
                    return Ok(result_date.to_object(py));
                }
            }
            // Handle %j without year (use current year)
            if let Ok(re) = Regex::new(r"^(\d{1,3})$") {
                if let Some(caps) = re.captures(value) {
                    let today = datetime_class.call_method0("today")?;
                    let year: i32 = today.getattr("year")?.extract()?;
                    let day_of_year: u16 = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day of year"))?;
                    let jan1 = date_class.call1((year, 1, 1))?;
                    let timedelta = datetime_module.getattr("timedelta")?;
                    let days = timedelta.call1((day_of_year as i32 - 1,))?;
                    let add_method = jan1.getattr("__add__")?;
                    let result_date = add_method.call1((days,))?;
                    return Ok(result_date.to_object(py));
                }
            }
        }
        
        // Use datetime.strptime for parsing dates
        // Try with the original format first
        let strptime = datetime_class.getattr("strptime")?;
        match strptime.call1((value, format_str)) {
            Ok(dt) => {
                // Convert datetime to date
                let year: i32 = dt.getattr("year")?.extract()?;
                let month: u8 = dt.getattr("month")?.extract()?;
                let day: u8 = dt.getattr("day")?.extract()?;
                let date = date_class.call1((year, month, day))?;
                Ok(date.to_object(py))
            },
            Err(e) => {
                // Check if this is a regex group redefinition error
                if is_regex_group_redefinition_error(&e) {
                    // Fall back to manual parsing
                    return parse_strftime_fallback(py, value, format_str);
                }
                // If strptime fails for other reasons, try to parse manually for flexible formats
                // Handle single-digit months/days by trying flexible parsing
                // For formats like %Y/%m/%d, try to parse with regex
                if format_str.contains("%Y") && format_str.contains("%m") && format_str.contains("%d") {
                    // Try to parse YYYY/MM/DD or YYYY/M/D format (flexible separators)
                    // Match the separator used in format_str
                    let sep = if format_str.contains('/') { "/" } else if format_str.contains('-') { "-" } else { "/" };
                    let pattern = format!(r"^(\d{{4}})\{}(\d{{1,2}})\{}(\d{{1,2}})$", regex::escape(sep), regex::escape(sep));
                    if let Ok(re) = Regex::new(&pattern) {
                        if let Some(caps) = re.captures(value) {
                            let year: i32 = caps.get(1).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
                            let month: u8 = caps.get(2).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
                            let day: u8 = caps.get(3).unwrap().as_str().parse().map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
                            let date = date_class.call1((year, month, day))?;
                            return Ok(date.to_object(py));
                        }
                    }
                }
                // If all else fails, return the original error
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid date format: {} with format {}", value, format_str)))
            }
        }
    } else {
        // Both date and time: return datetime
        let strptime = datetime_class.getattr("strptime")?;
        match strptime.call1((value, format_str)) {
            Ok(dt) => Ok(dt.to_object(py)),
            Err(e) => {
                // Check if this is a regex group redefinition error
                if is_regex_group_redefinition_error(&e) {
                    // Fall back to manual parsing
                    parse_strftime_fallback(py, value, format_str)
                } else {
                    Err(e)
                }
            }
        }
    }
}

