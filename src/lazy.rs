use pyo3::exceptions::{PyIndexError, PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyList, PyString};
use std::sync::Arc;

// Type tags matching B-FAST wire specification
const TAG_DATETIME: u8 = 0xD1;
const TAG_DATE: u8 = 0xD2;
const TAG_TIME: u8 = 0xD3;
const TAG_UUID: u8 = 0xD4;
const TAG_DECIMAL: u8 = 0xD5;

/// Skips a single B-FAST binary value starting at `*offset`.
/// Advances `*offset` past the end of the value.
pub fn skip_value(data: &[u8], offset: &mut usize) -> Result<(), &'static str> {
    if *offset >= data.len() {
        return Err("Unexpected end of buffer");
    }
    let tag = data[*offset];
    *offset += 1;

    match tag {
        0x10 | 0x20 | 0x21 => Ok(()), // null, false, true
        0x30..=0x37 => Ok(()),        // small ints 0..=7
        0x38 => {
            // int64
            if *offset + 8 > data.len() {
                return Err("Unexpected end of int64");
            }
            *offset += 8;
            Ok(())
        }
        0x40 => {
            // float64
            if *offset + 8 > data.len() {
                return Err("Unexpected end of float64");
            }
            *offset += 8;
            Ok(())
        }
        0x50 => {
            // string
            if *offset + 4 > data.len() {
                return Err("Unexpected end of string length");
            }
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            if *offset + len > data.len() {
                return Err("Unexpected end of string payload");
            }
            *offset += len;
            Ok(())
        }
        0x60 => {
            // list
            if *offset + 4 > data.len() {
                return Err("Unexpected end of list length");
            }
            let count = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            for _ in 0..count {
                skip_value(data, offset)?;
            }
            Ok(())
        }
        0x70 => {
            // dict/object
            while *offset < data.len() && data[*offset] != 0x7F {
                if *offset + 4 > data.len() {
                    return Err("Unexpected end of dict key id");
                }
                *offset += 4; // skip key_id (u32)
                skip_value(data, offset)?;
            }
            if *offset >= data.len() {
                return Err("Unterminated dict (missing 0x7F)");
            }
            *offset += 1; // skip 0x7F
            Ok(())
        }
        0x80 => {
            // bytes
            if *offset + 4 > data.len() {
                return Err("Unexpected end of bytes length");
            }
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            if *offset + len > data.len() {
                return Err("Unexpected end of bytes payload");
            }
            *offset += len;
            Ok(())
        }
        0x90 => {
            // numpy float64 array
            if *offset + 4 > data.len() {
                return Err("Unexpected end of numpy array length");
            }
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            let byte_len = len * 8;
            if *offset + byte_len > data.len() {
                return Err("Unexpected end of numpy array payload");
            }
            *offset += byte_len;
            Ok(())
        }
        TAG_DATETIME | TAG_DATE | TAG_TIME | TAG_UUID | TAG_DECIMAL => {
            if *offset + 4 > data.len() {
                return Err("Unexpected end of tagged extended type length");
            }
            let len = u32::from_le_bytes(data[*offset..*offset + 4].try_into().unwrap()) as usize;
            *offset += 4;
            if *offset + len > data.len() {
                return Err("Unexpected end of tagged extended type payload");
            }
            *offset += len;
            Ok(())
        }
        _ => Err("Unknown tag encountered in B-FAST payload"),
    }
}

/// Zero-Allocation Virtual Proxy over raw B-FAST binary buffers.
/// Defers deserialization until fields or indices are accessed on-demand.
#[pyclass(name = "BFastLazyView")]
pub struct BFastLazyView {
    pub(crate) data: Arc<Vec<u8>>,
    pub(crate) offset: usize,
    pub(crate) string_table: Arc<Vec<String>>,
}

impl BFastLazyView {
    pub fn new(data: Arc<Vec<u8>>, offset: usize, string_table: Arc<Vec<String>>) -> Self {
        Self {
            data,
            offset,
            string_table,
        }
    }

    #[inline(always)]
    fn get_tag(&self) -> PyResult<u8> {
        if self.offset >= self.data.len() {
            return Err(PyValueError::new_err("Unexpected end of B-FAST buffer"));
        }
        Ok(self.data[self.offset])
    }

    fn decode_value_at(&self, py: Python, offset: usize) -> PyResult<PyObject> {
        if offset >= self.data.len() {
            return Err(PyValueError::new_err("Unexpected end of buffer"));
        }
        let tag = self.data[offset];

        match tag {
            0x10 => Ok(py.None()),
            0x20 => Ok(pyo3::types::PyBool::new(py, false)
                .to_owned()
                .into_any()
                .unbind()),
            0x21 => Ok(pyo3::types::PyBool::new(py, true)
                .to_owned()
                .into_any()
                .unbind()),
            0x30..=0x37 => {
                let val = (tag & 0x0F) as i64;
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            0x38 => {
                if offset + 9 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated int64"));
                }
                let val = i64::from_le_bytes(self.data[offset + 1..offset + 9].try_into().unwrap());
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            0x40 => {
                if offset + 9 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated float64"));
                }
                let val = f64::from_le_bytes(self.data[offset + 1..offset + 9].try_into().unwrap());
                Ok(val.into_pyobject(py)?.into_any().unbind())
            }
            0x50 => {
                if offset + 5 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated string length"));
                }
                let len = u32::from_le_bytes(self.data[offset + 1..offset + 5].try_into().unwrap())
                    as usize;
                let start = offset + 5;
                if start + len > self.data.len() {
                    return Err(PyValueError::new_err("Truncated string payload"));
                }
                let slice = &self.data[start..start + len];
                let s = std::str::from_utf8(slice)
                    .map_err(|e| PyValueError::new_err(format!("Invalid UTF-8: {e}")))?;
                Ok(PyString::new(py, s).to_owned().into_any().unbind())
            }
            0x80 => {
                if offset + 5 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated bytes length"));
                }
                let len = u32::from_le_bytes(self.data[offset + 1..offset + 5].try_into().unwrap())
                    as usize;
                let start = offset + 5;
                if start + len > self.data.len() {
                    return Err(PyValueError::new_err("Truncated bytes payload"));
                }
                Ok(PyBytes::new(py, &self.data[start..start + len])
                    .to_owned()
                    .into_any()
                    .unbind())
            }
            0x60 | 0x70 => {
                // Return nested BFastLazyView for sub-objects or sub-lists
                let child = BFastLazyView {
                    data: Arc::clone(&self.data),
                    offset,
                    string_table: Arc::clone(&self.string_table),
                };
                Ok(child.into_pyobject(py)?.into_any().unbind())
            }
            TAG_DATETIME | TAG_DATE | TAG_TIME | TAG_UUID | TAG_DECIMAL => {
                if offset + 5 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated tagged type length"));
                }
                let len = u32::from_le_bytes(self.data[offset + 1..offset + 5].try_into().unwrap())
                    as usize;
                let start = offset + 5;
                if start + len > self.data.len() {
                    return Err(PyValueError::new_err("Truncated tagged type payload"));
                }
                let slice = &self.data[start..start + len];
                let s = std::str::from_utf8(slice)
                    .map_err(|e| PyValueError::new_err(format!("Invalid UTF-8: {e}")))?;

                match tag {
                    TAG_DATETIME => {
                        let dt_cls = py.import("datetime")?.getattr("datetime")?;
                        let dt = dt_cls.call_method1("fromisoformat", (s,))?;
                        Ok(dt.into_any().unbind())
                    }
                    TAG_DATE => {
                        let d_cls = py.import("datetime")?.getattr("date")?;
                        let d = d_cls.call_method1("fromisoformat", (s,))?;
                        Ok(d.into_any().unbind())
                    }
                    TAG_TIME => {
                        let t_cls = py.import("datetime")?.getattr("time")?;
                        let t = t_cls.call_method1("fromisoformat", (s,))?;
                        Ok(t.into_any().unbind())
                    }
                    TAG_UUID => {
                        let uuid_cls = py.import("uuid")?.getattr("UUID")?;
                        let u = uuid_cls.call1((s,))?;
                        Ok(u.into_any().unbind())
                    }
                    TAG_DECIMAL => {
                        let dec_cls = py.import("decimal")?.getattr("Decimal")?;
                        let dec = dec_cls.call1((s,))?;
                        Ok(dec.into_any().unbind())
                    }
                    _ => Ok(PyString::new(py, s).to_owned().into_any().unbind()),
                }
            }
            0x90 => {
                // NumPy float64 array
                if offset + 5 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated numpy array length"));
                }
                let len = u32::from_le_bytes(self.data[offset + 1..offset + 5].try_into().unwrap())
                    as usize;
                let start = offset + 5;
                let byte_len = len * 8;
                if start + byte_len > self.data.len() {
                    return Err(PyValueError::new_err("Truncated numpy array payload"));
                }
                let list = PyList::empty(py);
                for i in 0..len {
                    let f = f64::from_le_bytes(
                        self.data[start + i * 8..start + (i + 1) * 8]
                            .try_into()
                            .unwrap(),
                    );
                    list.append(f)?;
                }
                Ok(list.into_any().unbind())
            }
            _ => Err(PyValueError::new_err(format!(
                "Unsupported tag in BFastLazyView: 0x{tag:02X}"
            ))),
        }
    }
}

#[pymethods]
impl BFastLazyView {
    /// Returns True if the view wraps a dictionary/object (tag 0x70).
    pub fn is_dict(&self) -> PyResult<bool> {
        Ok(self.get_tag()? == 0x70)
    }

    /// Returns True if the view wraps a list/array (tag 0x60).
    pub fn is_list(&self) -> PyResult<bool> {
        Ok(self.get_tag()? == 0x60)
    }

    /// Returns the number of items if list, or number of key-value pairs if dict.
    pub fn __len__(&self) -> PyResult<usize> {
        let tag = self.get_tag()?;
        if tag == 0x60 {
            if self.offset + 5 > self.data.len() {
                return Err(PyValueError::new_err("Truncated list header"));
            }
            let count = u32::from_le_bytes(
                self.data[self.offset + 1..self.offset + 5]
                    .try_into()
                    .unwrap(),
            ) as usize;
            Ok(count)
        } else if tag == 0x70 {
            let mut count = 0;
            let mut cur = self.offset + 1;
            while cur < self.data.len() && self.data[cur] != 0x7F {
                if cur + 4 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated dict key"));
                }
                cur += 4;
                skip_value(&self.data, &mut cur)
                    .map_err(|e| PyValueError::new_err(format!("Error parsing dict: {e}")))?;
                count += 1;
            }
            Ok(count)
        } else {
            Err(PyValueError::new_err(
                "len() only supported on dict and list views",
            ))
        }
    }

    /// Access child item by string key (for dict) or integer index (for list).
    pub fn __getitem__(&self, py: Python, key: Bound<'_, PyAny>) -> PyResult<PyObject> {
        let tag = self.get_tag()?;

        if tag == 0x70 {
            // Dict lookup
            let key_str = if let Ok(s) = key.downcast::<PyString>() {
                s.to_str()?
            } else {
                return Err(PyKeyError::new_err("Dictionary keys must be strings"));
            };

            // Fast-check: if key is not in string table, it cannot be in this dict
            let target_id = self.string_table.iter().position(|s| s == key_str);

            let target_id = match target_id {
                Some(id) => id as u32,
                None => return Err(PyKeyError::new_err(key_str.to_string())),
            };

            let mut cur = self.offset + 1;
            while cur < self.data.len() && self.data[cur] != 0x7F {
                if cur + 4 > self.data.len() {
                    return Err(PyValueError::new_err("Truncated dict key"));
                }
                let key_id = u32::from_le_bytes(self.data[cur..cur + 4].try_into().unwrap());
                cur += 4;

                if key_id == target_id {
                    return self.decode_value_at(py, cur);
                }

                skip_value(&self.data, &mut cur)
                    .map_err(|e| PyValueError::new_err(format!("Error traversing dict: {e}")))?;
            }

            Err(PyKeyError::new_err(key_str.to_string()))
        } else if tag == 0x60 {
            // List lookup
            let mut idx: isize = key.extract()?;
            let count = self.__len__()? as isize;

            if idx < 0 {
                idx += count;
            }
            if idx < 0 || idx >= count {
                return Err(PyIndexError::new_err("Index out of bounds"));
            }

            let mut cur = self.offset + 5;
            for _ in 0..idx {
                skip_value(&self.data, &mut cur)
                    .map_err(|e| PyValueError::new_err(format!("Error traversing list: {e}")))?;
            }

            self.decode_value_at(py, cur)
        } else {
            Err(PyValueError::new_err(
                "Subscript indexing only supported on dict and list views",
            ))
        }
    }

    /// Dictionary get method with optional default.
    #[pyo3(signature = (key, default = None))]
    pub fn get(&self, py: Python, key: &str, default: Option<PyObject>) -> PyResult<PyObject> {
        let key_obj = PyString::new(py, key);
        match self.__getitem__(py, key_obj.into_any()) {
            Ok(val) => Ok(val),
            Err(e) if e.is_instance_of::<PyKeyError>(py) => {
                Ok(default.unwrap_or_else(|| py.None()))
            }
            Err(e) => Err(e),
        }
    }

    /// Returns list of string keys if this view is a dictionary.
    pub fn keys(&self) -> PyResult<Vec<String>> {
        let tag = self.get_tag()?;
        if tag != 0x70 {
            return Err(PyValueError::new_err("keys() only supported on dict views"));
        }

        let mut keys = Vec::new();
        let mut cur = self.offset + 1;
        while cur < self.data.len() && self.data[cur] != 0x7F {
            if cur + 4 > self.data.len() {
                return Err(PyValueError::new_err("Truncated dict key"));
            }
            let key_id = u32::from_le_bytes(self.data[cur..cur + 4].try_into().unwrap()) as usize;
            cur += 4;

            if key_id < self.string_table.len() {
                keys.push(self.string_table[key_id].clone());
            }

            skip_value(&self.data, &mut cur)
                .map_err(|e| PyValueError::new_err(format!("Error traversing dict: {e}")))?;
        }

        Ok(keys)
    }

    /// Checks if a key exists in dictionary view.
    pub fn __contains__(&self, key: &str) -> PyResult<bool> {
        let tag = self.get_tag()?;
        if tag != 0x70 {
            return Ok(false);
        }

        let target_id = match self.string_table.iter().position(|s| s == key) {
            Some(id) => id as u32,
            None => return Ok(false),
        };

        let mut cur = self.offset + 1;
        while cur < self.data.len() && self.data[cur] != 0x7F {
            if cur + 4 > self.data.len() {
                return Ok(false);
            }
            let key_id = u32::from_le_bytes(self.data[cur..cur + 4].try_into().unwrap());
            cur += 4;

            if key_id == target_id {
                return Ok(true);
            }

            if skip_value(&self.data, &mut cur).is_err() {
                return Ok(false);
            }
        }

        Ok(false)
    }

    /// Materializes the full lazy view (or subtree) eagerly into native Python structures.
    pub fn materialize(&self, py: Python) -> PyResult<PyObject> {
        let tag = self.get_tag()?;
        if tag == 0x70 {
            let dict = PyDict::new(py);
            let mut cur = self.offset + 1;
            while cur < self.data.len() && self.data[cur] != 0x7F {
                let key_id =
                    u32::from_le_bytes(self.data[cur..cur + 4].try_into().unwrap()) as usize;
                cur += 4;
                let key_name = &self.string_table[key_id];

                let val = self.decode_value_at(py, cur)?;
                let py_val = if let Ok(child_view) = val.extract::<PyRef<BFastLazyView>>(py) {
                    child_view.materialize(py)?
                } else {
                    val
                };

                dict.set_item(key_name, py_val)?;
                skip_value(&self.data, &mut cur)
                    .map_err(|e| PyValueError::new_err(format!("Materialize error: {e}")))?;
            }
            Ok(dict.into_any().unbind())
        } else if tag == 0x60 {
            let count = self.__len__()?;
            let list = PyList::empty(py);
            let mut cur = self.offset + 5;
            for _ in 0..count {
                let val = self.decode_value_at(py, cur)?;
                let py_val = if let Ok(child_view) = val.extract::<PyRef<BFastLazyView>>(py) {
                    child_view.materialize(py)?
                } else {
                    val
                };
                list.append(py_val)?;
                skip_value(&self.data, &mut cur)
                    .map_err(|e| PyValueError::new_err(format!("Materialize error: {e}")))?;
            }
            Ok(list.into_any().unbind())
        } else {
            self.decode_value_at(py, self.offset)
        }
    }

    /// Alias for materialize() for dict views.
    pub fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        self.materialize(py)
    }

    /// Alias for materialize() for list views.
    pub fn to_list(&self, py: Python) -> PyResult<PyObject> {
        self.materialize(py)
    }

    fn __repr__(&self) -> PyResult<String> {
        let tag = self.get_tag()?;
        let kind = match tag {
            0x70 => "dict",
            0x60 => "list",
            _ => "value",
        };
        Ok(format!(
            "<BFastLazyView type='{kind}' offset={}>",
            self.offset
        ))
    }
}
