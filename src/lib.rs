use pyo3::prelude::*;
use pyo3::types::{PyDict, PyAny, PyBytes, PyList, PyString, PyTuple, PySet, PyFrozenSet};
use ahash::{AHashMap, AHasher};
use std::hash::{Hash, Hasher};
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use std::ptr;
use std::mem;
use rayon::prelude::*;

mod errors;
mod allocator;

// Performance tuning constants
const BATCH_SIZE: usize = 8;
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

// Fast path markers for common cases
#[inline(always)]
const fn is_fast_path(b: bool) -> bool {
    b
}

#[cold]
#[inline(never)]
fn handle_slow_path<T, E>(result: Result<T, E>) -> Result<T, E> {
    result
}

#[repr(align(64))]
#[pyclass]
pub struct BFast {
    string_table: AHashMap<String, u32>,
    next_id: u32,
    work_buffer: Vec<u8>,
    key_cache: [Option<(u32, u32)>; 64],
    cache_index: usize,
    recursion_depth: usize,
}

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
                "Maximum recursion depth exceeded"
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
        if is_fast_path(len == 0) {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }
        
        self.check_recursion_depth()?;
        
        let first_item = list.get_item(0)?;
        if !first_item.hasattr("__dict__")? {
            self.decrease_recursion_depth();
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Not Pydantic"));
        }
        
        let dict = first_item.getattr("__dict__")?.downcast::<PyDict>()?;
        let field_names: Vec<String> = dict.keys().iter().map(|k| k.to_string()).collect();
        
        let field_ids: Vec<u32> = field_names.iter()
            .map(|name| self.get_or_create_string_id_fast(name))
            .collect();
        
        // Auto-detect: check if first object has complex types
        let use_fast_mode = self.detect_simple_types(&dict, &field_names)?;
        
        
        self.ensure_buffer_capacity(5 + len * 50);
        self.work_buffer.push(0x60);
        self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
        
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
    fn serialize_pydantic_fast(&mut self, obj: &PyAny, field_names: &[String], field_ids: &[u32]) -> PyResult<()> {
        self.work_buffer.push(0x70);
        
        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        
        // Fast path: direct iteration for simple types
        for (i, field_name) in field_names.iter().enumerate() {
            self.work_buffer.extend_from_slice(&field_ids[i].to_le_bytes());
            
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
                self.work_buffer.extend_from_slice(&(n as i64).to_le_bytes());
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
            self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
    fn serialize_pydantic_complex(&mut self, obj: &PyAny, field_names: &[String], field_ids: &[u32]) -> PyResult<()> {
        // Complex path: handles all types including datetime, UUID, Decimal
        self.work_buffer.push(0x70);
        
        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        
        for (i, field_name) in field_names.iter().enumerate() {
            self.work_buffer.extend_from_slice(&field_ids[i].to_le_bytes());
            
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
                self.work_buffer.extend_from_slice(&(n as i64).to_le_bytes());
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
            self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                    return Ok(());
                }
                "UUID" => {
                    let hex_str = val.getattr("hex")?.extract::<String>()?;
                    self.work_buffer.push(TAG_UUID);
                    let bytes = hex_str.as_bytes();
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
                self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
            self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
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
            self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
        }
        
        // bytes / bytearray (check before collections)
        if let Ok(py_bytes) = val.extract::<&[u8]>() {
            self.work_buffer.push(0x80);
            self.work_buffer.extend_from_slice(&(py_bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(py_bytes);
            return Ok(());
        }

        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len();
            self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
            
            for item in list.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }
        
        // tuple (serialize as list)
        if let Ok(tuple) = val.downcast::<PyTuple>() {
            self.work_buffer.push(0x60);
            let len = tuple.len();
            self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
            
            for item in tuple.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }
        
        // set / frozenset (serialize as list)
        if let Ok(set) = val.downcast::<PySet>() {
            self.work_buffer.push(0x60);
            let len = set.len();
            self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
            
            for item in set.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }
        
        if let Ok(frozenset) = val.downcast::<PyFrozenSet>() {
            self.work_buffer.push(0x60);
            let len = frozenset.len();
            self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
            
            for item in frozenset.iter() {
                self.serialize_any_optimized(item)?;
            }
            return Ok(());
        }

        if let Ok(array) = val.extract::<PyReadonlyArrayDyn<f64>>() {
            self.work_buffer.push(0x90);
            let raw_data = array.as_slice()?;
            self.work_buffer.extend_from_slice(&(raw_data.len() as u32).to_le_bytes());
            
            let byte_slice = unsafe {
                std::slice::from_raw_parts(
                    raw_data.as_ptr() as *const u8, 
                    raw_data.len() * 8
                )
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
            if let Ok(type_name) = val.get_type().name() {
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
        self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.work_buffer.extend_from_slice(bytes);
        Ok(())
    }
}

#[pymodule]
fn b_fast(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<BFast>()?;
    m.add("BFastError", _py.get_type::<pyo3::exceptions::PyValueError>())?;
    Ok(())
}
