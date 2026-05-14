use crate::datetime::common::{create_fixed_tz, get_abbreviated_month_map};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use regex::Regex;

// Cached regex patterns for RFC2822 datetime parsing
static RE_RFC2822_WITH_WEEKDAY_4DIGIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2})(\d{2})$").unwrap()
});

static RE_RFC2822_WITH_WEEKDAY_COLON: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun),\s+(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2}):(\d{2})$").unwrap()
});

static RE_RFC2822_NO_WEEKDAY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{1,2})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{4})\s+(\d{2}):(\d{2}):(\d{2})\s+([+-])(\d{2})(\d{2})$").unwrap()
});

/// Parse RFC2822 datetime string and return Python datetime object
/// Format: Mon, 21 Nov 2011 10:21:36 +1000
pub fn parse_rfc2822_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;

    // Map month abbreviations to numbers
    let month_map = get_abbreviated_month_map();

    // Try with optional weekday prefix: Mon, 21 Nov 2011 10:21:36 +1000
    if let Some(caps) = RE_RFC2822_WITH_WEEKDAY_4DIGIT.captures(value) {
        if let (
            Some(day_match),
            Some(month_match),
            Some(year_match),
            Some(hour_match),
            Some(minute_match),
            Some(second_match),
            Some(tz_sign),
            Some(tz_hour_match),
            Some(tz_min_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
            caps.get(6),
            caps.get(7),
            caps.get(8),
            caps.get(9),
        ) {
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let month_name = month_match.as_str();
            let month = *month_map.get(month_name).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid month: {}",
                    month_name
                ))
            })?;
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = second_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;

            let sign_str = tz_sign.as_str();
            let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour")
            })?;
            let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute")
            })?;
            let sign = if sign_str == "+" { 1 } else { -1 };
            let offset_minutes = sign * (tz_hour * 60 + tz_min);
            let tzinfo = create_fixed_tz(py, offset_minutes, "")?;

            let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
            return dt.into_py_any(py);
        }
    }

    // Try with timezone +10:00 format
    if let Some(caps) = RE_RFC2822_WITH_WEEKDAY_COLON.captures(value) {
        if let (
            Some(day_match),
            Some(month_match),
            Some(year_match),
            Some(hour_match),
            Some(minute_match),
            Some(second_match),
            Some(tz_sign),
            Some(tz_hour_match),
            Some(tz_min_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
            caps.get(6),
            caps.get(7),
            caps.get(8),
            caps.get(9),
        ) {
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let month_name = month_match.as_str();
            let month = *month_map.get(month_name).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid month: {}",
                    month_name
                ))
            })?;
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = second_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;

            let sign_str = tz_sign.as_str();
            let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour")
            })?;
            let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute")
            })?;
            let sign = if sign_str == "+" { 1 } else { -1 };
            let offset_minutes = sign * (tz_hour * 60 + tz_min);
            let tzinfo = create_fixed_tz(py, offset_minutes, "")?;

            let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
            return dt.into_py_any(py);
        }
    }

    // Try without weekday prefix: 21 Nov 2011 10:21:36 +1000
    if let Some(caps) = RE_RFC2822_NO_WEEKDAY.captures(value) {
        if let (
            Some(day_match),
            Some(month_match),
            Some(year_match),
            Some(hour_match),
            Some(minute_match),
            Some(second_match),
            Some(tz_sign),
            Some(tz_hour_match),
            Some(tz_min_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
            caps.get(6),
            caps.get(7),
            caps.get(8),
            caps.get(9),
        ) {
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let month_name = month_match.as_str();
            let month = *month_map.get(month_name).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid month: {}",
                    month_name
                ))
            })?;
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = second_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid second"))?;

            let sign_str = tz_sign.as_str();
            let tz_hour: i32 = tz_hour_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone hour")
            })?;
            let tz_min: i32 = tz_min_match.as_str().parse().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid timezone minute")
            })?;
            let sign = if sign_str == "+" { 1 } else { -1 };
            let offset_minutes = sign * (tz_hour * 60 + tz_min);
            let tzinfo = create_fixed_tz(py, offset_minutes, "")?;

            let dt = datetime_class.call1((year, month, day, hour, minute, second, 0, tzinfo))?;
            return dt.into_py_any(py);
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "Invalid RFC2822 datetime: {}",
        value
    )))
}
