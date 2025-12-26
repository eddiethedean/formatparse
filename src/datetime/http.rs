use pyo3::prelude::*;
use regex::Regex;
use crate::datetime::common::{get_abbreviated_month_map, create_fixed_tz};

/// Parse HTTP log format: 21/Nov/2011:10:21:36 +1000
pub fn parse_http_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import_bound("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;
    
    let month_map = get_abbreviated_month_map();
    
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

