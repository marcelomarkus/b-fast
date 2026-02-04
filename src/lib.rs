use pyo3::prelude::*;
use pyo3::types::{PyDict, PyAny, PyBytes, PyList, PyString};
use pyo3::ffi;
use ahash::AHashMap;
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use std::ptr;

mod errors;

// Global caches for ultra-fast access
static mut TYPE_CACHE: Option<AHashMap<*const ffi::PyTypeObject, u8>> = None;
static mut PYDANTIC_FIELDS_CACHE: Option<AHashMap<*const ffi::PyTypeObject, Vec<String>>> = None;
static mut CACHE_INITIALIZED: bool = false;

const TYPE_PYDANTIC: u8 = 8;

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
        unsafe {
            if !CACHE_INITIALIZED {
                TYPE_CACHE = Some(AHashMap::with_capacity(64));
                PYDANTIC_FIELDS_CACHE = Some(AHashMap::with_capacity(32));
                CACHE_INITIALIZED = true;
            }
        }
        
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
        
        // Intelligent pre-allocation
        let estimated_size = if let Ok(list) = obj.downcast::<PyList>() {
            let len = list.len();
            if len > 100 { len * 80 + 4096 } else { 8192 }
        } else { 16384 };
        
        if self.work_buffer.capacity() < estimated_size {
            self.work_buffer.reserve(estimated_size);
        }
        
        let header_pos = self.work_buffer.len();
        self.work_buffer.extend_from_slice(&[0u8; 6]);
        
        // ULTRA-FAST: Direct Pydantic processing for lists
        if let Ok(list) = obj.downcast::<PyList>() {
            if list.len() > 10 {
                if let Ok(()) = self.serialize_pydantic_list_direct(list) {
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
        
        // Fallback to regular serialization
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
    fn serialize_pydantic_list_direct(&mut self, list: &PyList) -> PyResult<()> {
        let len = list.len();
        if len == 0 {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }
        
        // Check if first item is actually a Pydantic object
        let first_item = list.get_item(0)?;
        if !first_item.hasattr("__dict__")? {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Not Pydantic"));
        }
        
        // FORCE the optimized path - extract fields from first object
        let first_dict = first_item.getattr("__dict__")?.downcast::<PyDict>()?;
        let field_names: Vec<String> = first_dict.keys().iter()
            .map(|k| k.to_string())
            .collect();
        
        // Pre-register all field names in string table
        let field_ids: Vec<u32> = field_names.iter()
            .map(|name| self.get_or_create_string_id(name))
            .collect();
        
        // Write list header
        self.work_buffer.push(0x60);
        self.work_buffer.extend_from_slice(&(len as u32).to_le_bytes());
        
        // ULTRA-OPTIMIZED: Process all objects with known structure
        for i in 0..len {
            let item = list.get_item(i)?;
            
            // DIRECT __dict__ access without any checks
            let dict = item.getattr("__dict__")?.downcast::<PyDict>()?;
            
            self.work_buffer.push(0x70);
            
            // Use pre-computed field IDs (no string lookups!)
            for (field_name, &field_id) in field_names.iter().zip(field_ids.iter()) {
                self.work_buffer.extend_from_slice(&field_id.to_le_bytes());
                
                if let Some(value) = dict.get_item(field_name)? {
                    // ULTRA-FAST primitive serialization
                    self.serialize_primitive_ultra_fast(value)?;
                } else {
                    self.work_buffer.push(0x10); // None
                }
            }
            
            self.work_buffer.push(0x7F);
        }
        
        Ok(())
    }

    #[inline(always)]
    fn extract_pydantic_fields(&self, obj: &PyAny) -> PyResult<Vec<String>> {
        // Try __pydantic_fields__ first (Pydantic v2)
        if let Ok(fields_info) = obj.getattr("__pydantic_fields__") {
            if let Ok(dict) = fields_info.downcast::<PyDict>() {
                return Ok(dict.keys().iter().map(|k| k.to_string()).collect());
            }
        }
        
        // Fallback to __dict__
        if let Ok(dict) = obj.getattr("__dict__") {
            if let Ok(py_dict) = dict.downcast::<PyDict>() {
                return Ok(py_dict.keys().iter().map(|k| k.to_string()).collect());
            }
        }
        
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("Not a Pydantic object"))
    }

    #[inline(always)]
    fn serialize_pydantic_object_direct(&mut self, obj: &PyAny, field_names: &[String], field_ids: &[u32]) -> PyResult<()> {
        self.work_buffer.push(0x70);
        
        // Get __dict__ directly
        let dict = obj.getattr("__dict__")?.downcast::<PyDict>()?;
        
        // Use pre-computed field IDs for maximum speed
        for (field_name, &field_id) in field_names.iter().zip(field_ids.iter()) {
            self.work_buffer.extend_from_slice(&field_id.to_le_bytes());
            
            if let Some(value) = dict.get_item(field_name)? {
                self.serialize_primitive_ultra_fast(value)?;
            } else {
                self.work_buffer.push(0x10); // None
            }
        }
        
        self.work_buffer.push(0x7F);
        Ok(())
    }

    #[inline(always)]
    fn serialize_primitive_ultra_fast(&mut self, val: &PyAny) -> PyResult<()> {
        // ULTRA-OPTIMIZED: Assume most common types first
        
        // None (very common)
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }
        
        // Integer (most common in Pydantic)
        if let Ok(n) = val.extract::<i64>() {
            if n >= 0 && n <= 15 {
                self.work_buffer.push(0x30 | (n as u8));
            } else {
                self.work_buffer.push(0x38);
                unsafe {
                    let bytes = n.to_le_bytes();
                    self.work_buffer.extend_from_slice(&bytes);
                }
            }
            return Ok(());
        }
        
        // Boolean
        if let Ok(b) = val.extract::<bool>() {
            self.work_buffer.push(if b { 0x21 } else { 0x20 });
            return Ok(());
        }
        
        // String (very common in Pydantic)
        if let Ok(py_str) = val.downcast::<PyString>() {
            self.work_buffer.push(0x50);
            let str_data = py_str.to_str()?;
            let bytes = str_data.as_bytes();
            let len = bytes.len() as u32;
            unsafe {
                self.work_buffer.extend_from_slice(&len.to_le_bytes());
                self.work_buffer.extend_from_slice(bytes);
            }
            return Ok(());
        }
        
        // List (common for scores field)
        if let Ok(list) = val.downcast::<PyList>() {
            self.work_buffer.push(0x60);
            let len = list.len() as u32;
            unsafe {
                self.work_buffer.extend_from_slice(&len.to_le_bytes());
            }
            
            // Assume list of floats (common case)
            for item in list.iter() {
                if let Ok(f) = item.extract::<f64>() {
                    self.work_buffer.push(0x40); // Float marker
                    unsafe {
                        self.work_buffer.extend_from_slice(&f.to_le_bytes());
                    }
                } else {
                    // Fallback for non-float items
                    self.serialize_primitive_ultra_fast(item)?;
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
        
        // Fallback (should be rare)
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
