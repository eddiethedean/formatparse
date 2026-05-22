//! LRU cache for compiled [`FormatParser`](crate::parser::FormatParser) instances.

use crate::parser::FormatParser;
use crate::pattern_normalize;
use lru::LruCache;
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

static PATTERN_CACHE: Lazy<Mutex<LruCache<u64, Arc<FormatParser>>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())));

fn lock_pattern_cache(
) -> Result<std::sync::MutexGuard<'static, LruCache<u64, Arc<FormatParser>>>, PyErr> {
    PATTERN_CACHE.lock().map_err(|_| {
        pyo3::exceptions::PyRuntimeError::new_err("formatparse pattern cache mutex was poisoned")
    })
}

/// Sorted `(type_name, with_pattern regex, regex_group_count tag)` for cache identity.
/// Must stay aligned with [`create_cache_key_hash`].
pub(crate) fn extract_extra_types_identity(
    py: Python<'_>,
    extra_types: &Option<HashMap<String, Py<PyAny>>>,
) -> Vec<(String, String, i64)> {
    let mut out = Vec::new();
    if let Some(extra_types) = extra_types {
        for (name, converter_obj) in extra_types {
            let converter_ref = converter_obj.bind(py);
            let pat = converter_ref
                .getattr("pattern")
                .ok()
                .and_then(|a| a.extract::<String>().ok())
                .unwrap_or_default();
            const GC_MISSING: i64 = -1;
            const GC_NONE: i64 = -2;
            let gc_tag = match converter_ref.getattr("regex_group_count") {
                Ok(v) => {
                    if v.is_none() {
                        GC_NONE
                    } else if let Ok(n) = v.extract::<i64>() {
                        n
                    } else {
                        GC_MISSING
                    }
                }
                Err(_) => GC_MISSING,
            };
            out.push((name.clone(), pat, gc_tag));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
    }
    out
}

/// Create a cache key hash from pattern and `extra_types`.
///
/// Must match what affects compilation in [`FormatParser::new_with_extra_types`]:
/// each converter's `pattern` string and `regex_group_count` (via
/// `validate_custom_type_pattern`). Keys alone are insufficient (same key, different
/// `with_pattern` / group count would incorrectly share a cached parser).
///
/// Callers must verify cache hits (see `FormatParser::matches_pattern_cache_request`)
/// because hash keys can theoretically collide.
fn create_cache_key_hash(
    py: Python<'_>,
    pattern: &str,
    extra_types: &Option<HashMap<String, Py<PyAny>>>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pattern.hash(&mut hasher);
    for (name, pat, gc_tag) in extract_extra_types_identity(py, extra_types) {
        name.hash(&mut hasher);
        pat.hash(&mut hasher);
        gc_tag.hash(&mut hasher);
    }
    hasher.finish()
}

/// Get or create a FormatParser from cache
pub(crate) fn get_or_create_parser(
    pattern: &str,
    extra_types: Option<HashMap<String, Py<PyAny>>>,
) -> PyResult<Arc<FormatParser>> {
    let normalized = pattern_normalize::prepare_compiled_pattern(pattern)?;
    Python::attach(|py| -> PyResult<Arc<FormatParser>> {
        let cache_key = create_cache_key_hash(py, &normalized, &extra_types);

        let cached = {
            let mut cache = lock_pattern_cache()?;
            cache.get(&cache_key).cloned()
        };

        if let Some(cached_parser) = cached {
            if cached_parser.matches_pattern_cache_request(py, &normalized, &extra_types) {
                return Ok(cached_parser);
            }
            let warnings = py.import("warnings")?;
            let msg = "formatparse pattern cache: hash collision or stale entry (e.g. \
                       mutated converter.pattern on a cached extra_types dict); recompiling";
            warnings.call_method1("warn", (msg,))?;
        }

        let parser = Arc::new(FormatParser::new_with_extra_types(
            &normalized,
            extra_types,
        )?);

        {
            let mut cache = lock_pattern_cache()?;
            cache.put(cache_key, parser.clone());
        }

        Ok(parser)
    })
}
