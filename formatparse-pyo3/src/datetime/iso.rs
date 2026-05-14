use crate::datetime::common::{create_fixed_tz, extract_microseconds};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use regex::Regex;

// Cached regex patterns for ISO 8601 datetime parsing
static RE_ISO_DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\d{4})-(\d{2})-(\d{2})$").unwrap());

static RE_ISO_DATETIME_Z: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?[Zz]$")
        .unwrap()
});

static RE_ISO_DATETIME_TZ_4DIGIT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?\s*([+-])(\d{2})(\d{2})$").unwrap()
});

static RE_ISO_DATETIME_TZ_COLON: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?\s*([+-])(\d{2}):(\d{2})$").unwrap()
});

static RE_ISO_DATETIME_NO_TZ: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?$").unwrap()
});

/// Parse ISO 8601 datetime string and return Python datetime object
pub fn parse_iso_datetime(py: Python, value: &str) -> PyResult<PyObject> {
    let datetime_module = py.import("datetime")?;
    let datetime_class = datetime_module.getattr("datetime")?;

    // Try to parse various ISO 8601 formats
    // YYYY-MM-DD
    if let Some(caps) = RE_ISO_DATE.captures(value) {
        if let (Some(year_match), Some(month_match), Some(day_match)) =
            (caps.get(1), caps.get(2), caps.get(3))
        {
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            // Return datetime with time 00:00:00
            let dt = datetime_class.call1((year, month, day, 0, 0, 0, 0, py.None()))?;
            return dt.into_py_any(py);
        }
    }

    // YYYY-MM-DDTHH:MM or YYYY-MM-DD HH:MM with optional timezone
    // First try with Z timezone
    if let Some(caps) = RE_ISO_DATETIME_Z.captures(value) {
        if let (
            Some(year_match),
            Some(month_match),
            Some(day_match),
            Some(hour_match),
            Some(minute_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
        ) {
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = caps
                .get(6)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let microsecond: u32 = extract_microseconds(caps.get(7));

            let tzinfo = create_fixed_tz(py, 0, "UTC")?;
            let dt = datetime_class.call1((
                year,
                month,
                day,
                hour,
                minute,
                second,
                microsecond,
                tzinfo,
            ))?;
            return dt.into_py_any(py);
        }
    }

    // Try with timezone offset +0100 or -0530 (4 digits) - allow optional space
    if let Some(caps) = RE_ISO_DATETIME_TZ_4DIGIT.captures(value) {
        if let (
            Some(year_match),
            Some(month_match),
            Some(day_match),
            Some(hour_match),
            Some(minute_match),
            Some(tz_sign),
            Some(tz_hour_match),
            Some(tz_min_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
            caps.get(8),
            caps.get(9),
            caps.get(10),
        ) {
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = caps
                .get(6)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let microsecond: u32 = extract_microseconds(caps.get(7));

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

            let dt = datetime_class.call1((
                year,
                month,
                day,
                hour,
                minute,
                second,
                microsecond,
                tzinfo,
            ))?;
            return dt.into_py_any(py);
        }
    }

    // Try with timezone offset +01:00 or -05:30 (with colon) - allow optional space before timezone
    if let Some(caps) = RE_ISO_DATETIME_TZ_COLON.captures(value) {
        if let (
            Some(year_match),
            Some(month_match),
            Some(day_match),
            Some(hour_match),
            Some(minute_match),
            Some(tz_sign),
            Some(tz_hour_match),
            Some(tz_min_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
            caps.get(8),
            caps.get(9),
            caps.get(10),
        ) {
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = caps
                .get(6)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let microsecond: u32 = extract_microseconds(caps.get(7));

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

            let dt = datetime_class.call1((
                year,
                month,
                day,
                hour,
                minute,
                second,
                microsecond,
                tzinfo,
            ))?;
            return dt.into_py_any(py);
        }
    }

    // Try without timezone
    if let Some(caps) = RE_ISO_DATETIME_NO_TZ.captures(value) {
        if let (
            Some(year_match),
            Some(month_match),
            Some(day_match),
            Some(hour_match),
            Some(minute_match),
        ) = (
            caps.get(1),
            caps.get(2),
            caps.get(3),
            caps.get(4),
            caps.get(5),
        ) {
            let year: i32 = year_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid year"))?;
            let month: u8 = month_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid month"))?;
            let day: u8 = day_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid day"))?;
            let hour: u8 = hour_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid hour"))?;
            let minute: u8 = minute_match
                .as_str()
                .parse()
                .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid minute"))?;
            let second: u8 = caps
                .get(6)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let microsecond: u32 = extract_microseconds(caps.get(7));

            let dt = datetime_class.call1((
                year,
                month,
                day,
                hour,
                minute,
                second,
                microsecond,
                py.None(),
            ))?;
            return dt.into_py_any(py);
        }
    }

    Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "Invalid ISO 8601 datetime: {}",
        value
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re_iso_date() {
        assert!(RE_ISO_DATE.is_match("2023-12-25"));
        assert!(!RE_ISO_DATE.is_match("2023-1-1")); // Must be zero-padded
        assert!(!RE_ISO_DATE.is_match("2023-12-25T10:00:00"));
    }

    #[test]
    fn test_re_iso_datetime_z() {
        assert!(RE_ISO_DATETIME_Z.is_match("2023-12-25T10:30:00Z"));
        assert!(RE_ISO_DATETIME_Z.is_match("2023-12-25 10:30:00z"));
        assert!(RE_ISO_DATETIME_Z.is_match("2023-12-25T10:30Z"));
        assert!(RE_ISO_DATETIME_Z.is_match("2023-12-25T10:30:45.123456Z"));
    }

    #[test]
    fn test_re_iso_datetime_tz_4digit() {
        assert!(RE_ISO_DATETIME_TZ_4DIGIT.is_match("2023-12-25T10:30:00+0100"));
        assert!(RE_ISO_DATETIME_TZ_4DIGIT.is_match("2023-12-25T10:30:00 -0530"));
        assert!(RE_ISO_DATETIME_TZ_4DIGIT.is_match("2023-12-25 10:30:00+1000"));
    }

    #[test]
    fn test_re_iso_datetime_tz_colon() {
        assert!(RE_ISO_DATETIME_TZ_COLON.is_match("2023-12-25T10:30:00+01:00"));
        assert!(RE_ISO_DATETIME_TZ_COLON.is_match("2023-12-25T10:30:00 -05:30"));
        assert!(RE_ISO_DATETIME_TZ_COLON.is_match("2023-12-25 10:30:00+10:00"));
    }

    #[test]
    fn test_re_iso_datetime_no_tz() {
        assert!(RE_ISO_DATETIME_NO_TZ.is_match("2023-12-25T10:30:00"));
        assert!(RE_ISO_DATETIME_NO_TZ.is_match("2023-12-25 10:30:00"));
        assert!(RE_ISO_DATETIME_NO_TZ.is_match("2023-12-25T10:30"));
        assert!(RE_ISO_DATETIME_NO_TZ.is_match("2023-12-25T10:30:45.123456"));
    }
}
