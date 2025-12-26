use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::get_month_map;

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
    let month_map = get_month_map();
    
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

