//! JSON conversion utilities for Python bindings
//!
//! This module provides Rust-native JSON conversion functions that are faster
//! than using Python's json module.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};

/// Convert a Python object to serde_json::Value using Rust
/// This is faster than using Python's json module
pub fn py_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    // None
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // Bool (must check before int, as bool is subclass of int in Python)
    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(serde_json::Value::Bool(b.is_true()));
    }

    // Int
    if let Ok(i) = obj.downcast::<PyInt>() {
        if let Ok(v) = i.extract::<i64>() {
            return Ok(serde_json::Value::Number(v.into()));
        }
        if let Ok(v) = i.extract::<u64>() {
            return Ok(serde_json::Value::Number(v.into()));
        }
        // Fall back to float for very large integers
        if let Ok(v) = i.extract::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(v) {
                return Ok(serde_json::Value::Number(n));
            }
        }
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Integer too large for JSON",
        ));
    }

    // Float
    if let Ok(f) = obj.downcast::<PyFloat>() {
        let v: f64 = f.extract()?;
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Ok(serde_json::Value::Number(n));
        }
        // NaN and Infinity are not valid JSON
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Float value is not valid JSON (NaN or Infinity)",
        ));
    }

    // String
    if let Ok(s) = obj.downcast::<PyString>() {
        let v: String = s.extract()?;
        return Ok(serde_json::Value::String(v));
    }

    // Bytes -> base64 string
    if let Ok(b) = obj.downcast::<PyBytes>() {
        let bytes: &[u8] = b.as_bytes();
        // Use base64 encoding for bytes
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(serde_json::Value::String(encoded));
    }

    // List
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_json_value(&item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Dict
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyTypeError, _>("Dict keys must be strings")
            })?;
            map.insert(key_str, py_to_json_value(&value)?);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Try to convert other types via their __dict__ or repr
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Cannot convert type to JSON: {:?}",
        obj.get_type().name()
    )))
}

/// Convert serde_json::Value to Python object
pub fn json_value_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid JSON number",
                ))
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, value) in map {
                dict.set_item(key, json_value_to_py(py, value)?)?;
            }
            Ok(dict.into())
        }
    }
}

/// Serialize Python object to JSON bytes using Rust's serde_json
#[pyfunction]
pub fn json_dumps(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    serde_json::to_string(&value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Serialize Python object to pretty JSON string
#[pyfunction]
pub fn json_dumps_pretty(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    serde_json::to_string_pretty(&value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Deserialize JSON string to Python object using Rust's serde_json
#[pyfunction]
pub fn json_loads(py: Python<'_>, s: &str) -> PyResult<PyObject> {
    let value: serde_json::Value = serde_json::from_str(s)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    json_value_to_py(py, &value)
}
