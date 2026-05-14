use crate::datetime::common::{
    create_fixed_tz, get_month_map, RE_TZ_4DIGIT, RE_TZ_COLON, RE_TZ_IN_STRING_EXTENDED,
};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use regex::Regex;

// Cached regex patterns for US datetime parsing
static RE_US_NUMERIC: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d{1,2})[-/](\d{1,2})[-/](\d{4})(?:\s+(.+))?$").unwrap());

static RE_US_NAMED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec|January|February|March|April|May|June|July|August|September|October|November|December)[-/](\d{1,2})[-/](\d{4})(?:\s+(.+))?$").unwrap()
});

/// Parse US (month/day) datetime format - similar to global but different order
pub fn parse_us_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;

    let month_map = get_month_map();

    let parse_tz = |tz_str: &str| -> PyResult<PyObject> {
        // Handle formats: +1000, +10:00, +10:30, etc.
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
        // Also handle 4-digit format: +1000 (10 hours, 00 minutes)
        if let Some(caps) = RE_TZ_4DIGIT.captures(tz_str) {
            if let (Some(sign_match), Some(tz_match)) = (caps.get(1), caps.get(2)) {
                let sign = if sign_match.as_str() == "+" { 1 } else { -1 };
                let tz_str = tz_match.as_str();
                // Regex ensures exactly 4 digits, but add defensive check
                if tz_str.len() >= 4 {
                    let hour: i32 = tz_str[..2].parse().map_err(|_| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone")
                    })?;
                    let min: i32 = tz_str[2..4].parse().map_err(|_| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone")
                    })?;
                    let offset_minutes = sign * (hour * 60 + min);
                    return create_fixed_tz(py, offset_minutes, "");
                }
            }
        }
        Ok(py.None())
    };

    let parse_time_with_ampm = |time_str: &str| -> Result<(u8, u8, u8), PyErr> {
        crate::datetime::common::parse_time_with_ampm(time_str)
    };

    // Numeric format: 11/21/2011 or 11-21-2011 (month/day/year)
    if let Some(caps) = RE_US_NUMERIC.captures(value) {
        if let (Some(month_match), Some(day_match), Some(year_match)) =
            (caps.get(1), caps.get(2), caps.get(3))
        {
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;

            let (hour, minute, second, tzinfo) = if let Some(time_part) = caps.get(4) {
                let time_str = time_part.as_str().trim();
                // Try to match timezone: +1000, +10:00, +10:30, etc.
                if let Some(tz_match) = RE_TZ_IN_STRING_EXTENDED
                    .captures(time_str)
                    .and_then(|c| c.get(1))
                {
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
            return dt.into_py_any(py);
        }
    }

    // Named month format: Nov-21-2011 or November-21-2011 (month-day-year)
    if let Some(caps) = RE_US_NAMED.captures(value) {
        if let (Some(month_match), Some(day_match), Some(year_match)) =
            (caps.get(1), caps.get(2), caps.get(3))
        {
            let month_name = month_match.as_str();
            let month = *month_map.get(month_name).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid month: {}",
                    month_name
                ))
            })?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;

            let (hour, minute, second, tzinfo) = if let Some(time_part) = caps.get(4) {
                let time_str = time_part.as_str().trim();
                // Try to match timezone: +1000, +10:00, +10:30, etc.
                if let Some(tz_match) = RE_TZ_IN_STRING_EXTENDED
                    .captures(time_str)
                    .and_then(|c| c.get(1))
                {
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
            return dt.into_py_any(py);
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "Invalid US datetime: {}",
        value
    )))
}
