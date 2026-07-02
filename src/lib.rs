#![allow(non_local_definitions)]

use ahash::{AHashMap, AHasher};
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyFrozenSet, PyList, PySet, PyString, PyTuple};
use rayon::prelude::*;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::mem;
use std::ptr;

mod errors;

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
    next_id: u32,
    work_buffer: Vec<u8>,
    key_cache: [Option<(u32, u32)>; 64],
    cache_index: usize,
    recursion_depth: usize,
}

#[allow(non_local_definitions)]
#[pymethods]
impl BFast {
    #[new]
    fn new() -> Self {
        BFast {
            string_table: AHashMap::with_capacity(1024),
            next_id: 0,
            work_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
            key_cache: [None; 64],
            cache_index: 0,
            recursion_depth: 0,
        }
    }

    pub fn encode_packed(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        self.work_buffer.clear();
        self.recursion_depth = 0;

        // CACHE-ALIGNED pre-allocation
        let estimated_size = if let Ok(list) = obj.downcast::<PyList>() {
            let len = list.len();
            ((len * 48 + 4096) + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)
        } else {
            8192
        };

        if self.work_buffer.capacity() < estimated_size {
            self.work_buffer.reserve(estimated_size);
        }

        // Reserve space for header
        let header_pos = self.work_buffer.len();
        self.work_buffer.extend_from_slice(&[0u8; 6]);

        // Write string table placeholder (will be filled later)
        let string_table_pos = self.work_buffer.len();

        // SIMD batch processing for lists
        if let Ok(list) = obj.downcast::<PyList>() {
            if list.len() > 8 {
                if let Ok(()) = self.serialize_pydantic_simd_batch(list) {
                    // Insert string table after header, before payload
                    let payload = self.work_buffer.split_off(string_table_pos);
                    self.write_string_table_vectorized()?;
                    self.work_buffer.extend_from_slice(&payload);
                    self.write_header_simd(header_pos, compress);

                    let final_data = if compress && self.work_buffer.len() > 256 {
                        if self.work_buffer.len() >= PARALLEL_COMPRESSION_THRESHOLD {
                            self.compress_parallel()
                        } else {
                            compress_prepend_size(&self.work_buffer)
                        }
                    } else {
                        mem::take(&mut self.work_buffer)
                    };

                    return Ok(PyBytes::new(obj.py(), &final_data).into());
                }
            }
        }

        self.serialize_any_optimized(obj)?;

        // Insert string table after header, before payload
        let payload = self.work_buffer.split_off(string_table_pos);
        self.write_string_table_vectorized()?;
        self.work_buffer.extend_from_slice(&payload);
        self.write_header_simd(header_pos, compress);

        let final_data = if compress && self.work_buffer.len() > 256 {
            if self.work_buffer.len() >= PARALLEL_COMPRESSION_THRESHOLD {
                self.compress_parallel()
            } else {
                compress_prepend_size(&self.work_buffer)
            }
        } else {
            mem::take(&mut self.work_buffer)
        };

        Ok(PyBytes::new(obj.py(), &final_data).into())
    }

    #[pyo3(signature = (bytes, *, decompress = true))]
    pub fn decode_packed(&self, py: Python, bytes: &[u8], decompress: bool) -> PyResult<PyObject> {
        let decompressed_data = if decompress {
            decompress_packed(bytes)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?
        } else {
            Cow::Borrowed(bytes)
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

        let mut offset = 6;
        let mut string_table = Vec::with_capacity(string_table_count);
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
            let string_val = std::str::from_utf8(string_bytes)
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid UTF-8 in string table: {}",
                        e
                    ))
                })?
                .to_string();
            string_table.push(string_val);
            offset += length;
        }

        let datetime_module = py.import("datetime")?;
        let datetime_class = datetime_module.getattr("datetime")?;
        let date_class = datetime_module.getattr("date")?;
        let time_class = datetime_module.getattr("time")?;

        let uuid_module = py.import("uuid")?;
        let uuid_class = uuid_module.getattr("UUID")?;

        let decimal_module = py.import("decimal")?;
        let decimal_class = decimal_module.getattr("Decimal")?;

        let mut parser = BFastParser {
            py,
            data: &decompressed_data,
            offset,
            string_table: &string_table,
            datetime_class,
            date_class,
            time_class,
            uuid_class,
            decimal_class,
            recursion_depth: 0,
        };

        parser.parse()
    }
}

impl BFast {
    fn compress_parallel(&self) -> Vec<u8> {
        const CHUNK_SIZE: usize = 256 * 1024;

        let data = &self.work_buffer;
        let total_size = data.len();

        if total_size < CHUNK_SIZE * 2 {
            return compress_prepend_size(data);
        }

        let chunks: Vec<Vec<u8>> = data
            .par_chunks(CHUNK_SIZE)
            .map(|chunk| compress_prepend_size(chunk))
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
        let field_names: Vec<String> = dict.keys().iter().map(|k| k.to_string()).collect();

        let field_ids: Vec<u32> = field_names
            .iter()
            .map(|name| self.get_or_create_string_id_fast(name))
            .collect();

        // Auto-detect: check if first object has complex types
        let use_fast_mode = self.detect_simple_types(&dict, &field_names)?;

        self.ensure_buffer_capacity(5 + len * 50);
        self.work_buffer.push(0x60);
        self.work_buffer
            .extend_from_slice(&(len as u32).to_le_bytes());

        // Choose serialization path based on type detection
        if use_fast_mode {
            // Fast path: simple types only (int, str, float, bool)
            for item in list.iter() {
                self.serialize_pydantic_fast(item, &field_names, &field_ids)?;
            }
        } else {
            // Complex path: handles datetime, UUID, Decimal, etc.
            for item in list.iter() {
                self.serialize_pydantic_complex(item, &field_names, &field_ids)?;
            }
        }

        self.decrease_recursion_depth();
        Ok(())
    }

    #[inline(always)]
    fn detect_simple_types(&self, dict: &PyDict, field_names: &[String]) -> PyResult<bool> {
        // Check first object's field types
        for field_name in field_names {
            if let Some(value) = dict.get_item(field_name)? {
                if value.is_none() {
                    continue;
                }

                // Check for complex types
                if let Ok(type_name) = value.get_type().name() {
                    match type_name {
                        "datetime" | "date" | "time" | "UUID" | "Decimal" => {
                            return Ok(false); // Use complex mode
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(true) // Use fast mode
    }

    #[inline(always)]
    fn serialize_pydantic_fast(
        &mut self,
        obj: &PyAny,
        field_names: &[String],
        field_ids: &[u32],
    ) -> PyResult<()> {
        self.work_buffer.push(0x70);

        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;

        // Fast path: direct iteration for simple types
        for (i, field_name) in field_names.iter().enumerate() {
            self.work_buffer
                .extend_from_slice(&field_ids[i].to_le_bytes());

            if let Some(value) = dict.get_item(field_name)? {
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
        // Optimized for simple types only
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        if val.is_instance_of::<pyo3::types::PyBool>() {
            let b = val.extract::<bool>()?;
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }

        if val.is_instance_of::<pyo3::types::PyLong>() {
            if let Ok(n) = val.extract::<i32>() {
                if n >= 0 && n <= 7 {
                    self.work_buffer.push(0x30 | (n as u8));
                    return Ok(());
                }
                self.work_buffer.push(0x38);
                self.work_buffer
                    .extend_from_slice(&(n as i64).to_le_bytes());
                return Ok(());
            }

            if let Ok(n) = val.extract::<i64>() {
                if n >= 0 && n <= 7 {
                    self.work_buffer.push(0x30 | (n as u8));
                } else {
                    self.work_buffer.push(0x38);
                    self.work_buffer.extend_from_slice(&n.to_le_bytes());
                }
                return Ok(());
            }
        }

        if val.is_instance_of::<PyString>() {
            let py_str = val.downcast::<PyString>()?;
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.ensure_buffer_capacity(4 + bytes.len());
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        if val.is_instance_of::<pyo3::types::PyFloat>() {
            let f = val.extract::<f64>()?;
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&f.to_le_bytes());
            return Ok(());
        }

        // Fallback
        self.work_buffer.push(0x10);
        Ok(())
    }

    #[inline(always)]
    fn serialize_pydantic_complex(
        &mut self,
        obj: &PyAny,
        field_names: &[String],
        field_ids: &[u32],
    ) -> PyResult<()> {
        // Complex path: handles all types including datetime, UUID, Decimal
        self.work_buffer.push(0x70);

        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;

        for (i, field_name) in field_names.iter().enumerate() {
            self.work_buffer
                .extend_from_slice(&field_ids[i].to_le_bytes());

            if let Some(value) = dict.get_item(field_name)? {
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
        // Fast type checking using pointer comparison

        // None check (fastest)
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        // Bool check (before int, as bool is subclass of int)
        if val.is_instance_of::<pyo3::types::PyBool>() {
            let b = val.extract::<bool>()?;
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }

        // Int check (most common for IDs)
        if val.is_instance_of::<pyo3::types::PyLong>() {
            if let Ok(n) = val.extract::<i32>() {
                if n >= 0 && n <= 7 {
                    self.work_buffer.push(0x30 | (n as u8));
                    return Ok(());
                }
                self.work_buffer.push(0x38);
                self.work_buffer
                    .extend_from_slice(&(n as i64).to_le_bytes());
                return Ok(());
            }

            if let Ok(n) = val.extract::<i64>() {
                if n >= 0 && n <= 7 {
                    self.work_buffer.push(0x30 | (n as u8));
                } else {
                    self.work_buffer.push(0x38);
                    self.work_buffer.extend_from_slice(&n.to_le_bytes());
                }
                return Ok(());
            }
        }

        // String check (most common for names/emails)
        if val.is_instance_of::<PyString>() {
            let py_str = val.downcast::<PyString>()?;
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
        if val.is_instance_of::<pyo3::types::PyFloat>() {
            let f = val.extract::<f64>()?;
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&f.to_le_bytes());
            return Ok(());
        }

        // Special types (Decimal, UUID, datetime, etc.)
        if let Ok(type_name) = val.get_type().name() {
            match type_name {
                "Decimal" => {
                    let dec_str = val.str()?.extract::<String>()?;
                    self.work_buffer.push(TAG_DECIMAL);
                    let bytes = dec_str.as_bytes();
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                "UUID" => {
                    let hex_str = val.getattr("hex")?.extract::<String>()?;
                    self.work_buffer.push(TAG_UUID);
                    let bytes = hex_str.as_bytes();
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                "datetime" | "date" | "time" => {
                    let iso_str = val.call_method0("isoformat")?.extract::<String>()?;
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
                _ => {}
            }
        }

        // Enum (extract .value)
        if val.hasattr("__class__")? {
            if let Ok(class) = val.getattr("__class__") {
                if let Ok(bases) = class.getattr("__bases__") {
                    if let Ok(bases_str) = bases.str() {
                        if bases_str.to_str()?.contains("Enum") {
                            let enum_value = val.getattr("value")?;
                            return self.serialize_value_ultra_fast(enum_value);
                        }
                    }
                }
            }
        }

        // Enum handling
        if val.hasattr("__class__")? {
            if let Ok(class) = val.getattr("__class__") {
                if let Ok(bases) = class.getattr("__bases__") {
                    if let Ok(bases_tuple) = bases.downcast::<PyTuple>() {
                        for base in bases_tuple.iter() {
                            if let Ok(base_name) = base.getattr("__name__")?.extract::<String>() {
                                if base_name == "Enum" || base_name == "IntEnum" {
                                    let enum_value = val.getattr("value")?;
                                    return self.serialize_value_ultra_fast(enum_value);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.serialize_any_optimized(val)
    }

    #[inline(always)]
    fn get_or_create_string_id_fast(&mut self, key_str: &str) -> u32 {
        let mut hasher = AHasher::default();
        key_str.hash(&mut hasher);
        let hash = hasher.finish() as u32;

        // Check cache with hash comparison
        for i in 0..self.key_cache.len() {
            if let Some((cached_hash, id)) = self.key_cache[i] {
                if cached_hash == hash {
                    return id;
                }
            }
        }

        if let Some(&existing_id) = self.string_table.get(key_str) {
            self.key_cache[self.cache_index] = Some((hash, existing_id));
            self.cache_index = (self.cache_index + 1) % self.key_cache.len();
            return existing_id;
        }

        let new_id = self.next_id;
        self.string_table.insert(key_str.to_owned(), new_id);
        self.next_id += 1;

        self.key_cache[self.cache_index] = Some((hash, new_id));
        self.cache_index = (self.cache_index + 1) % self.key_cache.len();

        new_id
    }

    #[inline(always)]
    fn write_header_simd(&mut self, pos: usize, compress: bool) {
        unsafe {
            let header = self.work_buffer.as_mut_ptr().add(pos);
            ptr::write_unaligned(header as *mut u16, u16::from_le_bytes(*b"BF"));
            *header.add(2) = if compress { 0x01 } else { 0x00 };
            *header.add(3) = 0x01;
            let count = self.string_table.len() as u16;
            ptr::write_unaligned(header.add(4) as *mut u16, count.to_le());
        }
    }

    #[inline(always)]
    fn write_string_table_vectorized(&mut self) -> PyResult<()> {
        if self.string_table.is_empty() {
            return Ok(());
        }

        let total_size: usize = self.string_table.keys().map(|s| s.len() + 1).sum();
        let aligned_size = (total_size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1);
        self.work_buffer.reserve(aligned_size);

        let mut sorted: Vec<_> = self.string_table.iter().collect();
        sorted.sort_unstable_by_key(|(_, &id)| id);

        for (string, _) in sorted {
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

        if let Ok(b) = val.extract::<bool>() {
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }

        // Check special types BEFORE basic types (Decimal can be extracted as f64)
        // Decimal
        if let Ok(type_name) = val.get_type().name() {
            if type_name == "Decimal" {
                let dec_str = val.str()?.extract::<String>()?;
                self.work_buffer.push(TAG_DECIMAL);
                let bytes = dec_str.as_bytes();
                self.work_buffer
                    .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                self.work_buffer.extend_from_slice(bytes);
                return Ok(());
            }
        }

        // datetime, date, time (ISO 8601) with type preservation
        if val.hasattr("isoformat")? {
            let iso_str = val.call_method0("isoformat")?.extract::<String>()?;
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

        // UUID
        if val.hasattr("hex")? {
            if let Ok(type_name) = val.get_type().name() {
                if type_name == "UUID" {
                    let hex_str = val.getattr("hex")?.extract::<String>()?;
                    self.work_buffer.push(TAG_UUID);
                    let bytes = hex_str.as_bytes();
                    self.work_buffer
                        .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
            }
        }

        if let Ok(n) = val.extract::<i64>() {
            if n >= 0 && n <= 7 {
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                self.work_buffer.extend_from_slice(&n.to_le_bytes());
            }
            return Ok(());
        }

        if let Ok(f) = val.extract::<f64>() {
            self.work_buffer.push(0x40);
            self.work_buffer.extend_from_slice(&f.to_le_bytes());
            return Ok(());
        }

        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.work_buffer
                .extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }

        // bytes / bytearray (check before collections)
        if let Ok(py_bytes) = val.extract::<&[u8]>() {
            self.work_buffer.push(0x80);
            self.work_buffer
                .extend_from_slice(&(py_bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(py_bytes);
            return Ok(());
        }

        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            for item in list.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        // tuple (serialize as list)
        if let Ok(tuple) = val.downcast::<PyTuple>() {
            self.work_buffer.push(0x60);
            let len = tuple.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            for item in tuple.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        // set / frozenset (serialize as list)
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

        if let Ok(array) = val.extract::<PyReadonlyArrayDyn<f64>>() {
            self.work_buffer.push(0x90);
            let raw_data = array.as_slice()?;
            self.work_buffer
                .extend_from_slice(&(raw_data.len() as u32).to_le_bytes());

            let byte_slice = unsafe {
                std::slice::from_raw_parts(raw_data.as_ptr() as *const u8, raw_data.len() * 8)
            };
            self.work_buffer.extend_from_slice(byte_slice);
            return Ok(());
        }

        // Check for dict or __dict__ (Pydantic models)
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

        // Enum (extract value) - check BEFORE __dict__
        if val.hasattr("value")? && val.hasattr("name")? {
            // Check if it's actually an Enum by checking the type name
            if let Ok(_type_name) = val.get_type().name() {
                // Python Enum types have names like "Priority", "Status", etc.
                // Check if it has __class__.__bases__ that includes Enum
                if let Ok(bases) = val.getattr("__class__")?.getattr("__bases__") {
                    let bases_str = bases.str()?.extract::<String>()?;
                    if bases_str.contains("Enum") {
                        let enum_value = val.getattr("value")?;
                        return self.serialize_any_optimized(enum_value);
                    }
                }
            }
        }

        // Try __dict__ for Pydantic models
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
    m.add_class::<BFast>()?;
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
        .map(|chunk_data| lz4_flex::decompress_size_prepended(chunk_data))
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
    string_table: &'a [String],
    datetime_class: &'py PyAny,
    date_class: &'py PyAny,
    time_class: &'py PyAny,
    uuid_class: &'py PyAny,
    decimal_class: &'py PyAny,
    recursion_depth: usize,
}

impl<'a, 'py> BFastParser<'a, 'py> {
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
            return Ok(self.py.None());
        }

        // Booleans
        if tag == 0x20 {
            return Ok(false.into_py(self.py));
        }
        if tag == 0x21 {
            return Ok(true.into_py(self.py));
        }

        // Int64
        if tag == 0x38 {
            self.check_bounds(8)?;
            let val =
                i64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
            self.offset += 8;
            return Ok(val.into_py(self.py));
        }

        // Small integers (bit-packed)
        if (tag & 0xF0) == 0x30 {
            let val = (tag & 0x0F) as i64;
            return Ok(val.into_py(self.py));
        }

        // Float64
        if tag == 0x40 {
            self.check_bounds(8)?;
            let val =
                f64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
            self.offset += 8;
            return Ok(val.into_py(self.py));
        }

        // Raw string
        if tag == 0x50 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let str_bytes = &self.data[self.offset..self.offset + length];
            self.offset += length;
            let val = std::str::from_utf8(str_bytes).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid UTF-8 in string: {}",
                    e
                ))
            })?;
            return Ok(PyString::new(self.py, val).into());
        }

        // List/Array
        if tag == 0x60 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;

            let max_elements = self.data.len() - self.offset;
            let mut list = Vec::with_capacity(length.min(max_elements));
            for _ in 0..length {
                list.push(self.parse()?);
            }
            return Ok(PyList::new(self.py, list).into());
        }

        // Object start
        if tag == 0x70 {
            let dict = PyDict::new(self.py);
            while self.offset < self.data.len() && self.data[self.offset] != 0x7F {
                self.check_bounds(4)?;
                let key_id =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;

                if key_id >= self.string_table.len() {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid string table index: {}",
                        key_id
                    )));
                }

                let key = &self.string_table[key_id];
                let value = self.parse()?;
                dict.set_item(key, value)?;
            }

            if self.offset >= self.data.len() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Object not properly terminated",
                ));
            }

            self.offset += 1; // Skip 0x7F
            return Ok(dict.into());
        }

        // Bytes
        if tag == 0x80 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length)?;
            let bytes_val = &self.data[self.offset..self.offset + length];
            self.offset += length;
            return Ok(PyBytes::new(self.py, bytes_val).into());
        }

        // NumPy Array (f64)
        if tag == 0x90 {
            self.check_bounds(4)?;
            let length =
                u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                    as usize;
            self.offset += 4;
            self.check_bounds(length * 8)?;

            // Decode to a python list of floats
            let mut list = Vec::with_capacity(length);
            for _ in 0..length {
                let val =
                    f64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
                list.push(val.into_py(self.py));
                self.offset += 8;
            }
            return Ok(PyList::new(self.py, list).into());
        }

        // DateTime (0xD1) - ISO 8601 string
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
            let obj = self
                .datetime_class
                .call_method1("fromisoformat", (iso_str,))?;
            return Ok(obj.into());
        }

        // Date (0xD2) - ISO 8601 date string
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
            let obj = self.date_class.call_method1("fromisoformat", (iso_str,))?;
            return Ok(obj.into());
        }

        // Time (0xD3) - ISO 8601 time string
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
            let obj = self.time_class.call_method1("fromisoformat", (iso_str,))?;
            return Ok(obj.into());
        }

        // UUID (0xD4)
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
            let obj = self.uuid_class.call1((hex_str,))?;
            return Ok(obj.into());
        }

        // Decimal (0xD5)
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
            let obj = self.decimal_class.call1((dec_str,))?;
            return Ok(obj.into());
        }

        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown tag: 0x{:02x}",
            tag
        )))
    }
}
