use crate::datetime::common::{create_fixed_tz, parse_time_with_ampm, RE_TZ_COLON};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use regex::Regex;

// Cached regex pattern for timezone in time string
static RE_TIME_TZ: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+([+-]\d{1,2}:?\d{2,4})$").unwrap());

/// Parse time format: 10:21:36, 10:21:36 AM, 10:21:36 PM, 10:21 - returns time object
pub fn parse_time(py: Python, value: &str) -> PyResult<Py<PyAny>> {
    let datetime_module = py.import("datetime")?;
    let time_class = datetime_module.getattr("time")?;

    let parse_tz = |tz_str: &str| -> PyResult<Py<PyAny>> {
        if let Some(caps) = RE_TZ_COLON.captures(tz_str) {
            if let (Some(sign_match), Some(hour_match), Some(min_match)) =
                (caps.get(1), caps.get(2), caps.get(3))
            {
                let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                let hour: i32 = hour_match.as_str().parse().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone")
                })?;
                let min: i32 = min_match.as_str().parse().unwrap_or(0);
                let offset_minutes = sign * (hour * 60 + min);
                return create_fixed_tz(py, offset_minutes, "");
            }
        }
        Ok(py.None())
    };

    // Check for timezone at the end
    let (time_str, tzinfo) =
        if let Some(tz_match) = RE_TIME_TZ.captures(value).and_then(|c| c.get(1)) {
            let tz_str = tz_match.as_str();
            let time_only = value[..value.len() - tz_str.len()].trim();
            (time_only, parse_tz(tz_str)?)
        } else {
            (value.trim(), py.None())
        };

    let (hour, minute, second) = parse_time_with_ampm(time_str)?;

    let time_obj = time_class.call1((hour, minute, second, 0, tzinfo))?;
    time_obj.into_py_any(py)
}
