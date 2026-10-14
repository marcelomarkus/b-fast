#![allow(non_local_definitions)]

use ahash::{AHashMap, AHasher};
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyFrozenSet, PyList, PySet, PyString, PyTuple};
use rayon::prelude::*;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::ptr;

mod errors;
pub mod streaming;

// Performance tuning constants
const CACHE_LINE_SIZE: usize = 64;
const PARALLEL_COMPRESSION_THRESHOLD: usize = 1_000_000;
const INITIAL_BUFFER_SIZE: usize = 4096;
const MAX_RECURSION_DEPTH: usize = 128;

// Type tags with metadata preservation
const TAG_DATETIME: u8 = 0xD1;
const TAG_DATE: u8 = 0xD2;
const TAG_TIME: u8 = 0xD3;
const TAG_UUID: u8 = 0xD4;
const TAG_DECIMAL: u8 = 0xD5;

#[allow(non_local_definitions)]
#[pyclass]
pub struct BFast {
    string_table: AHashMap<String, u32>,
    string_list: Vec<String>,
    next_id: u32,
    work_buffer: Vec<u8>,
    payload_buffer: Vec<u8>,
    compressed_buffer: Vec<u8>,
    decompress_buffer: Vec<u8>,
    frame_buffer: Vec<u8>,
    key_cache: [Option<(u64, u32)>; 64],
    recursion_depth: usize,
    last_was_compressed: bool,
    py_none: PyObject,
    py_true: PyObject,
    py_false: PyObject,
    small_ints: [PyObject; 16],
}

impl Default for BFast {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(non_local_definitions)]
#[pymethods]
impl BFast {
    #[new]
    pub fn new() -> Self {
        Python::with_gil(|py| {
            let small_ints = [
                0i64.into_py(py),
                1i64.into_py(py),
                2i64.into_py(py),
                3i64.into_py(py),
                4i64.into_py(py),
                5i64.into_py(py),
                6i64.into_py(py),
                7i64.into_py(py),
                8i64.into_py(py),
                9i64.into_py(py),
                10i64.into_py(py),
                11i64.into_py(py),
                12i64.into_py(py),
                13i64.into_py(py),
                14i64.into_py(py),
                15i64.into_py(py),
            ];
            BFast {
                string_table: AHashMap::with_capacity(1024),
                string_list: Vec::with_capacity(1024),
                next_id: 0,
                work_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                payload_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                compressed_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                decompress_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                frame_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                key_cache: [None; 64],
                recursion_depth: 0,
                last_was_compressed: false,
                py_none: py.None(),
                py_true: true.into_py(py),
                py_false: false.into_py(py),
                small_ints,
            }
        })
    }

    pub fn encode_packed(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        self.encode_packed_internal(obj, compress)?;
        let slice = if self.last_was_compressed {
            &self.compressed_buffer
        } else {
            &self.work_buffer
        };
        Ok(PyBytes::new(obj.py(), slice).into())
    }

    #[pyo3(signature = (obj, *, compress = true))]
    pub fn encode_frame(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        let py = obj.py();
        self.encode_packed_internal(obj, compress)?;
        let is_compressed = if self.last_was_compressed { 1u8 } else { 0u8 };
        let slice_len = if self.last_was_compressed {
            self.compressed_buffer.len()
        } else {
            self.work_buffer.len()
        };
        let len = slice_len as u32;

        self.frame_buffer.clear();
        self.frame_buffer.reserve(6 + slice_len);
        self.frame_buffer.extend_from_slice(&len.to_le_bytes());
        self.frame_buffer.push(crate::streaming::FRAME_DATA);
        self.frame_buffer.push(is_compressed);
        if self.last_was_compressed {
            self.frame_buffer.extend_from_slice(&self.compressed_buffer);
        } else {
            self.frame_buffer.extend_from_slice(&self.work_buffer);
        }

        Ok(PyBytes::new(py, &self.frame_buffer).into())
    }

    #[staticmethod]
    pub fn get_stream_handshake(py: Python) -> PyResult<PyObject> {
        Ok(PyBytes::new(py, &[0x42, 0x53, crate::streaming::STREAM_VERSION, 0x00]).into())
    }

    #[staticmethod]
    pub fn get_eos_frame(py: Python) -> PyResult<PyObject> {
        Ok(PyBytes::new(
            py,
            &[0x00, 0x00, 0x00, 0x00, crate::streaming::FRAME_EOS, 0x00],
        )
        .into())
    }

    #[pyo3(signature = (bytes, *, decompress = true))]
    pub fn decode_packed(
        &mut self,
        py: Python,
        bytes: &[u8],
        decompress: bool,
    ) -> PyResult<PyObject> {
        let decompressed_data: &[u8] = if decompress {
            if bytes.len() < 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Buffer too small for B-FAST payload",
                ));
            }
            if &bytes[0..2] == b"BF" {
                bytes
            } else {
                if bytes.len() < 4 {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Buffer too small for compressed B-FAST data",
                    ));
                }
                let uncompressed_size =
                    u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
                self.decompress_buffer.clear();
                self.decompress_buffer.resize(uncompressed_size, 0);

                if lz4_flex::decompress_into(&bytes[4..], &mut self.decompress_buffer).is_ok() {
                    &self.decompress_buffer
                } else {
                    let decompressed = decompress_packed(bytes)
                        .map_err(pyo3::exceptions::PyValueError::new_err)?;
                    self.decompress_buffer = decompressed.into_owned();
                    &self.decompress_buffer
                }
            }
        } else {
            bytes
        };

        if decompressed_data.len() < 6 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Decompressed buffer too small for B-FAST header",
            ));
        }

        let magic = &decompressed_data[0..2];
        if magic != b"BF" {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Invalid B-FAST magic number",
            ));
        }

        let string_table_count =
            u16::from_le_bytes(decompressed_data[4..6].try_into().unwrap()) as usize;

        let max_possible = (decompressed_data.len() - 6) / 2;
        let mut string_table = Vec::with_capacity(string_table_count.min(max_possible));
        let mut offset = 6;
        for _ in 0..string_table_count {
            if offset >= decompressed_data.len() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Unexpected end of buffer in string table",
                ));
            }
            let length = decompressed_data[offset] as usize;
            offset += 1;
            if offset + length > decompressed_data.len() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "String extends beyond buffer in string table",
                ));
            }
            let string_bytes = &decompressed_data[offset..offset + length];
            let string_val = std::str::from_utf8(string_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in string table: {}",
                    e
                ))
            })?;
            string_table.push(PyString::intern(py, string_val));
            offset += length;
        }

        let mut parser = BFastParser {
            py,
            data: decompressed_data,
            offset,
            string_table: &string_table,
            datetime_fromisoformat: None,
            date_fromisoformat: None,
            time_fromisoformat: None,
            uuid_class: None,
            decimal_class: None,
            py_none: &self.py_none,
            py_true: &self.py_true,
            py_false: &self.py_false,
            small_ints: &self.small_ints,
            recursion_depth: 0,
        };

        parser.parse()
    }
}

impl BFast {
    fn encode_packed_internal(&mut self, obj: &PyAny, compress: bool) -> PyResult<()> {
        self.work_buffer.clear();
        self.payload_buffer.clear();
        self.recursion_depth = 0;

        // Serialize payload into payload_buffer
        std::mem::swap(&mut self.work_buffer, &mut self.payload_buffer);

        let serialize_res = if let Ok(list) = obj.downcast::<PyList>() {
            if list.len() > 8 && self.serialize_pydantic_simd_batch(list).is_ok() {
                Ok(())
            } else {
                self.work_buffer.clear();
                self.serialize_any_optimized(obj)
            }
        } else {
            self.serialize_any_optimized(obj)
        };

        std::mem::swap(&mut self.work_buffer, &mut self.payload_buffer);
        serialize_res?;

        // Write header + string table to work_buffer, followed by payload
        self.work_buffer.extend_from_slice(&[0u8; 6]);
        self.write_string_table_vectorized()?;
        self.write_header_simd(0, compress);
        self.work_buffer.extend_from_slice(&self.payload_buffer);

        self.finalize_compression(compress)
    }

    #[inline(always)]
    fn finalize_compression(&mut self, compress: bool) -> PyResult<()> {
        if compress && self.work_buffer.len() > 256 {
            if self.work_buffer.len() >= PARALLEL_COMPRESSION_THRESHOLD {
                self.compressed_buffer = self.compress_parallel();
            } else {
                self.compressed_buffer = compress_prepend_size(&self.work_buffer);
            }
            self.last_was_compressed = true;
        } else {
            self.last_was_compressed = false;
        }
        Ok(())
    }

    fn compress_parallel(&self) -> Vec<u8> {
        const CHUNK_SIZE: usize = 256 * 1024;

        let data = &self.work_buffer;
        let total_size = data.len();

        if total_size < CHUNK_SIZE * 2 {
            return compress_prepend_size(data);
        }

        let chunks: Vec<Vec<u8>> = data
            .par_chunks(CHUNK_SIZE)
            .map(compress_prepend_size)
            .collect();

        let mut result = Vec::with_capacity(total_size / 2);
        result.extend_from_slice(&(total_size as u32).to_le_bytes());
        result.extend_from_slice(&(chunks.len() as u32).to_le_bytes());

        for chunk in &chunks {
            result.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            result.extend_from_slice(chunk);
        }

        result
    }

    #[inline(always)]
    fn ensure_buffer_capacity(&mut self, additional: usize) {
        let required = self.work_buffer.len() + additional;
        if required > self.work_buffer.capacity() {
            let new_cap = (self.work_buffer.capacity() * 2).max(required);
            self.work_buffer.reserve(new_cap - self.work_buffer.len());
        }
    }

    #[inline(always)]
    fn check_recursion_depth(&mut self) -> PyResult<()> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
                "Maximum recursion depth exceeded",
            ));
        }
        Ok(())
    }

    #[inline(always)]
    fn decrease_recursion_depth(&mut self) {
        self.recursion_depth -= 1;
    }

    #[inline(always)]
    fn serialize_pydantic_simd_batch(&mut self, list: &PyList) -> PyResult<()> {
        let len = list.len();
        if len == 0 {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }

        self.check_recursion_depth()?;

        let first_item = list.get_item(0)?;
        if !first_item.hasattr("__dict__")? {
            self.decrease_recursion_depth();
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Not Pydantic",
            ));
        }

        let dict = first_item.getattr("__dict__")?.downcast::<PyDict>()?;
        let field_keys: Vec<&PyAny> = dict.keys().iter().collect();

        let field_ids: Vec<u32> = field_keys
            .iter()
            .map(|k| {
                let name = if let Ok(s) = k.downcast::<PyString>() {
                    s.to_str().unwrap_or("")
                } else {
                    ""
                };
                self.get_or_create_string_id_fast(name)
            })
            .collect();

        // Auto-detect: check if first object has complex types
        let use_fast_mode = self.detect_simple_types(dict, &field_keys)?;

        self.ensure_buffer_capacity(5 + len * (field_ids.len() * 16 + 4));

        // Write list opcode and length
        self.work_buffer.push(0x60);
        self.work_buffer
            .extend_from_slice(&(len as u32).to_le_bytes());

        // Serialize all items directly into work_buffer (payload_buffer)
        if use_fast_mode {
            for item in list.iter() {
                self.serialize_pydantic_fast(item, &field_keys, &field_ids)?;
            }
        } else {
            for item in list.iter() {
                self.serialize_pydantic_complex(item, &field_keys, &field_ids)?;
            }
        }

        self.decrease_recursion_depth();
        Ok(())
    }

    #[inline(always)]
    fn detect_simple_types(&self, dict: &PyDict, field_keys: &[&PyAny]) -> PyResult<bool> {
        // Check first object's field types
        for key in field_keys {
            if let Some(value) = dict.get_item(key)? {
                if value.is_none() {
                    continue;
                }

                // Check for complex types
                if let Ok("datetime" | "date" | "time" | "UUID" | "Decimal") =
                    value.get_type().name()
                {
                    return Ok(false); // Use complex mode
                }
            }
        }
        Ok(true) // Use fast mode
    }

    #[inline(always)]
    fn serialize_pydantic_fast(
        &mut self,
        obj: &PyAny,
        field_keys: &[&PyAny],
        field_ids: &[u32],
    ) -> PyResult<()> {
        self.work_buffer.push(0x70);

        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        let dict_ptr = dict.as_ptr();

        for (i, key) in field_keys.iter().enumerate() {
            self.work_buffer
                .extend_from_slice(&field_ids[i].to_le_bytes());

            let item_ptr = unsafe { pyo3::ffi::PyDict_GetItemWithError(dict_ptr, key.as_ptr()) };
            if !item_ptr.is_null() {
                let value = unsafe { obj.py().from_borrowed_ptr::<PyAny>(item_ptr) };
                self.serialize_value_fast(value)?;
            } else {
                self.work_buffer.push(0x10);
            }
        }

        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value_fast(&mut self, val: &PyAny) -> PyResult<()> {
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        if let Ok(b) = val.downcast::<pyo3::types::PyBool>() {
            self.work_buffer.push(if b.is_true() { 0x21 } else { 0x20 });
            return Ok(());
        }

        if let Ok(l) = val.downcast::<pyo3::types::PyLong>() {
            let n: i64 = l.extract()?;
            if (0..=7).contains(&n) {
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                self.work_buffer.extend_from_slice(&n.to_le_bytes());
            }
            return Ok(());
        }

        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.ensure_buffer_capacity(4 + bytes.len());
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        if let Ok(f) = val.downcast::<pyo3::types::PyFloat>() {
            let val = f.value();
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&val.to_le_bytes());
            return Ok(());
        }

        // Fast path for list in serialize_value_fast
        if let Ok(list) = val.downcast::<PyList>() {
            let len = list.len();
            self.work_buffer.push(0x60);
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            if len > 0 {
                let list_ptr = list.as_ptr();
                let first_item = unsafe { pyo3::ffi::PyList_GetItem(list_ptr, 0) };
                if !first_item.is_null()
                    && unsafe { pyo3::ffi::PyFloat_CheckExact(first_item) } != 0
                {
                    self.ensure_buffer_capacity(len * 9);
                    for i in 0..len {
                        let item_ptr = unsafe {
                            pyo3::ffi::PyList_GetItem(list_ptr, i as pyo3::ffi::Py_ssize_t)
                        };
                        let fval = unsafe { pyo3::ffi::PyFloat_AsDouble(item_ptr) };
                        self.work_buffer.push(0x40);
                        self.work_buffer.extend_from_slice(&fval.to_le_bytes());
                    }
                    return Ok(());
                }
            }

            for item in list.iter() {
                self.serialize_value_fast(item)?;
            }
            return Ok(());
        }

        // Fallback: handle dicts and complex types properly
        self.serialize_any_optimized(val)
    }

    #[inline(always)]
    fn serialize_pydantic_complex(
        &mut self,
        obj: &PyAny,
        field_keys: &[&PyAny],
        field_ids: &[u32],
    ) -> PyResult<()> {
        // Complex path: handles all types including datetime, UUID, Decimal
        self.work_buffer.push(0x70);

        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;

        for (i, key) in field_keys.iter().enumerate() {
            self.work_buffer
                .extend_from_slice(&field_ids[i].to_le_bytes());

            if let Some(value) = dict.get_item(key)? {
                self.serialize_value_ultra_fast(value)?;
            } else {
                self.work_buffer.push(0x10);
            }
        }

        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value_ultra_fast(&mut self, val: &PyAny) -> PyResult<()> {
        // Fast type checking using direct downcasts
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        // Bool check (must come before int)
        if let Ok(b) = val.downcast::<pyo3::types::PyBool>() {
            self.work_buffer.push(if b.is_true() { 0x21 } else { 0x20 });
            return Ok(());
        }

        // Int check
        if let Ok(l) = val.downcast::<pyo3::types::PyLong>() {
            let n: i64 = l.extract()?;
            if (0..=7).contains(&n) {
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                self.work_buffer.extend_from_slice(&n.to_le_bytes());
            }
            return Ok(());
        }

        // String check
        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.ensure_buffer_capacity(4 + bytes.len());
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        // Float check
        if let Ok(f) = val.downcast::<pyo3::types::PyFloat>() {
            let val = f.value();
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&val.to_le_bytes());
            return Ok(());
        }

        // Special types (Decimal, UUID, datetime, etc.)
        if let Ok(type_name) = val.get_type().name() {
            match type_name {
                "Decimal" => {
                    let dec_str = val.str()?;
                    let bytes = dec_str.to_str()?.as_bytes();
                    self.work_buffer.push(TAG_DECIMAL);
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                "UUID" => {
                    let hex_obj = val.getattr("hex")?;
                    let bytes = hex_obj.extract::<&str>()?.as_bytes();
                    self.work_buffer.push(TAG_UUID);
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                "datetime" | "date" | "time" => {
                    let iso_obj = val.call_method0("isoformat")?;
                    let bytes = iso_obj.extract::<&str>()?.as_bytes();
                    let tag = match type_name {
                        "datetime" => TAG_DATETIME,
                        "date" => TAG_DATE,
                        "time" => TAG_TIME,
                        _ => 0x50,
                    };
                    self.work_buffer.push(tag);
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                _ => {}
            }
        }

        // Enum (extract .value) - fast attribute check
        if let Ok(enum_val) = val.getattr("value") {
            if val.hasattr("name")? {
                return self.serialize_value_ultra_fast(enum_val);
            }
        }

        self.serialize_any_optimized(val)
    }

    #[inline(always)]
    fn get_or_create_string_id_fast(&mut self, key_str: &str) -> u32 {
        let mut hasher = AHasher::default();
        key_str.hash(&mut hasher);
        let hash = hasher.finish();
        let slot = (hash as usize) & 63;

        if let Some((cached_hash, id)) = self.key_cache[slot] {
            if cached_hash == hash {
                return id;
            }
        }

        if let Some(&existing_id) = self.string_table.get(key_str) {
            self.key_cache[slot] = Some((hash, existing_id));
            return existing_id;
        }

        let new_id = self.next_id;
        self.string_table.insert(key_str.to_owned(), new_id);
        self.string_list.push(key_str.to_owned());
        self.next_id += 1;

        self.key_cache[slot] = Some((hash, new_id));
        new_id
    }

    #[inline(always)]
    fn write_header_simd(&mut self, pos: usize, compress: bool) {
        unsafe {
            let header = self.work_buffer.as_mut_ptr().add(pos);
            ptr::write_unaligned(header as *mut u16, u16::from_le_bytes(*b"BF"));
            *header.add(2) = if compress { 0x01 } else { 0x00 };
            *header.add(3) = 0x01;
            let count = self.string_list.len() as u16;
            ptr::write_unaligned(header.add(4) as *mut u16, count.to_le());
        }
    }

    #[inline(always)]
    fn write_string_table_vectorized(&mut self) -> PyResult<()> {
        if self.string_list.is_empty() {
            return Ok(());
        }

        let total_size: usize = self.string_list.iter().map(|s| s.len() + 1).sum();
        let aligned_size = (total_size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
        self.work_buffer.reserve(aligned_size);

        for string in &self.string_list {
            let bytes = string.as_bytes();
            self.work_buffer.push(bytes.len() as u8);
            self.work_buffer.extend_from_slice(bytes);
        }
        Ok(())
    }

    #[inline(always)]
    fn serialize_any_optimized(&mut self, val: &PyAny) -> PyResult<()> {
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        // 1. PyBool MUST come before PyLong because bool is a subclass of int in Python
        if let Ok(b) = val.downcast::<pyo3::types::PyBool>() {
            self.work_buffer.push(if b.is_true() { 0x21 } else { 0x20 });
            return Ok(());
        }

        // 2. PyLong (int)
        if let Ok(l) = val.downcast::<pyo3::types::PyLong>() {
            if let Ok(n) = l.extract::<i64>() {
                if (0..=7).contains(&n) {
                    self.work_buffer.push(0x30 | (n as u8));
                } else {
                    self.work_buffer.push(0x38);
                    self.work_buffer.extend_from_slice(&n.to_le_bytes());
                }
                return Ok(());
            }
        }

        // 3. PyFloat (float)
        if let Ok(f) = val.downcast::<pyo3::types::PyFloat>() {
            let val = f.value();
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&val.to_le_bytes());
            return Ok(());
        }

        // 4. PyString (str)
        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.ensure_buffer_capacity(4 + bytes.len());
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        // 5. PyDict (dict)
        if let Ok(dict) = val.downcast::<PyDict>() {
            self.work_buffer.push(0x70);

            for (k, v) in dict.iter() {
                let key_str = if let Ok(py_str) = k.downcast::<PyString>() {
                    py_str.to_str()?
                } else {
                    &k.to_string()
                };

                let id = self.get_or_create_string_id_fast(key_str);
                self.work_buffer.extend_from_slice(&id.to_le_bytes());
                self.serialize_any_optimized(v)?;
            }

            self.work_buffer.push(0x7F);
            return Ok(());
        }

        // 6. PyList (list)
        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            let list_ptr = list.as_ptr();
            let py = val.py();
            for i in 0..len {
                let item_ptr =
                    unsafe { pyo3::ffi::PyList_GetItem(list_ptr, i as pyo3::ffi::Py_ssize_t) };
                let item = unsafe { py.from_borrowed_ptr::<PyAny>(item_ptr) };
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        // 7. PyTuple (serialize as list)
        if let Ok(tuple) = val.downcast::<PyTuple>() {
            self.work_buffer.push(0x60);
            let len = tuple.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            let tuple_ptr = tuple.as_ptr();
            let py = val.py();
            for i in 0..len {
                let item_ptr =
                    unsafe { pyo3::ffi::PyTuple_GetItem(tuple_ptr, i as pyo3::ffi::Py_ssize_t) };
                let item = unsafe { py.from_borrowed_ptr::<PyAny>(item_ptr) };
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        // 8. PySet / PyFrozenSet (serialize as list)
        if let Ok(set) = val.downcast::<PySet>() {
            self.work_buffer.push(0x60);
            let len = set.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            for item in set.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        if let Ok(frozenset) = val.downcast::<PyFrozenSet>() {
            self.work_buffer.push(0x60);
            let len = frozenset.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            for item in frozenset.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        // 9. PyBytes / bytearray
        if let Ok(py_bytes) = val.extract::<&[u8]>() {
            self.work_buffer.push(0x80);
            self.work_buffer
                .extend_from_slice(&(py_bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(py_bytes);
            return Ok(());
        }

        // 10. NumPy array
        if let Ok(array) = val.extract::<PyReadonlyArrayDyn<f64>>() {
            self.work_buffer.push(0x90);
            let raw_data = array.as_slice()?;
            self.ensure_buffer_capacity(4 + raw_data.len() * 8);
            self.work_buffer
                .extend_from_slice(&(raw_data.len() as u32).to_le_bytes());

            let byte_slice = unsafe {
                std::slice::from_raw_parts(raw_data.as_ptr() as *const u8, raw_data.len() * 8)
            };
            self.work_buffer.extend_from_slice(byte_slice);
            return Ok(());
        }

        // 11. Decimal
        if let Ok(type_name) = val.get_type().name() {
            if type_name == "Decimal" {
                let dec_str = val.str()?;
                let bytes = dec_str.to_str()?.as_bytes();
                self.work_buffer.push(TAG_DECIMAL);
                self.work_buffer
                    .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                self.work_buffer.extend_from_slice(bytes);
                return Ok(());
            }
        }

        // 12. datetime, date, time (ISO 8601) with type preservation
        if val.hasattr("isoformat")? {
            let iso_obj = val.call_method0("isoformat")?;
            let iso_str = iso_obj.extract::<&str>()?;
            let type_name = val.get_type().name()?;

            let tag = match type_name {
                "datetime" => TAG_DATETIME,
                "date" => TAG_DATE,
                "time" => TAG_TIME,
                _ => 0x50,
            };

            self.work_buffer.push(tag);
            let bytes = iso_str.as_bytes();
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        // 13. UUID
        if val.hasattr("hex")? {
            if let Ok(type_name) = val.get_type().name() {
                if type_name == "UUID" {
                    let hex_obj = val.getattr("hex")?;
                    let hex_str = hex_obj.extract::<&str>()?;
                    self.work_buffer.push(TAG_UUID);
                    let bytes = hex_str.as_bytes();
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
            }
        }

        // 14. Enum (extract value) - check BEFORE __dict__
        if let Ok(enum_val) = val.getattr("value") {
            if val.hasattr("name")? {
                return self.serialize_any_optimized(enum_val);
            }
        }

        // 15. DataFrame & Series support (Polars, Pandas, PyArrow) - check BEFORE __dict__
        if let Ok(type_name) = val.get_type().name() {
            if type_name == "DataFrame" {
                if val.hasattr("to_dicts")? {
                    // Polars DataFrame
                    let records = val.call_method0("to_dicts")?;
                    return self.serialize_any_optimized(records);
                } else if val.hasattr("to_dict")? {
                    // Pandas DataFrame
                    let records = val.call_method1("to_dict", ("records",))?;
                    return self.serialize_any_optimized(records);
                }
            } else if type_name == "LazyFrame" {
                if val.hasattr("collect")? {
                    let df = val.call_method0("collect")?;
                    if df.hasattr("to_dicts")? {
                        let records = df.call_method0("to_dicts")?;
                        return self.serialize_any_optimized(records);
                    }
                }
            } else if (type_name == "Series" || type_name == "Index") && val.hasattr("to_list")? {
                let list_vals = val.call_method0("to_list")?;
                return self.serialize_any_optimized(list_vals);
            } else if (type_name == "Table" || type_name == "RecordBatch")
                && val.hasattr("to_pylist")?
            {
                let records = val.call_method0("to_pylist")?;
                return self.serialize_any_optimized(records);
            }
        }

        // 16. Try __dict__ for Pydantic models
        if let Ok(dict_attr) = val.getattr("__dict__") {
            if let Ok(dict) = dict_attr.downcast::<PyDict>() {
                self.work_buffer.push(0x70);

                for (k, v) in dict.iter() {
                    let key_str = if let Ok(py_str) = k.downcast::<PyString>() {
                        py_str.to_str()?
                    } else {
                        &k.to_string()
                    };

                    let id = self.get_or_create_string_id_fast(key_str);
                    self.work_buffer.extend_from_slice(&id.to_le_bytes());
                    self.serialize_any_optimized(v)?;
                }

                self.work_buffer.push(0x7F);
                return Ok(());
            }
        }

        // Fallback: convert to string
        let str_repr = val.str()?.extract::<String>()?;
        self.work_buffer.push(0x50);
        let bytes = str_repr.as_bytes();
        self.work_buffer
            .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.work_buffer.extend_from_slice(bytes);
        Ok(())
    }
}

#[pymodule]
fn _b_fast(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<BFast>()?;
    m.add_class::<streaming::BFastStreamDecoder>()?;
    m.add_class::<streaming::BFastStreamEncoder>()?;
    m.add(
        "BFastError",
        _py.get_type::<pyo3::exceptions::PyValueError>(),
    )?;
    Ok(())
}

fn decompress_packed(data: &[u8]) -> Result<Cow<'_, [u8]>, String> {
    if data.len() < 2 {
        return Err("Buffer too small for B-FAST payload".to_string());
    }
    if &data[0..2] == b"BF" {
        return Ok(Cow::Borrowed(data));
    }
    if data.len() < 8 {
        return Err("Buffer too small for compressed B-FAST data".to_string());
    }

    // Try single-chunk decompression first
    if let Ok(decompressed) = lz4_flex::decompress_size_prepended(data) {
        return Ok(Cow::Owned(decompressed));
    }

    // Fall back to parallel chunk decompression
    let uncompressed_size = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let chunks_count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;

    let max_possible_chunks = (data.len() - 8) / 4;
    if chunks_count > max_possible_chunks {
        return Err("Invalid chunks count in parallel compression header".to_string());
    }

    let mut offset = 8;
    let mut chunk_slices = Vec::with_capacity(chunks_count);

    for _ in 0..chunks_count {
        if offset + 4 > data.len() {
            return Err("Unexpected end of data in parallel compression chunk headers".to_string());
        }
        let chunk_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + chunk_len > data.len() {
            return Err("Unexpected end of data in parallel compression chunk data".to_string());
        }
        chunk_slices.push(&data[offset..offset + chunk_len]);
        offset += chunk_len;
    }

    let decompressed_chunks: Result<Vec<Vec<u8>>, _> = chunk_slices
        .into_par_iter()
        .map(lz4_flex::decompress_size_prepended)
        .collect();

    let decompressed_chunks =
        decompressed_chunks.map_err(|e| format!("LZ4 chunk decompression failed: {}", e))?;
    let result = decompressed_chunks.concat();
    if result.len() != uncompressed_size {
        return Err(format!(
            "Decompressed size mismatch: expected {}, got {}",
            uncompressed_size,
            result.len()
        ));
    }
    Ok(Cow::Owned(result))
}

struct BFastParser<'a, 'py> {
    py: Python<'py>,
    data: &'a [u8],
    offset: usize,
    string_table: &'a [&'py PyString],
    datetime_fromisoformat: Option<&'py PyAny>,
    date_fromisoformat: Option<&'py PyAny>,
    time_fromisoformat: Option<&'py PyAny>,
    uuid_class: Option<&'py PyAny>,
    decimal_class: Option<&'py PyAny>,
    py_none: &'a PyObject,
    py_true: &'a PyObject,
    py_false: &'a PyObject,
    small_ints: &'a [PyObject; 16],
    recursion_depth: usize,
}

impl<'a, 'py> BFastParser<'a, 'py> {
    fn get_datetime_fromisoformat(&mut self) -> PyResult<&'py PyAny> {
        if let Some(func) = self.datetime_fromisoformat {
            Ok(func)
        } else {
            let func = self
                .py
                .import("datetime")?
                .getattr("datetime")?
                .getattr("fromisoformat")?;
            self.datetime_fromisoformat = Some(func);
            Ok(func)
        }
    }

    fn get_date_fromisoformat(&mut self) -> PyResult<&'py PyAny> {
        if let Some(func) = self.date_fromisoformat {
            Ok(func)
        } else {
            let func = self
                .py
                .import("datetime")?
                .getattr("date")?
                .getattr("fromisoformat")?;
            self.date_fromisoformat = Some(func);
            Ok(func)
        }
    }

    fn get_time_fromisoformat(&mut self) -> PyResult<&'py PyAny> {
        if let Some(func) = self.time_fromisoformat {
            Ok(func)
        } else {
            let func = self
                .py
                .import("datetime")?
                .getattr("time")?
                .getattr("fromisoformat")?;
            self.time_fromisoformat = Some(func);
            Ok(func)
        }
    }

    fn get_uuid_class(&mut self) -> PyResult<&'py PyAny> {
        if let Some(cls) = self.uuid_class {
            Ok(cls)
        } else {
            let cls = self.py.import("uuid")?.getattr("UUID")?;
            self.uuid_class = Some(cls);
            Ok(cls)
        }
    }

    fn get_decimal_class(&mut self) -> PyResult<&'py PyAny> {
        if let Some(cls) = self.decimal_class {
            Ok(cls)
        } else {
            let cls = self.py.import("decimal")?.getattr("Decimal")?;
            self.decimal_class = Some(cls);
            Ok(cls)
        }
    }

    fn check_bounds(&self, size: usize) -> PyResult<()> {
        if self.offset + size > self.data.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Unexpected end of buffer during parsing",
            ));
        }
        Ok(())
    }

    fn parse(&mut self) -> PyResult<PyObject> {
        self.recursion_depth += 1;
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            return Err(PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
                "Maximum recursion depth exceeded during B-FAST decoding",
            ));
        }

        self.check_bounds(1)?;
        let tag = self.data[self.offset];
        self.offset += 1;

        let result = self.parse_tag(tag);

        self.recursion_depth -= 1;
        result
    }

    fn parse_tag(&mut self, tag: u8) -> PyResult<PyObject> {
        // Null
        if tag == 0x10 {
            return Ok(self.py_none.clone_ref(self.py));
        }

        // Booleans
        if tag == 0x20 {
            return Ok(self.py_false.clone_ref(self.py));
        }
        if tag == 0x21 {
            return Ok(self.py_true.clone_ref(self.py));
        }

        // Int64
        if tag == 0x38 {
            self.check_bounds(8)?;
            let val =
                i64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
            self.offset += 8;
            if (0..=15).contains(&val) {
                return Ok(self.small_ints[val as usize].clone_ref(self.py));
            }
            let ptr = unsafe { pyo3::ffi::PyLong_FromLongLong(val) };
            if ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, ptr) });
        }

        // Small integers (bit-packed)
        if (tag & 0xF0) == 0x30 {
            let val = (tag & 0x0F) as usize;
            return Ok(self.small_ints[val].clone_ref(self.py));
        }

        // Float64
        if tag == 0x40 {
            self.check_bounds(8)?;
            let val =
                f64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
            self.offset += 8;
            let ptr = unsafe { pyo3::ffi::PyFloat_FromDouble(val) };
            if ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, ptr) });
        }

        // Raw string: direct single-pass C-API construction
        if tag == 0x50 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let ptr = unsafe {
                pyo3::ffi::PyUnicode_FromStringAndSize(
                    str_bytes.as_ptr() as *const std::os::raw::c_char,
                    length as pyo3::ffi::Py_ssize_t,
                )
            };
            if ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, ptr) });
        }

        // List/Array: allocate PyList directly and steal items
        if tag == 0x60 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;

            let py_list_ptr = unsafe { pyo3::ffi::PyList_New(length as pyo3::ffi::Py_ssize_t) };
            if py_list_ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            for i in 0..length {
                match self.parse() {
                    Ok(item) => unsafe {
                        pyo3::ffi::PyList_SetItem(
                            py_list_ptr,
                            i as pyo3::ffi::Py_ssize_t,
                            item.into_ptr(),
                        );
                    },
                    Err(e) => {
                        unsafe {
                            pyo3::ffi::Py_DECREF(py_list_ptr);
                        }
                        return Err(e);
                    }
                }
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, py_list_ptr) });
        }

        // Object start: direct C-API dict assignment with interned keys
        if tag == 0x70 {
            let dict_ptr = unsafe { pyo3::ffi::PyDict_New() };
            if dict_ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            while self.offset < self.data.len() && self.data[self.offset] != 0x7F {
                self.check_bounds(4)?;
                let key_id = unsafe {
                    let bytes = std::ptr::read_unaligned(
                        self.data.as_ptr().add(self.offset) as *const [u8; 4]
                    );
                    u32::from_le_bytes(bytes) as usize
                };
                self.offset += 4;

                if key_id >= self.string_table.len() {
                    unsafe { pyo3::ffi::Py_DECREF(dict_ptr) };
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid string table index: {}",
                        key_id
                    )));
                }

                let key = self.string_table[key_id];
                match self.parse() {
                    Ok(value) => {
                        let ret = unsafe {
                            pyo3::ffi::PyDict_SetItem(dict_ptr, key.as_ptr(), value.as_ptr())
                        };
                        if ret != 0 {
                            unsafe { pyo3::ffi::Py_DECREF(dict_ptr) };
                            return Err(PyErr::fetch(self.py));
                        }
                    }
                    Err(e) => {
                        unsafe { pyo3::ffi::Py_DECREF(dict_ptr) };
                        return Err(e);
                    }
                }
            }

            if self.offset >= self.data.len() {
                unsafe { pyo3::ffi::Py_DECREF(dict_ptr) };
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Object not properly terminated",
                ));
            }

            self.offset += 1; // Skip 0x7F
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, dict_ptr) });
        }

        // Bytes: direct C-API bytes construction
        if tag == 0x80 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let bytes_val = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let ptr = unsafe {
                pyo3::ffi::PyBytes_FromStringAndSize(
                    bytes_val.as_ptr() as *const std::os::raw::c_char,
                    length as pyo3::ffi::Py_ssize_t,
                )
            };
            if ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, ptr) });
        }

        // NumPy Array (f64): decode directly into PyList of PyFloat without intermediate Vec
        if tag == 0x90 {
            self.check_bounds(4)?;
            let length = unsafe {
                let bytes =
                    std::ptr::read_unaligned(self.data.as_ptr().add(self.offset) as *const [u8; 4]);
                u32::from_le_bytes(bytes) as usize
            };
            self.offset += 4;
            self.check_bounds(length * 8)?;

            let py_list_ptr = unsafe { pyo3::ffi::PyList_New(length as pyo3::ffi::Py_ssize_t) };
            if py_list_ptr.is_null() {
                return Err(PyErr::fetch(self.py));
            }
            let data_slice = &self.data[self.offset..self.offset + length * 8];
            self.offset += length * 8;
            let mut f_ptr = data_slice.as_ptr();

            for i in 0..length {
                let val = unsafe {
                    let val_bytes = std::ptr::read_unaligned(f_ptr as *const [u8; 8]);
                    f_ptr = f_ptr.add(8);
                    f64::from_le_bytes(val_bytes)
                };
                let float_ptr = unsafe { pyo3::ffi::PyFloat_FromDouble(val) };
                if float_ptr.is_null() {
                    unsafe { pyo3::ffi::Py_DECREF(py_list_ptr) };
                    return Err(PyErr::fetch(self.py));
                }
                unsafe {
                    pyo3::ffi::PyList_SetItem(py_list_ptr, i as pyo3::ffi::Py_ssize_t, float_ptr);
                }
            }
            return Ok(unsafe { PyObject::from_owned_ptr(self.py, py_list_ptr) });
        }

        // DateTime (0xD1) - ISO 8601 string via cached callable
        if tag == TAG_DATETIME {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let iso_str = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in datetime string: {}",
                    e
                ))
            })?;
            let obj = self.get_datetime_fromisoformat()?.call1((iso_str,))?;
            return Ok(obj.into());
        }

        // Date (0xD2) - ISO 8601 date string via cached callable
        if tag == TAG_DATE {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let iso_str = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in date string: {}",
                    e
                ))
            })?;
            let obj = self.get_date_fromisoformat()?.call1((iso_str,))?;
            return Ok(obj.into());
        }

        // Time (0xD3) - ISO 8601 time string via cached callable
        if tag == TAG_TIME {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let iso_str = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in time string: {}",
                    e
                ))
            })?;
            let obj = self.get_time_fromisoformat()?.call1((iso_str,))?;
            return Ok(obj.into());
        }

        // UUID (0xD4) via cached class
        if tag == TAG_UUID {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let hex_str = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in UUID string: {}",
                    e
                ))
            })?;
            let obj = self.get_uuid_class()?.call1((hex_str,))?;
            return Ok(obj.into());
        }

        // Decimal (0xD5) via cached class
        if tag == TAG_DECIMAL {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let dec_str = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in Decimal string: {}",
                    e
                ))
            })?;
            let obj = self.get_decimal_class()?.call1((dec_str,))?;
            return Ok(obj.into());
        }

        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown tag: 0x{:02x}",
            tag
        )))
    }
}
