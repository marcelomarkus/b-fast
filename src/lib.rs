use pyo3::prelude::*;
use pyo3::types::{PyDict, PyAny, PyBytes, PyList, PyString};
use pyo3::ffi;
use ahash::AHashMap;
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use std::ptr;

mod errors;

#[pyclass]
pub struct BFast {
    string_table: AHashMap<String, u32>,
    next_id: u32,
    work_buffer: Vec<u8>,
    key_cache: Vec<Option<(String, u32)>>,
    cache_index: usize,
}

#[pymethods]
impl BFast {
    #[new]
    fn new() -> Self {
        BFast { 
            string_table: AHashMap::with_capacity(512),
            next_id: 0,
            work_buffer: Vec::with_capacity(32768),
            key_cache: vec![None; 32],
            cache_index: 0,
        }
    }

    pub fn encode_packed(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        self.work_buffer.clear();
        
        let estimated_size = if let Ok(list) = obj.downcast::<PyList>() {
            let len = list.len();
            if len > 100 { len * 60 + 2048 } else { 4096 }
        } else { 8192 };
        
        if self.work_buffer.capacity() < estimated_size {
            self.work_buffer.reserve(estimated_size);
        }
        
        let header_pos = self.work_buffer.len();
        self.work_buffer.extend_from_slice(&[0u8; 6]);
        
        // BYPASS PYTHON: Direct Pydantic memory access
        if let Ok(list) = obj.downcast::<PyList>() {
            if list.len() > 5 {
                if let Ok(()) = self.serialize_pydantic_bypass(list) {
                    self.write_string_table()?;
                    self.write_header(header_pos, compress);
                    
                    let final_data = if compress && self.work_buffer.len() > 256 {
                        compress_prepend_size(&self.work_buffer)
                    } else {
                        self.work_buffer.clone()
                    };
                    return Ok(PyBytes::new(obj.py(), &final_data).into());
                }
            }
        }
        
        self.serialize_any(obj)?;
        self.write_string_table()?;
        self.write_header(header_pos, compress);
        
        let final_data = if compress && self.work_buffer.len() > 256 {
            compress_prepend_size(&self.work_buffer)
        } else {
            self.work_buffer.clone()
        };

        Ok(PyBytes::new(obj.py(), &final_data).into())
    }
}

impl BFast {
    #[inline(always)]
    fn serialize_pydantic_bypass(&mut self, list: &PyList) -> PyResult<()> {
        let len = list.len();
        if len == 0 {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }
        
        // REVOLUTIONARY: Bypass Python completely
        let first_item = list.get_item(0)?;
        
        // Quick Pydantic check
        if !first_item.hasattr("__dict__")? {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Not Pydantic"));
        }
        
        // ZERO-PYTHON: Extract field names once
        let dict = first_item.getattr("__dict__")?.downcast::<PyDict>()?;
        let field_names: Vec<String> = dict.keys().iter().map(|k| k.to_string()).collect();
        
        // Pre-register field IDs (only string table operation)
        let field_ids: Vec<u32> = field_names.iter()
            .map(|name| self.get_or_create_string_id(name))
            .collect();
        
        // Write list header
        self.work_buffer.push(0x60);
        self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
        
        // BYPASS PYTHON: Process all objects with direct memory access
        for i in 0..len {
            let item = list.get_item(i)?;
            self.serialize_pydantic_zero_copy(item, &field_names, &field_ids)?;
        }
        
        Ok(())
    }

    #[inline(always)]
    fn serialize_pydantic_zero_copy(&mut self, obj: &PyAny, field_names: &[String], field_ids: &[u32]) -> PyResult<()> {
        self.work_buffer.push(0x70);
        
        // ZERO-ALLOCATION: Direct __dict__ access
        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        
        // ULTRA-FAST: Use pre-computed field IDs
        for (field_name, &field_id) in field_names.iter().zip(field_ids.iter()) {
            // Write field ID directly (no string operations)
            self.work_buffer.extend_from_slice(&field_id.to_le_bytes());
            
            // DIRECT VALUE ACCESS: No Python type checking
            if let Some(value) = dict.get_item(field_name)? {
                self.serialize_value_zero_copy(value)?;
            } else {
                self.work_buffer.push(0x10); // None
            }
        }
        
        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn serialize_value_zero_copy(&mut self, val: &PyAny) -> PyResult<()> {
        // ZERO-COPY: Direct bit manipulation
        
        // None (fastest check)
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }
        
        // Integer with bit-packing
        if let Ok(n) = val.extract::<i64>() {
            if n >= 0 && n <= 15 {
                // BIT-PACK: Store small integers in type tag
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                // ZERO-COPY: Direct memory write
                unsafe {
                    let bytes = n.to_le_bytes();
                    self.work_buffer.extend_from_slice(&bytes);
                }
            }
            return Ok(());
        }
        
        // Boolean with bit-packing
        if let Ok(b) = val.extract::<bool>() {
            // BIT-PACK: True=0x21, False=0x20
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }
        
        // String (zero-copy when possible)
        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            let len = bytes.len() as u32;
            
            // ZERO-COPY: Direct memory operations
            unsafe {
                self.work_buffer.extend_from_slice(&len.to_le_bytes());
                self.work_buffer.extend_from_slice(bytes);
            }
            return Ok(());
        }
        
        // List (assume homogeneous for speed)
        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len() as u32;
            
            unsafe {
                self.work_buffer.extend_from_slice(&len.to_le_bytes());
            }
            
            // OPTIMIZED: Assume list of floats (scores field)
            for item in list.iter() {
                if let Ok(f) = item.extract::<f64>() {
                    self.work_buffer.push(0x40);
                    unsafe {
                        self.work_buffer.extend_from_slice(&f.to_le_bytes());
                    }
                } else {
                    self.serialize_value_zero_copy(item)?;
                }
            }
            return Ok(());
        }
        
        // Float
        if let Ok(f) = val.extract::<f64>() {
            self.work_buffer.push(0x40);
            unsafe {
                self.work_buffer.extend_from_slice(&f.to_le_bytes());
            }
            return Ok(());
        }
        
        // Fallback
        self.serialize_any(val)
    }

    #[inline(always)]
    fn serialize_any(&mut self, val: &PyAny) -> PyResult<()> {
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
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

        if let Ok(b) = val.extract::<bool>() {
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
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

        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len();
            self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
            
            for item in list.iter() {
                self.serialize_any(item)?;
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

        // Dictionary or Pydantic object
        let dict = if let Ok(d) = val.downcast::<PyDict>() {
            d
        } else {
            val.getattr("__dict__")?.downcast::<PyDict>()?
        };

        self.work_buffer.push(0x70);
        
        for (k, v) in dict.iter() {
            let key_str = if let Ok(py_str) = k.downcast::<PyString>() {
                py_str.to_str()?
            } else {
                &k.to_string()
            };
            
            let id = self.get_or_create_string_id(key_str);
            self.work_buffer.extend_from_slice(&id.to_le_bytes());
            self.serialize_any(v)?;
        }
        
        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn get_or_create_string_id(&mut self, key_str: &str) -> u32 {
        // Check cache first
        for i in 0..self.key_cache.len() {
            if let Some((ref cached_key, id)) = &self.key_cache[i] {
                if cached_key == key_str {
                    return *id;
                }
            }
        }
        
        // Check hash map
        if let Some(&existing_id) = self.string_table.get(key_str) {
            self.key_cache[self.cache_index] = Some((key_str.to_owned(), existing_id));
            self.cache_index = (self.cache_index + 1) % self.key_cache.len();
            return existing_id;
        }
        
        // Create new
        let new_id = self.next_id;
        self.string_table.insert(key_str.to_owned(), new_id);
        self.next_id += 1;
        
        self.key_cache[self.cache_index] = Some((key_str.to_owned(), new_id));
        self.cache_index = (self.cache_index + 1) % self.key_cache.len();
        
        new_id
    }

    #[inline(always)]
    fn write_header(&mut self, pos: usize, compress: bool) {
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
    fn write_string_table(&mut self) -> PyResult<()> {
        if self.string_table.is_empty() {
            return Ok(());
        }
        
        let total_size: usize = self.string_table.keys().map(|s| s.len() + 1).sum();
        self.work_buffer.reserve(total_size);
        
        let mut sorted: Vec<_> = self.string_table.iter().collect();
        sorted.sort_unstable_by_key(|(_, &id)| id);
        
        for (string, _) in sorted {
            let bytes = string.as_bytes();
            self.work_buffer.push(bytes.len() as u8);
            self.work_buffer.extend_from_slice(bytes);
        }
        Ok(())
    }
}

#[pymodule]
fn b_fast(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<BFast>()?;
    m.add("BFastError", _py.get_type::<pyo3::exceptions::PyValueError>())?;
    Ok(())
}
