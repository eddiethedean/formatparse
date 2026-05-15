use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

pub fn get_nested_dict_value(
    named: &HashMap<String, PyObject>,
    path: &[String],
    py: Python,
) -> PyResult<Option<PyObject>> {
    if path.is_empty() {
        return Ok(None);
    }

    if path.len() == 1 {
        // Simple case - just get directly
        return Ok(named.get(&path[0]).map(|obj| obj.clone_ref(py)));
    }

    // Navigate through nested dicts
    let first_key = &path[0];
    let mut current_obj: PyObject = match named.get(first_key) {
        Some(v) => v.clone_ref(py),
        None => return Ok(None),
    };

    for key in path.iter().skip(1) {
        let current_dict = match current_obj.bind(py).downcast::<PyDict>() {
            Ok(d) => d,
            Err(_) => return Ok(None), // Not a dict, path doesn't exist
        };

        match current_dict.get_item(key.as_str())? {
            Some(v) => {
                // Get the PyObject to continue navigation
                current_obj = v.into();
            }
            None => return Ok(None), // Path doesn't exist
        }
    }

    Ok(Some(current_obj))
}

/// Insert a value into a nested dict structure in the named HashMap
pub fn insert_nested_dict(
    named: &mut HashMap<String, PyObject>,
    path: &[String],
    value: PyObject,
    py: Python,
) -> PyResult<()> {
    if path.is_empty() {
        return Ok(());
    }

    if path.len() == 1 {
        // Simple case - just insert directly
        named.insert(path[0].clone(), value);
        return Ok(());
    }

    // Need to create nested dicts
    let first_key = &path[0];

    // Get or create the top-level dict
    let top_dict = if let Some(existing) = named.get(first_key) {
        // Check if it's already a dict
        if let Ok(dict) = existing.bind(py).downcast::<PyDict>() {
            dict.clone()
        } else {
            // It's not a dict, we can't nest - this is an error case
            // For now, just replace it (this shouldn't happen in practice)
            let new_dict = PyDict::new(py);
            let new_dict_obj = new_dict.clone().into_py_any(py)?;
            named.insert(first_key.clone(), new_dict_obj);
            new_dict
        }
    } else {
        let new_dict = PyDict::new(py);
        let new_dict_obj = new_dict.clone().into_py_any(py)?;
        named.insert(first_key.clone(), new_dict_obj);
        new_dict
    };

    // Navigate/create nested dicts
    let mut current_dict = top_dict;
    for key in path.iter().skip(1).take(path.len() - 2) {
        let nested_dict = if let Some(existing) = current_dict.get_item(key.as_str())? {
            if let Ok(dict) = existing.downcast::<PyDict>() {
                dict.clone()
            } else {
                // Not a dict, replace it
                let new_dict = PyDict::new(py);
                let new_dict_obj = new_dict.clone().into_py_any(py)?;
                current_dict.set_item(key.as_str(), new_dict_obj)?;
                new_dict
            }
        } else {
            let new_dict = PyDict::new(py);
            let new_dict_obj = new_dict.clone().into_py_any(py)?;
            current_dict.set_item(key.as_str(), new_dict_obj)?;
            new_dict
        };
        current_dict = nested_dict;
    }

    // Set the final value
    let final_key = &path[path.len() - 1];
    current_dict.set_item(final_key.as_str(), value)?;

    Ok(())
}
