use crate::error;
use regex::Regex;
use pyo3::prelude::*;

/// Build a regex from a pattern string with DOTALL flag
pub fn build_regex(pattern: &str) -> PyResult<Regex> {
    let regex_with_flags = format!("(?s){}", pattern);
    Regex::new(&regex_with_flags).map_err(|e| error::regex_error(&e.to_string()))
}

/// Build a case-insensitive regex from a pattern string with DOTALL flag
pub fn build_case_insensitive_regex(pattern: &str) -> Option<Regex> {
    Regex::new(&format!("(?s)(?i){}", pattern)).ok()
}

/// Remove anchors and flags from a regex string for search operations
pub fn prepare_search_regex(regex_str: &str) -> String {
    let mut search_regex_str = regex_str.to_string();
    
    // Remove (?s) flag if present
    if search_regex_str.starts_with("(?s)") {
        search_regex_str = search_regex_str[4..].to_string();
    }
    
    // Remove ^ anchor
    if search_regex_str.starts_with("^") {
        search_regex_str = search_regex_str[1..].to_string();
    }
    
    // Remove $ anchor
    if search_regex_str.ends_with("$") {
        search_regex_str = search_regex_str[..search_regex_str.len()-1].to_string();
    }
    
    search_regex_str
}

/// Build a search regex (without anchors) with optional case sensitivity
pub fn build_search_regex(regex_str: &str, case_sensitive: bool) -> PyResult<Regex> {
    let search_regex_str = prepare_search_regex(regex_str);
    
    // Always add (?s) for DOTALL, and conditionally add (?i) for case insensitive
    let pattern = if case_sensitive {
        format!("(?s){}", search_regex_str)
    } else {
        format!("(?s)(?i){}", search_regex_str)
    };
    
    Regex::new(&pattern).map_err(|e| error::regex_error(&e.to_string()))
}

