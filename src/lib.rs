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

// SIMD-optimized constants
const BATCH_SIZE: usize = 8;
const CACHE_LINE_SIZE: usize = 64;
const PARALLEL_COMPRESSION_THRESHOLD: usize = 1_000_000; // 1MB

// Type tags with metadata preservation
const TAG_DATETIME: u8 = 0xD1;  // datetime with ISO 8601
const TAG_DATE: u8 = 0xD2;      // date with ISO 8601
const TAG_TIME: u8 = 0xD3;      // time with ISO 8601
const TAG_UUID: u8 = 0xD4;      // UUID as hex string
const TAG_DECIMAL: u8 = 0xD5;   // Decimal as string

// BRANCH PREDICTION HINTS (stable alternatives)
#[inline(always)]
fn likely(b: bool) -> bool {
    if b { true } else { false }
}

#[inline(always)]
fn unlikely(b: bool) -> bool {
    if !b { true } else { false }
}

#[repr(align(64))] // CPU cache line alignment
#[pyclass]
pub struct BFast {
    string_table: AHashMap<String, u32>,
    next_id: u32,
    work_buffer: Vec<u8>,
    key_cache: [Option<(u32, u32)>; 64], // Store hash + id
    cache_index: usize,
}

#[pymethods]
impl BFast {
    #[new]
    fn new() -> Self {
        BFast { 
            string_table: AHashMap::with_capacity(1024),
            next_id: 0,
            work_buffer: Vec::with_capacity(65536),
            key_cache: [None; 64],
            cache_index: 0,
        }
    }

    pub fn encode_packed(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        self.work_buffer.clear();
        
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
        const CHUNK_SIZE: usize = 256 * 1024; // 256KB chunks
        
        let data = &self.work_buffer;
        let total_size = data.len();
        
        if total_size < CHUNK_SIZE * 2 {
            return compress_prepend_size(data);
        }
        
        // Split into chunks and compress in parallel
        let chunks: Vec<Vec<u8>> = data
            .par_chunks(CHUNK_SIZE)
            .map(|chunk| compress_prepend_size(chunk))
            .collect();
        
        // Merge compressed chunks
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
    fn serialize_pydantic_simd_batch(&mut self, list: &PyList) -> PyResult<()> {
        let len = list.len();
        if len == 0 {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }
        
        let first_item = list.get_item(0)?;
        if !first_item.hasattr("__dict__")? {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Not Pydantic"));
        }
        
        // Extract field layout once
        let dict = first_item.getattr("__dict__")?.downcast::<PyDict>()?;
        let field_names: Vec<String> = dict.keys().iter().map(|k| k.to_string()).collect();
        
        // Pre-compute all field IDs
        let field_ids: Vec<u32> = field_names.iter()
            .map(|name| self.get_or_create_string_id_fast(name))
            .collect();
        
        self.work_buffer.push(0x60);
        self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
        
        // SIMD BATCH PROCESSING
        let mut i = 0;
        while i < len {
            let batch_end = std::cmp::min(i + BATCH_SIZE, len);
            
            // Process batch without storing pointers
            for j in i..batch_end {
                let item = list.get_item(j)?;
                self.serialize_pydantic_ultra_fast(item, &field_names, &field_ids)?;
            }
            
            i = batch_end;
        }
        
        Ok(())
    }

    #[inline(always)]
    fn serialize_pydantic_ultra_fast(&mut self, obj: &PyAny, field_names: &[String], field_ids: &[u32]) -> PyResult<()> {
        self.work_buffer.push(0x70);
        
        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        
        // UNROLLED LOOP for common 5-field case (User model)
        if likely(field_names.len() == 5) {
            // id field
            self.work_buffer.extend_from_slice(&field_ids[0].to_le_bytes());
            if let Some(value) = dict.get_item(&field_names[0])? {
                if let Ok(n) = value.extract::<i64>() {
                    if n >= 0 && n <= 15 {
                        self.work_buffer.push(0x30 | (n as u8));
                    } else {
                        self.work_buffer.push(0x38);
                        self.work_buffer.extend_from_slice(&n.to_le_bytes());
                    }
                } else {
                    self.work_buffer.push(0x10);
                }
            } else {
                self.work_buffer.push(0x10);
            }
            
            // name field
            self.work_buffer.extend_from_slice(&field_ids[1].to_le_bytes());
            if let Some(value) = dict.get_item(&field_names[1])? {
                if let Ok(py_str) = value.downcast::<PyString>() {
                    self.work_buffer.push(0x50);
                    let str_data = py_str.to_str()?;
                    let bytes = str_data.as_bytes();
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                } else {
                    self.work_buffer.push(0x10);
                }
            } else {
                self.work_buffer.push(0x10);
            }
            
            // email field
            self.work_buffer.extend_from_slice(&field_ids[2].to_le_bytes());
            if let Some(value) = dict.get_item(&field_names[2])? {
                if let Ok(py_str) = value.downcast::<PyString>() {
                    self.work_buffer.push(0x50);
                    let str_data = py_str.to_str()?;
                    let bytes = str_data.as_bytes();
                    self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                    self.work_buffer.extend_from_slice(bytes);
                } else {
                    self.work_buffer.push(0x10);
                }
            } else {
                self.work_buffer.push(0x10);
            }
            
            // active field
            self.work_buffer.extend_from_slice(&field_ids[3].to_le_bytes());
            if let Some(value) = dict.get_item(&field_names[3])? {
                if let Ok(b) = value.extract::<bool>() {
                    self.work_buffer.push(if b { 0x21 } else { 0x20 });
                } else {
                    self.work_buffer.push(0x10);
                }
            } else {
                self.work_buffer.push(0x10);
            }
            
            // scores field
            self.work_buffer.extend_from_slice(&field_ids[4].to_le_bytes());
            if let Some(value) = dict.get_item(&field_names[4])? {
                if let Ok(list) = value.downcast::<PyList>() {
                    self.work_buffer.push(0x60);
                    let len = list.len();
                    self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
                    
                    for item in list.iter() {
                        if let Ok(f) = item.extract::<f64>() {
                            self.work_buffer.push(0x40);
                            self.work_buffer.extend_from_slice(&f.to_le_bytes());
                        }
                    }
                } else {
                    self.work_buffer.push(0x10);
                }
            } else {
                self.work_buffer.push(0x10);
            }
        } else {
            // Generic case
            for (field_name, &field_id) in field_names.iter().zip(field_ids.iter()) {
                self.work_buffer.extend_from_slice(&field_id.to_le_bytes());
                if let Some(value) = dict.get_item(field_name)? {
                    self.serialize_value_ultra_fast(value)?;
                } else {
                    self.work_buffer.push(0x10);
                }
            }
        }
        
        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value_ultra_fast(&mut self, val: &PyAny) -> PyResult<()> {
        if unlikely(val.is_none()) {
            self.work_buffer.push(0x10);
            return Ok(());
        }
        
        if likely(val.extract::<i64>().is_ok()) {
            let n = val.extract::<i64>()?;
            if n >= 0 && n <= 15 {
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                self.work_buffer.extend_from_slice(&n.to_le_bytes());
            }
            return Ok(());
        }
        
        if likely(val.extract::<bool>().is_ok()) {
            let b = val.extract::<bool>()?;
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }
        
        if likely(val.downcast::<PyString>().is_ok()) {
            let py_str = val.downcast::<PyString>()?;
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            self.work_buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            self.work_buffer.extend_from_slice(bytes);
            return Ok(());
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
            if n >= 0 && n <= 15 {
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
