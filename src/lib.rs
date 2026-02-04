use pyo3::prelude::*;
use pyo3::types::{PyDict, PyAny, PyBytes, PyList};
use ahash::AHashMap;
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;

mod errors;
use errors::BFastError;

#[pyclass]
pub struct BFast {
    string_table: AHashMap<String, u32>,
    next_id: u32,
}

#[pymethods]
impl BFast {
    #[new]
    fn new() -> Self {
        BFast { 
            string_table: AHashMap::new(), 
            next_id: 0 
        }
    }

    pub fn encode_packed(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        let mut payload_buffer = Vec::with_capacity(1024);
        
        // First pass: serialize to collect string table
        self.serialize_value(obj, &mut payload_buffer)?;
        
        // Build complete header with string table
        let mut buffer = Vec::with_capacity(payload_buffer.len() + 256);
        self.write_header(&mut buffer, compress)?;
        
        // Append payload
        buffer.extend_from_slice(&payload_buffer);
        
        let final_data = if compress && buffer.len() > 512 {
            compress_prepend_size(&buffer)
        } else {
            buffer
        };

        // Return as Python bytes
        Python::with_gil(|py| {
            Ok(PyBytes::new(py, &final_data).into())
        })
    }
}

impl BFast {
    fn write_header(&self, buffer: &mut Vec<u8>, compress: bool) -> PyResult<()> {
        // Magic number (2 bytes): 'BF'
        buffer.extend_from_slice(b"BF");
        
        // Flags (1 byte): bit 0 = compression
        let flags = if compress { 0x01 } else { 0x00 };
        buffer.push(flags);
        
        // Version (1 byte)
        buffer.push(0x01);
        
        // String table count (2 bytes, little endian)
        let string_count = self.string_table.len() as u16;
        buffer.extend_from_slice(&string_count.to_le_bytes());
        
        // String table data: [Length (1 byte)][UTF-8 Data]
        // Sort by ID to maintain order
        let mut sorted_strings: Vec<_> = self.string_table.iter().collect();
        sorted_strings.sort_by_key(|(_, &id)| id);
        
        for (string, _) in sorted_strings {
            let bytes = string.as_bytes();
            if bytes.len() > 255 {
                return Err(BFastError::StringTooLong(string.clone()).into());
            }
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
        
        Ok(())
    }

    fn serialize_value(&mut self, val: &PyAny, buffer: &mut Vec<u8>) -> PyResult<()> {
        // 1. None/null
        if val.is_none() {
            buffer.push(0x10);
        }
        // 2. Booleans
        else if let Ok(b) = val.extract::<bool>() {
            buffer.push(if b { 0x21 } else { 0x20 });
        }
        // 3. Strings (Raw strings for now, interning handled in objects)
        else if let Ok(s) = val.extract::<String>() {
            buffer.push(0x50);
            let bytes = s.as_bytes();
            buffer.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buffer.extend_from_slice(bytes);
        }
        // 4. Integers with bit-packing
        else if let Ok(n) = val.extract::<i64>() {
            if (0..=15).contains(&n) { 
                buffer.push(0x30 | (n as u8)); 
            } else { 
                buffer.push(0x38); 
                buffer.extend_from_slice(&n.to_le_bytes());
            }
        }
        // 5. Lists/Arrays
        else if let Ok(list) = val.downcast::<PyList>() {
            buffer.push(0x60);
            buffer.extend_from_slice(&(list.len() as u32).to_le_bytes());
            for item in list.iter() {
                self.serialize_value(item, buffer)?;
            }
        }
        // 6. NumPy Arrays (Zero-copy)
        else if let Ok(array) = val.extract::<PyReadonlyArrayDyn<f64>>() {
            buffer.push(0x90);
            let raw_data = array.as_slice()?;
            buffer.extend_from_slice(&(raw_data.len() as u32).to_le_bytes());
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(raw_data.as_ptr() as *const u8, raw_data.len() * 8)
            };
            buffer.extend_from_slice(bytes);
        }
        // 7. Pydantic Models & Dictionaries (String Interning)
        else {
            let dict = if let Ok(d) = val.downcast::<PyDict>() {
                d
            } else if val.hasattr("__dict__")? {
                // Pydantic introspection - direct memory access
                val.getattr("__dict__")?.downcast::<PyDict>()?
            } else {
                return Ok(());
            };

            buffer.push(0x70);
            for (k, v) in dict.iter() {
                let key = k.to_string();
                // String interning: repeated keys become IDs
                let id = *self.string_table.entry(key).or_insert_with(|| {
                    let id = self.next_id;
                    self.next_id += 1;
                    id
                });
                buffer.extend_from_slice(&id.to_le_bytes());
                self.serialize_value(v, buffer)?;
            }
            buffer.push(0x7F);
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