#![allow(non_local_definitions)]

use ahash::AHashMap;
use lz4_flex::compress_prepend_size;
use numpy::PyReadonlyArrayDyn;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyFrozenSet, PyList, PySet, PyString, PyTuple};
use rayon::prelude::*;
use std::borrow::Cow;
use std::ptr;

mod errors;
pub mod lazy;
pub mod native;
pub mod streaming;
use std::sync::Arc;

extern "C" {
    fn _PyObject_GetDictPtr(obj: *mut pyo3::ffi::PyObject) -> *mut *mut pyo3::ffi::PyObject;
}

// Performance tuning constants
const PARALLEL_COMPRESSION_THRESHOLD: usize = 1_000_000;
const INITIAL_BUFFER_SIZE: usize = 65536;
const MAX_RECURSION_DEPTH: usize = 128;

// Type tags with metadata preservation
const TAG_DATETIME: u8 = 0xD1;
const TAG_DATE: u8 = 0xD2;
const TAG_TIME: u8 = 0xD3;
const TAG_UUID: u8 = 0xD4;
const TAG_DECIMAL: u8 = 0xD5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FastFieldKind {
    Int,
    Float,
    Bool,
    String,
    FloatList,
    IntList,
    Generic,
}

#[derive(Clone, Copy)]
pub(crate) struct FastFieldInfo {
    pub(crate) key_ptr: *mut pyo3::ffi::PyObject,
    pub(crate) id_bytes: [u8; 4],
    pub(crate) kind: FastFieldKind,
}

pub(crate) struct TemplateField {
    pub(crate) key: pyo3::Py<pyo3::types::PyAny>,
    pub(crate) id_bytes: [u8; 4],
    pub(crate) kind: FastFieldKind,
}

pub(crate) struct HomogeneousTemplate {
    pub(crate) prefix: Vec<u8>,
    pub(crate) fields: Vec<TemplateField>,
    pub(crate) is_pydantic: bool,
}

#[allow(non_local_definitions)]
#[pyclass]
pub struct BFast {
    work_buffer: Vec<u8>,
    string_table_bytes: Vec<u8>,
    next_id: u32,
    last_was_compressed: bool,
    fast_cpython_abi: bool,
    recursion_depth: usize,
    ascii_str_offset: usize,
    long_tag_zero: usize,
    long_tag_one: usize,
    py_none: PyObject,
    py_true: PyObject,
    py_false: PyObject,
    dict_attr: PyObject,
    string_table: AHashMap<String, u32>,
    payload_buffer: Vec<u8>,
    compressed_buffer: Vec<u8>,
    decompress_buffer: Vec<u8>,
    small_ints: [PyObject; 16],
    key_cache: [Option<(u64, u32)>; 64],
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
            let small_ints = std::array::from_fn(|i| unsafe {
                pyo3::Bound::from_owned_ptr(py, pyo3::ffi::PyLong_FromLongLong(i as i64))
                    .into_any()
                    .unbind()
            });
            let dict_attr = PyString::intern(py, "__dict__").into_any().unbind();
            let dict_ptr = dict_attr.as_ptr();
            let mut str_sz: pyo3::ffi::Py_ssize_t = 0;
            let utf8_ptr = unsafe { pyo3::ffi::PyUnicode_AsUTF8AndSize(dict_ptr, &mut str_sz) };
            let ascii_str_offset = if !utf8_ptr.is_null() && str_sz == 8 {
                let off = (utf8_ptr as usize).wrapping_sub(dict_ptr as usize);
                let len_at_16 = unsafe { *(dict_ptr.byte_add(16) as *const isize) };
                let state_at_32 = unsafe { *dict_ptr.byte_add(32).cast::<u8>() };
                if (32..=64).contains(&off) && len_at_16 == 8 && (state_at_32 & 0x7C) == 0x64 {
                    off
                } else {
                    0
                }
            } else {
                0
            };

            let f_test = pyo3::types::PyFloat::new(py, 3.25);
            let l_test = pyo3::types::PyList::new(py, [&f_test]);
            let fast_cpython_abi = ascii_str_offset != 0
                && unsafe { *(f_test.as_ptr().byte_add(16) as *const f64) == 3.25 }
                && l_test.is_ok_and(|l| unsafe {
                    *(l.as_ptr().byte_add(16) as *const isize) == 1
                        && **(l.as_ptr().byte_add(24) as *const *const *mut pyo3::ffi::PyObject)
                            == f_test.as_ptr()
                });

            let (long_tag_zero, long_tag_one) = if fast_cpython_abi {
                unsafe {
                    let p0 = small_ints[0].as_ptr();
                    let p1 = small_ints[1].as_ptr();
                    let p5 = small_ints[5].as_ptr();
                    let t0 = *(p0.byte_add(16) as *const usize);
                    let t1 = *(p1.byte_add(16) as *const usize);
                    let t5 = *(p5.byte_add(16) as *const usize);
                    let d1 = *(p1.byte_add(24) as *const u32);
                    let d5 = *(p5.byte_add(24) as *const u32);
                    if t1 == t5 && t0 != t1 && d1 == 1 && d5 == 5 {
                        (t0, t1)
                    } else {
                        (usize::MAX, usize::MAX)
                    }
                }
            } else {
                (usize::MAX, usize::MAX)
            };

            BFast {
                work_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                string_table_bytes: Vec::with_capacity(512),
                next_id: 0,
                last_was_compressed: false,
                fast_cpython_abi,
                recursion_depth: 0,
                ascii_str_offset,
                long_tag_zero,
                long_tag_one,
                py_none: py.None(),
                py_true: pyo3::types::PyBool::new(py, true)
                    .to_owned()
                    .into_any()
                    .unbind(),
                py_false: pyo3::types::PyBool::new(py, false)
                    .to_owned()
                    .into_any()
                    .unbind(),
                dict_attr,
                string_table: AHashMap::with_capacity(64),
                payload_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                compressed_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                decompress_buffer: Vec::with_capacity(INITIAL_BUFFER_SIZE),
                small_ints,
                key_cache: [None; 64],
            }
        })
    }

    pub fn encode_packed(&mut self, obj: &Bound<'_, PyAny>, compress: bool) -> PyResult<PyObject> {
        if !compress && unsafe { pyo3::ffi::PyList_CheckExact(obj.as_ptr()) != 0 } {
            let list = unsafe { obj.downcast_unchecked::<PyList>() };
            let list_len = if self.fast_cpython_abi {
                unsafe { *(list.as_ptr().byte_add(16) as *const isize) as usize }
            } else {
                list.len()
            };
            if list_len > 8 && self.recursion_depth < MAX_RECURSION_DEPTH {
                if let Some(res) = self.encode_pydantic_batch_direct_pybytes(list) {
                    return Ok(res);
                }
            }
        }
        self.encode_packed_internal(obj, compress)?;
        let slice = if self.last_was_compressed {
            &self.compressed_buffer
        } else {
            &self.work_buffer
        };
        Self::create_pybytes_fast(obj.py(), slice)
    }

    #[pyo3(signature = (obj, *, compress = true))]
    pub fn encode_frame(&mut self, obj: &Bound<'_, PyAny>, compress: bool) -> PyResult<PyObject> {
        let py = obj.py();
        self.encode_packed_internal(obj, compress)?;
        let is_compressed = if self.last_was_compressed { 1u8 } else { 0u8 };
        let slice_len = if self.last_was_compressed {
            self.compressed_buffer.len()
        } else {
            self.work_buffer.len()
        };
        let len = slice_len as u32;
        let total_len = 6 + slice_len;
        unsafe {
            let ptr = pyo3::ffi::PyBytes_FromStringAndSize(
                ptr::null(),
                total_len as pyo3::ffi::Py_ssize_t,
            );
            if ptr.is_null() {
                return Err(PyErr::fetch(py));
            }
            let dst = pyo3::ffi::PyBytes_AsString(ptr) as *mut u8;
            ptr::write_unaligned(dst as *mut u32, len.to_le());
            *dst.add(4) = crate::streaming::FRAME_DATA;
            *dst.add(5) = is_compressed;
            let src_ptr = if self.last_was_compressed {
                self.compressed_buffer.as_ptr()
            } else {
                self.work_buffer.as_ptr()
            };
            Self::copy_bytes_no_erms(dst.add(6), src_ptr, slice_len);
            Ok(PyObject::from_owned_ptr(py, ptr))
        }
    }

    #[staticmethod]
    pub fn get_stream_handshake(py: Python) -> PyResult<PyObject> {
        Ok(
            PyBytes::new(py, &[0x42, 0x53, crate::streaming::STREAM_VERSION, 0x00])
                .unbind()
                .into_any(),
        )
    }

    #[staticmethod]
    pub fn get_eos_frame(py: Python) -> PyResult<PyObject> {
        Ok(PyBytes::new(
            py,
            &[0x00, 0x00, 0x00, 0x00, crate::streaming::FRAME_EOS, 0x00],
        )
        .unbind()
        .into_any())
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

    #[pyo3(signature = (bytes, *, decompress = true))]
    pub fn decode_lazy(
        &mut self,
        _py: Python,
        bytes: &[u8],
        decompress: bool,
    ) -> PyResult<lazy::BFastLazyView> {
        let decompressed_data: Arc<Vec<u8>> = if decompress {
            if bytes.len() < 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Buffer too small for B-FAST payload",
                ));
            }
            if &bytes[0..2] == b"BF" {
                Arc::new(bytes.to_vec())
            } else {
                if bytes.len() < 4 {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Buffer too small for compressed B-FAST data",
                    ));
                }
                let uncompressed_size =
                    u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
                let mut buf = vec![0u8; uncompressed_size];
                if lz4_flex::decompress_into(&bytes[4..], &mut buf).is_ok() {
                    Arc::new(buf)
                } else {
                    let decompressed = decompress_packed(bytes)
                        .map_err(pyo3::exceptions::PyValueError::new_err)?;
                    Arc::new(decompressed.into_owned())
                }
            }
        } else {
            Arc::new(bytes.to_vec())
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
            string_table.push(string_val.to_string());
            offset += length;
        }

        Ok(lazy::BFastLazyView::new(
            decompressed_data,
            offset,
            Arc::new(string_table),
        ))
    }
}

impl BFast {
    fn encode_packed_internal(&mut self, obj: &Bound<'_, PyAny>, compress: bool) -> PyResult<()> {
        self.work_buffer.clear();
        self.recursion_depth = 0;

        if unsafe { pyo3::ffi::PyList_CheckExact(obj.as_ptr()) != 0 } {
            let list = unsafe { obj.downcast_unchecked::<PyList>() };
            if list.len() > 8 {
                if self.recursion_depth >= MAX_RECURSION_DEPTH {
                    return Err(PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
                        "Maximum recursion depth exceeded",
                    ));
                }
                if self.encode_pydantic_batch_direct(list, compress) {
                    return self.finalize_compression(compress);
                }
                self.work_buffer.clear();
            }
        }

        self.encode_packed_two_buffer(obj, compress)
    }

    #[inline(never)]
    fn encode_packed_two_buffer(&mut self, obj: &Bound<'_, PyAny>, compress: bool) -> PyResult<()> {
        // Two-buffer path (handles extended types, nested dicts, and complex models)
        self.payload_buffer.clear();
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

    #[inline(never)]
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

    fn decrease_recursion_depth(&mut self) {
        self.recursion_depth -= 1;
    }

    #[inline(always)]
    fn create_pybytes_fast(py: Python<'_>, slice: &[u8]) -> PyResult<PyObject> {
        let len = slice.len();
        unsafe {
            let ptr =
                pyo3::ffi::PyBytes_FromStringAndSize(ptr::null(), len as pyo3::ffi::Py_ssize_t);
            if ptr.is_null() {
                return Err(PyErr::fetch(py));
            }
            let dst = pyo3::ffi::PyBytes_AsString(ptr) as *mut u8;
            Self::copy_bytes_no_erms(dst, slice.as_ptr(), len);
            Ok(PyObject::from_owned_ptr(py, ptr))
        }
    }

    #[inline(always)]
    unsafe fn copy_bytes_short(dst: *mut u8, src: *const u8, len: usize) {
        if len <= 16 {
            if len >= 8 {
                let a = ptr::read_unaligned(src as *const u64);
                let b = ptr::read_unaligned(src.add(len - 8) as *const u64);
                ptr::write_unaligned(dst as *mut u64, a);
                ptr::write_unaligned(dst.add(len - 8) as *mut u64, b);
            } else if len >= 4 {
                let a = ptr::read_unaligned(src as *const u32);
                let b = ptr::read_unaligned(src.add(len - 4) as *const u32);
                ptr::write_unaligned(dst as *mut u32, a);
                ptr::write_unaligned(dst.add(len - 4) as *mut u32, b);
            } else if len > 0 {
                *dst = *src;
                *dst.add(len >> 1) = *src.add(len >> 1);
                *dst.add(len - 1) = *src.add(len - 1);
            }
        } else if len <= 32 {
            let a0 = ptr::read_unaligned(src as *const u64);
            let a1 = ptr::read_unaligned(src.add(8) as *const u64);
            let b0 = ptr::read_unaligned(src.add(len - 16) as *const u64);
            let b1 = ptr::read_unaligned(src.add(len - 8) as *const u64);
            ptr::write_unaligned(dst as *mut u64, a0);
            ptr::write_unaligned(dst.add(8) as *mut u64, a1);
            ptr::write_unaligned(dst.add(len - 16) as *mut u64, b0);
            ptr::write_unaligned(dst.add(len - 8) as *mut u64, b1);
        } else if len <= 64 {
            let a0 = ptr::read_unaligned(src as *const u64);
            let a1 = ptr::read_unaligned(src.add(8) as *const u64);
            let a2 = ptr::read_unaligned(src.add(16) as *const u64);
            let a3 = ptr::read_unaligned(src.add(24) as *const u64);
            let b0 = ptr::read_unaligned(src.add(len - 32) as *const u64);
            let b1 = ptr::read_unaligned(src.add(len - 24) as *const u64);
            let b2 = ptr::read_unaligned(src.add(len - 16) as *const u64);
            let b3 = ptr::read_unaligned(src.add(len - 8) as *const u64);
            ptr::write_unaligned(dst as *mut u64, a0);
            ptr::write_unaligned(dst.add(8) as *mut u64, a1);
            ptr::write_unaligned(dst.add(16) as *mut u64, a2);
            ptr::write_unaligned(dst.add(24) as *mut u64, a3);
            ptr::write_unaligned(dst.add(len - 32) as *mut u64, b0);
            ptr::write_unaligned(dst.add(len - 24) as *mut u64, b1);
            ptr::write_unaligned(dst.add(len - 16) as *mut u64, b2);
            ptr::write_unaligned(dst.add(len - 8) as *mut u64, b3);
        } else {
            Self::copy_bytes_no_erms(dst, src, len);
        }
    }

    #[inline(always)]
    unsafe fn copy_bytes_no_erms(mut dst: *mut u8, mut src: *const u8, mut len: usize) {
        while len >= 64 {
            let c0 = ptr::read_unaligned(src as *const u128);
            let c1 = ptr::read_unaligned(src.add(16) as *const u128);
            let c2 = ptr::read_unaligned(src.add(32) as *const u128);
            let c3 = ptr::read_unaligned(src.add(48) as *const u128);
            ptr::write_unaligned(dst as *mut u128, c0);
            ptr::write_unaligned(dst.add(16) as *mut u128, c1);
            ptr::write_unaligned(dst.add(32) as *mut u128, c2);
            ptr::write_unaligned(dst.add(48) as *mut u128, c3);
            src = src.add(64);
            dst = std::hint::black_box(dst.add(64));
            len -= 64;
        }
        if len > 0 {
            Self::copy_bytes_short(dst, src, len);
        }
    }

    #[inline(always)]
    fn detect_fast_field_kind(&self, val_ptr: *mut pyo3::ffi::PyObject) -> FastFieldKind {
        let py_true_ptr = self.py_true.as_ptr();
        let py_false_ptr = self.py_false.as_ptr();
        if val_ptr == py_true_ptr || val_ptr == py_false_ptr {
            FastFieldKind::Bool
        } else if unsafe { pyo3::ffi::PyLong_CheckExact(val_ptr) != 0 } {
            FastFieldKind::Int
        } else if unsafe { pyo3::ffi::PyUnicode_CheckExact(val_ptr) != 0 } {
            FastFieldKind::String
        } else if unsafe { pyo3::ffi::PyFloat_CheckExact(val_ptr) != 0 } {
            FastFieldKind::Float
        } else if unsafe { pyo3::ffi::PyList_CheckExact(val_ptr) != 0 } {
            let l_len = unsafe { pyo3::ffi::PyList_Size(val_ptr) };
            if l_len > 0 {
                let elem0 = unsafe { pyo3::ffi::PyList_GetItem(val_ptr, 0) };
                if !elem0.is_null() && unsafe { pyo3::ffi::PyFloat_CheckExact(elem0) != 0 } {
                    FastFieldKind::FloatList
                } else if !elem0.is_null()
                    && elem0 != py_true_ptr
                    && elem0 != py_false_ptr
                    && unsafe { pyo3::ffi::PyLong_CheckExact(elem0) != 0 }
                {
                    FastFieldKind::IntList
                } else {
                    FastFieldKind::Generic
                }
            } else {
                FastFieldKind::Generic
            }
        } else {
            FastFieldKind::Generic
        }
    }

    #[inline(always)]
    fn encode_pydantic_batch_direct(&mut self, list: &Bound<'_, PyList>, compress: bool) -> bool {
        let list_ptr = list.as_ptr();
        let len = if self.fast_cpython_abi {
            unsafe { *(list_ptr.byte_add(16) as *const isize) as usize }
        } else {
            list.len()
        };
        if len == 0 {
            return false;
        }

        self.recursion_depth += 1;

        let first_ptr = unsafe {
            if self.fast_cpython_abi {
                let items = *(list_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject);
                *items
            } else {
                pyo3::ffi::PyList_GetItem(list_ptr, 0)
            }
        };
        if first_ptr.is_null() {
            self.decrease_recursion_depth();
            return false;
        }
        let dict_attr_ptr = self.dict_attr.as_ptr();

        let mut needs_first_decref = false;
        let mut first_type_ptr: *mut pyo3::ffi::PyTypeObject = ptr::null_mut();
        let mut obj_dict_offset: isize = 0;

        let (is_pydantic, first_dict_ptr) = unsafe {
            let dict_ptr_ptr = _PyObject_GetDictPtr(first_ptr);
            if !dict_ptr_ptr.is_null() && !(*dict_ptr_ptr).is_null() {
                let d = *dict_ptr_ptr;
                if pyo3::ffi::PyDict_CheckExact(d) != 0 || pyo3::ffi::PyDict_Check(d) != 0 {
                    first_type_ptr = (*first_ptr).ob_type;
                    obj_dict_offset = (dict_ptr_ptr as isize) - (first_ptr as isize);
                    (true, d)
                } else {
                    self.decrease_recursion_depth();
                    return false;
                }
            } else if pyo3::ffi::PyDict_CheckExact(first_ptr) != 0
                || pyo3::ffi::PyDict_Check(first_ptr) != 0
            {
                (false, first_ptr)
            } else {
                let p = pyo3::ffi::PyObject_GetAttr(first_ptr, dict_attr_ptr);
                if !p.is_null() {
                    if pyo3::ffi::PyDict_CheckExact(p) != 0 || pyo3::ffi::PyDict_Check(p) != 0 {
                        needs_first_decref = true;
                        (true, p)
                    } else {
                        pyo3::ffi::Py_DECREF(p);
                        self.decrease_recursion_depth();
                        return false;
                    }
                } else {
                    pyo3::ffi::PyErr_Clear();
                    self.decrease_recursion_depth();
                    return false;
                }
            }
        };

        let dict_len = unsafe { pyo3::ffi::PyDict_Size(first_dict_ptr) };
        if dict_len <= 0 || dict_len > 32 {
            if needs_first_decref {
                unsafe { pyo3::ffi::Py_DECREF(first_dict_ptr) };
            }
            self.decrease_recursion_depth();
            return false;
        }

        let mut fields_uninit = [std::mem::MaybeUninit::<FastFieldInfo>::uninit(); 32];
        let mut field_count = 0;
        let mut first_val0_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut first_val1_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();

        let mut pos: pyo3::ffi::Py_ssize_t = 0;
        let mut key_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut val_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let ascii_off = self.ascii_str_offset;

        while unsafe {
            pyo3::ffi::PyDict_Next(first_dict_ptr, &mut pos, &mut key_ptr, &mut val_ptr)
        } != 0
        {
            let kind = self.detect_fast_field_kind(val_ptr);
            if kind == FastFieldKind::Generic {
                if needs_first_decref {
                    unsafe { pyo3::ffi::Py_DECREF(first_dict_ptr) };
                }
                self.decrease_recursion_depth();
                return false;
            }

            let (utf8_ptr, str_len) = unsafe {
                if ascii_off != 0
                    && (*key_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyUnicode_Type)
                    && (*key_ptr.byte_add(32).cast::<u8>() & 0x7C) == 0x64
                {
                    let sz = *(key_ptr.byte_add(16) as *const isize);
                    if sz >= 0 {
                        (key_ptr.byte_add(ascii_off) as *const u8, sz as usize)
                    } else {
                        return false;
                    }
                } else {
                    let mut str_size: pyo3::ffi::Py_ssize_t = 0;
                    let p = pyo3::ffi::PyUnicode_AsUTF8AndSize(key_ptr, &mut str_size);
                    if p.is_null() {
                        pyo3::ffi::PyErr_Clear();
                        if needs_first_decref {
                            pyo3::ffi::Py_DECREF(first_dict_ptr);
                        }
                        self.decrease_recursion_depth();
                        return false;
                    }
                    (p as *const u8, str_size as usize)
                }
            };
            let key_bytes = unsafe { std::slice::from_raw_parts(utf8_ptr, str_len) };
            let name = unsafe { std::str::from_utf8_unchecked(key_bytes) };
            let id = self.get_or_create_string_id_fast(name);

            if field_count == 0 {
                first_val0_ptr = val_ptr;
            } else if field_count == 1 {
                first_val1_ptr = val_ptr;
            }

            fields_uninit[field_count].write(FastFieldInfo {
                key_ptr,
                id_bytes: id.to_le_bytes(),
                kind,
            });
            field_count += 1;
        }

        let active_fields = unsafe {
            std::slice::from_raw_parts(fields_uninit.as_ptr() as *const FastFieldInfo, field_count)
        };

        let mut dk_header_word = 0u64;
        let mut entries_offset = 0usize;
        let mut entry_stride = 0usize;

        if self.fast_cpython_abi && field_count >= 1 {
            unsafe {
                if (*first_dict_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyDict_Type)
                    && *(first_dict_ptr.byte_add(16) as *const usize) == field_count
                    && *(first_dict_ptr.byte_add(40) as *const usize) == 0
                {
                    let ma_keys = *(first_dict_ptr.byte_add(32) as *const *const u8);
                    if !ma_keys.is_null() && *(ma_keys.add(24) as *const usize) == field_count {
                        let k0 = active_fields[0].key_ptr;
                        let mut off = 32usize;
                        while off <= 88 {
                            let cand_k = *(ma_keys.add(off) as *const *mut pyo3::ffi::PyObject);
                            let cand_v = *(ma_keys.add(off + 8) as *const *mut pyo3::ffi::PyObject);
                            if cand_k == k0 && cand_v == first_val0_ptr {
                                if field_count == 1 {
                                    dk_header_word = ((*(ma_keys.add(8) as *const u32)
                                        & 0x00FF_FFFF)
                                        | 0x8000_0000)
                                        as u64;
                                    entries_offset = off;
                                    entry_stride = 16;
                                } else {
                                    let k1 = active_fields[1].key_ptr;
                                    let cand_k1_16 =
                                        *(ma_keys.add(off + 16) as *const *mut pyo3::ffi::PyObject);
                                    let cand_v1_16 =
                                        *(ma_keys.add(off + 24) as *const *mut pyo3::ffi::PyObject);
                                    if cand_k1_16 == k1 && cand_v1_16 == first_val1_ptr {
                                        dk_header_word = ((*(ma_keys.add(8) as *const u32)
                                            & 0x00FF_FFFF)
                                            | 0x8000_0000)
                                            as u64;
                                        entries_offset = off;
                                        entry_stride = 16;
                                    } else {
                                        let cand_k1_24 = *(ma_keys.add(off + 24)
                                            as *const *mut pyo3::ffi::PyObject);
                                        let cand_v1_24 = *(ma_keys.add(off + 32)
                                            as *const *mut pyo3::ffi::PyObject);
                                        if cand_k1_24 == k1 && cand_v1_24 == first_val1_ptr {
                                            dk_header_word = ((*(ma_keys.add(8) as *const u32)
                                                & 0x00FF_FFFF)
                                                | 0x8000_0000)
                                                as u64;
                                            entries_offset = off;
                                            entry_stride = 24;
                                        }
                                    }
                                }
                                break;
                            }
                            off += 8;
                        }
                    }
                }
            }
        }

        if needs_first_decref {
            unsafe { pyo3::ffi::Py_DECREF(first_dict_ptr) };
        }

        let st_len = self.string_table_bytes.len();
        let needed_initial = 512 + 11 + st_len + len * (field_count * 28 + 16);
        if self.work_buffer.capacity() < needed_initial {
            self.work_buffer.reserve(needed_initial);
        }

        let len_bytes = (len as u32).to_le_bytes();
        unsafe {
            let base = self.work_buffer.as_mut_ptr();
            ptr::write_unaligned(base as *mut u16, u16::from_le_bytes(*b"BF"));
            *base.add(2) = if compress { 0x01 } else { 0x00 };
            *base.add(3) = 0x01;
            ptr::write_unaligned(base.add(4) as *mut u16, (self.next_id as u16).to_le());
            if st_len > 0 {
                Self::copy_bytes_short(base.add(6), self.string_table_bytes.as_ptr(), st_len);
            }
            let list_hdr_ptr = base.add(6 + st_len);
            *list_hdr_ptr = 0x60;
            ptr::write_unaligned(
                list_hdr_ptr.add(1) as *mut u32,
                u32::from_ne_bytes(len_bytes),
            );
            self.work_buffer.set_len(11 + st_len);
        }

        let py = list.py();
        let fast_abi = self.fast_cpython_abi;
        let list_items_ptr = if fast_abi {
            unsafe { *(list_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject) }
        } else {
            ptr::null()
        };

        let mut start_idx = 0usize;
        if dk_header_word != 0 && !list_items_ptr.is_null() && self.long_tag_one != usize::MAX {
            unsafe {
                let base = self.work_buffer.as_mut_ptr();
                let mut dst = base.add(11 + st_len);
                let end = base.add(self.work_buffer.capacity());
                let fields_ptr = active_fields.as_ptr();
                let ascii_off = self.ascii_str_offset;
                let lt0 = self.long_tag_zero;
                let lt1 = self.long_tag_one;
                let py_none_ptr = self.py_none.as_ptr();
                let py_true_ptr = self.py_true.as_ptr();
                let py_false_ptr = self.py_false.as_ptr();

                while start_idx < len {
                    let item_ptr = *list_items_ptr.add(start_idx);
                    if item_ptr.is_null() {
                        break;
                    }
                    let next_dst = Self::serialize_item_makeys_raw(
                        dst,
                        end,
                        item_ptr,
                        fields_ptr,
                        field_count,
                        is_pydantic,
                        first_type_ptr,
                        obj_dict_offset,
                        dk_header_word,
                        entries_offset,
                        entry_stride,
                        ascii_off,
                        lt0,
                        lt1,
                        py_none_ptr,
                        py_true_ptr,
                        py_false_ptr,
                    );
                    if next_dst.is_null() {
                        break;
                    }
                    dst = next_dst;
                    start_idx += 1;
                }
                self.work_buffer
                    .set_len((dst as usize).wrapping_sub(base as usize));
            }
        }

        for i in start_idx..len {
            let item_ptr = unsafe {
                if !list_items_ptr.is_null() {
                    *list_items_ptr.add(i)
                } else {
                    pyo3::ffi::PyList_GetItem(list_ptr, i as pyo3::ffi::Py_ssize_t)
                }
            };
            if !self.serialize_pydantic_fast_ptr_with_layout(
                item_ptr,
                py,
                active_fields,
                dict_attr_ptr,
                is_pydantic,
                first_type_ptr,
                obj_dict_offset,
                dk_header_word,
                entries_offset,
                entry_stride,
            ) {
                self.decrease_recursion_depth();
                return false;
            }
        }

        self.decrease_recursion_depth();
        true
    }

    #[inline(always)]
    fn encode_pydantic_batch_direct_pybytes(
        &mut self,
        list: &Bound<'_, PyList>,
    ) -> Option<PyObject> {
        if !self.fast_cpython_abi || self.long_tag_one == usize::MAX {
            return None;
        }
        let list_ptr = list.as_ptr();
        let len = unsafe { *(list_ptr.byte_add(16) as *const isize) as usize };
        if len == 0 {
            return None;
        }

        let list_items_ptr =
            unsafe { *(list_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject) };
        if list_items_ptr.is_null() {
            return None;
        }
        let first_ptr = unsafe { *list_items_ptr };
        if first_ptr.is_null() {
            return None;
        }

        let mut first_type_ptr: *mut pyo3::ffi::PyTypeObject = ptr::null_mut();
        let mut obj_dict_offset: isize = 0;

        let (is_pydantic, first_dict_ptr) = unsafe {
            let dict_ptr_ptr = _PyObject_GetDictPtr(first_ptr);
            if !dict_ptr_ptr.is_null() && !(*dict_ptr_ptr).is_null() {
                let d = *dict_ptr_ptr;
                if (*d).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyDict_Type) {
                    first_type_ptr = (*first_ptr).ob_type;
                    obj_dict_offset = (dict_ptr_ptr as isize) - (first_ptr as isize);
                    (true, d)
                } else {
                    return None;
                }
            } else if (*first_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyDict_Type) {
                (false, first_ptr)
            } else {
                return None;
            }
        };

        let field_count = unsafe { *(first_dict_ptr.byte_add(16) as *const usize) };
        if field_count == 0
            || field_count > 32
            || unsafe { *(first_dict_ptr.byte_add(40) as *const usize) != 0 }
        {
            return None;
        }
        let ma_keys = unsafe { *(first_dict_ptr.byte_add(32) as *const *const u8) };
        if ma_keys.is_null() || unsafe { *(ma_keys.add(24) as *const usize) != field_count } {
            return None;
        }

        let mut fields_uninit = [std::mem::MaybeUninit::<FastFieldInfo>::uninit(); 32];
        let mut f_idx = 0usize;
        let mut first_val0_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut first_val1_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();

        let mut pos: pyo3::ffi::Py_ssize_t = 0;
        let mut key_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut val_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let ascii_off = self.ascii_str_offset;

        while unsafe {
            pyo3::ffi::PyDict_Next(first_dict_ptr, &mut pos, &mut key_ptr, &mut val_ptr)
        } != 0
        {
            let kind = self.detect_fast_field_kind(val_ptr);
            if kind == FastFieldKind::Generic {
                return None;
            }
            let (utf8_ptr, str_len) = unsafe {
                if (*key_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyUnicode_Type)
                    && (*key_ptr.byte_add(32).cast::<u8>() & 0x7C) == 0x64
                {
                    let sz = *(key_ptr.byte_add(16) as *const isize);
                    if sz >= 0 {
                        (key_ptr.byte_add(ascii_off) as *const u8, sz as usize)
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            };
            let key_bytes = unsafe { std::slice::from_raw_parts(utf8_ptr, str_len) };
            let name = unsafe { std::str::from_utf8_unchecked(key_bytes) };
            let id = self.get_or_create_string_id_fast(name);

            if f_idx == 0 {
                first_val0_ptr = val_ptr;
            } else if f_idx == 1 {
                first_val1_ptr = val_ptr;
            }

            fields_uninit[f_idx].write(FastFieldInfo {
                key_ptr,
                id_bytes: id.to_le_bytes(),
                kind,
            });
            f_idx += 1;
        }

        if f_idx != field_count {
            return None;
        }

        let active_fields = unsafe {
            std::slice::from_raw_parts(fields_uninit.as_ptr() as *const FastFieldInfo, field_count)
        };

        let mut dk_header_word = 0u64;
        let mut entries_offset = 0usize;
        let mut entry_stride = 0usize;

        unsafe {
            let k0 = active_fields[0].key_ptr;
            let mut off = 32usize;
            while off <= 88 {
                let cand_k = *(ma_keys.add(off) as *const *mut pyo3::ffi::PyObject);
                let cand_v = *(ma_keys.add(off + 8) as *const *mut pyo3::ffi::PyObject);
                if cand_k == k0 && cand_v == first_val0_ptr {
                    if field_count == 1 {
                        dk_header_word =
                            ((*(ma_keys.add(8) as *const u32) & 0x00FF_FFFF) | 0x8000_0000) as u64;
                        entries_offset = off;
                        entry_stride = 16;
                    } else {
                        let k1 = active_fields[1].key_ptr;
                        let cand_k1_16 =
                            *(ma_keys.add(off + 16) as *const *mut pyo3::ffi::PyObject);
                        let cand_v1_16 =
                            *(ma_keys.add(off + 24) as *const *mut pyo3::ffi::PyObject);
                        if cand_k1_16 == k1 && cand_v1_16 == first_val1_ptr {
                            dk_header_word = ((*(ma_keys.add(8) as *const u32) & 0x00FF_FFFF)
                                | 0x8000_0000) as u64;
                            entries_offset = off;
                            entry_stride = 16;
                        } else {
                            let cand_k1_24 =
                                *(ma_keys.add(off + 24) as *const *mut pyo3::ffi::PyObject);
                            let cand_v1_24 =
                                *(ma_keys.add(off + 32) as *const *mut pyo3::ffi::PyObject);
                            if cand_k1_24 == k1 && cand_v1_24 == first_val1_ptr {
                                dk_header_word = ((*(ma_keys.add(8) as *const u32) & 0x00FF_FFFF)
                                    | 0x8000_0000)
                                    as u64;
                                entries_offset = off;
                                entry_stride = 24;
                            }
                        }
                    }
                    break;
                }
                off += 8;
            }
        }

        if dk_header_word == 0 {
            return None;
        }

        let st_len = self.string_table_bytes.len();
        let cap = 512 + 11 + st_len + len * (field_count * 26 + 16);
        let py = list.py();

        unsafe {
            let pybytes_ptr =
                pyo3::ffi::PyBytes_FromStringAndSize(ptr::null(), cap as pyo3::ffi::Py_ssize_t);
            if pybytes_ptr.is_null() {
                pyo3::ffi::PyErr_Clear();
                return None;
            }
            let base = pyo3::ffi::PyBytes_AsString(pybytes_ptr) as *mut u8;
            let end = base.add(cap);

            ptr::write_unaligned(base as *mut u16, u16::from_le_bytes(*b"BF"));
            *base.add(2) = 0x00;
            *base.add(3) = 0x01;
            ptr::write_unaligned(base.add(4) as *mut u16, (self.next_id as u16).to_le());
            if st_len > 0 {
                Self::copy_bytes_short(base.add(6), self.string_table_bytes.as_ptr(), st_len);
            }
            let list_hdr_ptr = base.add(6 + st_len);
            *list_hdr_ptr = 0x60;
            ptr::write_unaligned(list_hdr_ptr.add(1) as *mut u32, (len as u32).to_le());

            let mut dst = base.add(11 + st_len);
            let fields_ptr = active_fields.as_ptr();
            let lt0 = self.long_tag_zero;
            let lt1 = self.long_tag_one;
            let py_none_ptr = self.py_none.as_ptr();
            let py_true_ptr = self.py_true.as_ptr();
            let py_false_ptr = self.py_false.as_ptr();

            for i in 0..len {
                let item_ptr = *list_items_ptr.add(i);
                if item_ptr.is_null() {
                    pyo3::ffi::Py_DECREF(pybytes_ptr);
                    return None;
                }
                let next_dst = Self::serialize_item_makeys_raw(
                    dst,
                    end,
                    item_ptr,
                    fields_ptr,
                    field_count,
                    is_pydantic,
                    first_type_ptr,
                    obj_dict_offset,
                    dk_header_word,
                    entries_offset,
                    entry_stride,
                    ascii_off,
                    lt0,
                    lt1,
                    py_none_ptr,
                    py_true_ptr,
                    py_false_ptr,
                );
                if next_dst.is_null() {
                    pyo3::ffi::Py_DECREF(pybytes_ptr);
                    return None;
                }
                dst = next_dst;
            }

            let final_len = (dst as usize).wrapping_sub(base as usize);
            (*(pybytes_ptr as *mut pyo3::ffi::PyVarObject)).ob_size =
                final_len as pyo3::ffi::Py_ssize_t;
            *dst = 0;
            self.last_was_compressed = false;
            Some(PyObject::from_owned_ptr(py, pybytes_ptr))
        }
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    unsafe fn serialize_item_makeys_raw(
        mut dst: *mut u8,
        end: *mut u8,
        item_ptr: *mut pyo3::ffi::PyObject,
        fields_ptr: *const FastFieldInfo,
        field_count: usize,
        is_pydantic: bool,
        first_type_ptr: *mut pyo3::ffi::PyTypeObject,
        obj_dict_offset: isize,
        dk_header_word: u64,
        entries_offset: usize,
        entry_stride: usize,
        ascii_str_offset: usize,
        long_tag_zero: usize,
        long_tag_one: usize,
        py_none_ptr: *mut pyo3::ffi::PyObject,
        py_true_ptr: *mut pyo3::ffi::PyObject,
        py_false_ptr: *mut pyo3::ffi::PyObject,
    ) -> *mut u8 {
        let dict_ptr = if is_pydantic {
            if (*item_ptr).ob_type != first_type_ptr {
                return ptr::null_mut();
            }
            let d = *(item_ptr.byte_offset(obj_dict_offset) as *const *mut pyo3::ffi::PyObject);
            if d.is_null() || (*d).ob_type != ptr::addr_of_mut!(pyo3::ffi::PyDict_Type) {
                return ptr::null_mut();
            }
            d
        } else {
            if (*item_ptr).ob_type != ptr::addr_of_mut!(pyo3::ffi::PyDict_Type) {
                return ptr::null_mut();
            }
            item_ptr
        };

        if *(dict_ptr.byte_add(16) as *const usize) != field_count
            || *(dict_ptr.byte_add(40) as *const usize) != 0
        {
            return ptr::null_mut();
        }
        let ma_keys = *(dict_ptr.byte_add(32) as *const *const u8);
        if ma_keys.is_null()
            || (((*(ma_keys.add(8) as *const u32) & 0x00FF_FFFF) | 0x8000_0000) as u64)
                != dk_header_word
            || *(ma_keys.add(24) as *const usize) != field_count
        {
            return ptr::null_mut();
        }

        if (end as usize).wrapping_sub(dst as usize) < 512 {
            return ptr::null_mut();
        }

        *dst = 0x70;
        dst = dst.add(1);

        let mut entry_ptr = ma_keys.add(entries_offset);
        let long_type = ptr::addr_of_mut!(pyo3::ffi::PyLong_Type);
        let float_type = ptr::addr_of_mut!(pyo3::ffi::PyFloat_Type);
        let unicode_type = ptr::addr_of_mut!(pyo3::ffi::PyUnicode_Type);
        let list_type = ptr::addr_of_mut!(pyo3::ffi::PyList_Type);

        for f_idx in 0..field_count {
            let field = &*fields_ptr.add(f_idx);
            let k = *(entry_ptr as *const *mut pyo3::ffi::PyObject);
            if k != field.key_ptr {
                return ptr::null_mut();
            }
            let val_ptr = *(entry_ptr.add(8) as *const *mut pyo3::ffi::PyObject);
            if val_ptr.is_null() {
                return ptr::null_mut();
            }
            entry_ptr = entry_ptr.add(entry_stride);

            let id_u32 = u32::from_ne_bytes(field.id_bytes);
            ptr::write_unaligned(dst as *mut u32, id_u32);

            if val_ptr == py_none_ptr {
                *dst.add(4) = 0x10;
                dst = dst.add(5);
                continue;
            }

            match field.kind {
                FastFieldKind::Int => {
                    if val_ptr == py_true_ptr
                        || val_ptr == py_false_ptr
                        || (*val_ptr).ob_type != long_type
                    {
                        return ptr::null_mut();
                    }
                    let tag = *(val_ptr.byte_add(16) as *const usize);
                    if tag == long_tag_one {
                        let d = *(val_ptr.byte_add(24) as *const u32);
                        if d < 8 {
                            *dst.add(4) = 0x30 | (d as u8);
                            dst = dst.add(5);
                        } else {
                            *dst.add(4) = 0x38;
                            ptr::write_unaligned(dst.add(5) as *mut u64, (d as u64).to_le());
                            dst = dst.add(13);
                        }
                    } else if tag == long_tag_zero {
                        *dst.add(4) = 0x30;
                        dst = dst.add(5);
                    } else {
                        return ptr::null_mut();
                    }
                }
                FastFieldKind::Float => {
                    if (*val_ptr).ob_type != float_type {
                        return ptr::null_mut();
                    }
                    let bits = (*(val_ptr.byte_add(16) as *const u64)).to_le();
                    *dst.add(4) = 0x40;
                    ptr::write_unaligned(dst.add(5) as *mut u64, bits);
                    dst = dst.add(13);
                }
                FastFieldKind::Bool => {
                    if val_ptr == py_true_ptr {
                        *dst.add(4) = 0x21;
                        dst = dst.add(5);
                    } else if val_ptr == py_false_ptr {
                        *dst.add(4) = 0x20;
                        dst = dst.add(5);
                    } else {
                        return ptr::null_mut();
                    }
                }
                FastFieldKind::String => {
                    if (*val_ptr).ob_type != unicode_type
                        || (*val_ptr.byte_add(32).cast::<u8>() & 0x7C) != 0x64
                    {
                        return ptr::null_mut();
                    }
                    let sz = *(val_ptr.byte_add(16) as *const isize);
                    if sz < 0 {
                        return ptr::null_mut();
                    }
                    let str_len = sz as usize;
                    if str_len > 64 && (end as usize).wrapping_sub(dst as usize) < 9 + str_len + 256
                    {
                        return ptr::null_mut();
                    }
                    *dst.add(4) = 0x50;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (str_len as u32).to_le());
                    let utf8_ptr = val_ptr.byte_add(ascii_str_offset) as *const u8;
                    Self::copy_bytes_short(dst.add(9), utf8_ptr, str_len);
                    dst = dst.add(9 + str_len);
                }
                FastFieldKind::FloatList => {
                    if (*val_ptr).ob_type != list_type {
                        return ptr::null_mut();
                    }
                    let sz = *(val_ptr.byte_add(16) as *const isize);
                    if sz < 0 {
                        return ptr::null_mut();
                    }
                    let list_len = sz as usize;
                    let needed = 9 + list_len * 9;
                    if list_len > 16 && (end as usize).wrapping_sub(dst as usize) < needed + 256 {
                        return ptr::null_mut();
                    }
                    *dst.add(4) = 0x60;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                    let items_ptr =
                        *(val_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject);
                    let mut cur = dst.add(9);
                    for j in 0..list_len {
                        let elem_ptr = *items_ptr.add(j);
                        if (*elem_ptr).ob_type != float_type {
                            return ptr::null_mut();
                        }
                        let bits = (*(elem_ptr.byte_add(16) as *const u64)).to_le();
                        *cur = 0x40;
                        ptr::write_unaligned(cur.add(1) as *mut u64, bits);
                        cur = cur.add(9);
                    }
                    dst = cur;
                }
                FastFieldKind::IntList => {
                    if (*val_ptr).ob_type != list_type {
                        return ptr::null_mut();
                    }
                    let sz = *(val_ptr.byte_add(16) as *const isize);
                    if sz < 0 {
                        return ptr::null_mut();
                    }
                    let list_len = sz as usize;
                    if list_len > 16
                        && (end as usize).wrapping_sub(dst as usize) < 9 + list_len * 9 + 256
                    {
                        return ptr::null_mut();
                    }
                    *dst.add(4) = 0x60;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                    let items_ptr =
                        *(val_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject);
                    let mut cur = dst.add(9);
                    for j in 0..list_len {
                        let elem_ptr = *items_ptr.add(j);
                        if elem_ptr == py_true_ptr
                            || elem_ptr == py_false_ptr
                            || (*elem_ptr).ob_type != long_type
                        {
                            return ptr::null_mut();
                        }
                        let tag = *(elem_ptr.byte_add(16) as *const usize);
                        if tag == long_tag_one {
                            let d = *(elem_ptr.byte_add(24) as *const u32);
                            if d < 8 {
                                *cur = 0x30 | (d as u8);
                                cur = cur.add(1);
                            } else {
                                *cur = 0x38;
                                ptr::write_unaligned(cur.add(1) as *mut u64, (d as u64).to_le());
                                cur = cur.add(9);
                            }
                        } else if tag == long_tag_zero {
                            *cur = 0x30;
                            cur = cur.add(1);
                        } else {
                            return ptr::null_mut();
                        }
                    }
                    dst = cur;
                }
                FastFieldKind::Generic => return ptr::null_mut(),
            }
        }
        *dst = 0x7F;
        dst.add(1)
    }

    #[inline(never)]
    fn serialize_pydantic_simd_batch(&mut self, list: &Bound<'_, PyList>) -> PyResult<()> {
        let len = list.len();
        if len == 0 {
            self.work_buffer.push(0x60);
            self.work_buffer.extend_from_slice(&0u32.to_le_bytes());
            return Ok(());
        }

        self.check_recursion_depth()?;

        let first_item = list.get_item(0)?;
        let first_ptr = first_item.as_ptr();
        let dict_attr_ptr = self.dict_attr.as_ptr();

        let (is_pydantic, dict) = {
            let dict_ptr_ptr = unsafe { _PyObject_GetDictPtr(first_ptr) };
            if !dict_ptr_ptr.is_null() && unsafe { !(*dict_ptr_ptr).is_null() } {
                let bound = unsafe { Bound::from_borrowed_ptr(list.py(), *dict_ptr_ptr) };
                if let Ok(d) = bound.downcast_into::<PyDict>() {
                    (true, d)
                } else {
                    self.decrease_recursion_depth();
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Invalid dict",
                    ));
                }
            } else {
                let p = unsafe { pyo3::ffi::PyObject_GetAttr(first_ptr, dict_attr_ptr) };
                if !p.is_null() {
                    let bound = unsafe { Bound::from_owned_ptr(list.py(), p) };
                    if let Ok(d) = bound.downcast_into::<PyDict>() {
                        (true, d)
                    } else {
                        self.decrease_recursion_depth();
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Invalid dict",
                        ));
                    }
                } else {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    if let Ok(d) = first_item.downcast::<PyDict>() {
                        (false, d.clone())
                    } else {
                        self.decrease_recursion_depth();
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Not Pydantic or Dict",
                        ));
                    }
                }
            }
        };

        let dict_len = dict.len();
        if dict_len > 32 {
            self.decrease_recursion_depth();
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Too many fields",
            ));
        }

        let mut fields = [FastFieldInfo {
            key_ptr: ptr::null_mut(),
            id_bytes: [0; 4],
            kind: FastFieldKind::Generic,
        }; 32];
        let mut field_count = 0;
        let mut use_fast_mode = true;

        for (k, val) in dict.iter() {
            let name = if let Ok(s) = k.downcast::<PyString>() {
                s.to_str().unwrap_or("")
            } else {
                ""
            };
            let id = self.get_or_create_string_id_fast(name);

            let kind = self.detect_fast_field_kind(val.as_ptr());

            if matches!(kind, FastFieldKind::Generic) && !val.is_none() {
                if let Ok(name_obj) = val.get_type().name() {
                    let type_name = name_obj.to_str().unwrap_or("");
                    if matches!(type_name, "datetime" | "date" | "time" | "UUID" | "Decimal") {
                        use_fast_mode = false;
                    }
                }
            }

            fields[field_count] = FastFieldInfo {
                key_ptr: k.as_ptr(),
                id_bytes: id.to_le_bytes(),
                kind,
            };
            field_count += 1;
        }

        let active_fields = &fields[..field_count];

        self.ensure_buffer_capacity(5 + len * (field_count * 32 + 32));

        // Write list opcode and length
        let len_bytes = (len as u32).to_le_bytes();
        let list_hdr: [u8; 5] = [0x60, len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]];
        self.work_buffer.extend_from_slice(&list_hdr);

        let list_ptr = list.as_ptr();
        if use_fast_mode {
            for i in 0..len {
                let item_ptr =
                    unsafe { pyo3::ffi::PyList_GetItem(list_ptr, i as pyo3::ffi::Py_ssize_t) };
                if !self.serialize_pydantic_fast_ptr_with_layout(
                    item_ptr,
                    list.py(),
                    active_fields,
                    dict_attr_ptr,
                    is_pydantic,
                    ptr::null_mut(),
                    0,
                    0,
                    0,
                    0,
                ) {
                    self.decrease_recursion_depth();
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Batch item serialization failed",
                    ));
                }
            }
        } else {
            let field_keys: Vec<Bound<'_, PyAny>> = dict.iter().map(|(k, _)| k).collect();
            let field_ids: Vec<u32> = active_fields
                .iter()
                .map(|f| u32::from_le_bytes(f.id_bytes))
                .collect();
            for item in list.iter() {
                self.serialize_pydantic_complex(&item, &field_keys, &field_ids, dict_attr_ptr)?;
            }
        }

        self.decrease_recursion_depth();
        Ok(())
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn serialize_pydantic_fast_ptr_with_layout(
        &mut self,
        obj_ptr: *mut pyo3::ffi::PyObject,
        py: Python<'_>,
        fields: &[FastFieldInfo],
        dict_attr_ptr: *mut pyo3::ffi::PyObject,
        is_pydantic: bool,
        first_type_ptr: *mut pyo3::ffi::PyTypeObject,
        obj_dict_offset: isize,
        dk_header_word: u64,
        entries_offset: usize,
        entry_stride: usize,
    ) -> bool {
        let item_start = self.work_buffer.len();
        if self.work_buffer.capacity() - item_start < 64 {
            self.work_buffer.reserve(4096);
        }
        unsafe {
            *self.work_buffer.as_mut_ptr().add(item_start) = 0x70;
            self.work_buffer.set_len(item_start + 1);
        }

        let mut needs_decref = false;
        let dict_ptr = if is_pydantic {
            let fast_d =
                if !first_type_ptr.is_null() && unsafe { (*obj_ptr).ob_type == first_type_ptr } {
                    unsafe {
                        *(obj_ptr.byte_offset(obj_dict_offset) as *const *mut pyo3::ffi::PyObject)
                    }
                } else {
                    ptr::null_mut()
                };
            if !fast_d.is_null() {
                fast_d
            } else {
                let dict_ptr_ptr = unsafe { _PyObject_GetDictPtr(obj_ptr) };
                if !dict_ptr_ptr.is_null() && unsafe { !(*dict_ptr_ptr).is_null() } {
                    unsafe { *dict_ptr_ptr }
                } else {
                    let p = unsafe { pyo3::ffi::PyObject_GetAttr(obj_ptr, dict_attr_ptr) };
                    if p.is_null() {
                        unsafe {
                            pyo3::ffi::PyErr_Clear();
                            *self.work_buffer.as_mut_ptr().add(item_start + 1) = 0x7F;
                            self.work_buffer.set_len(item_start + 2);
                        }
                        return true;
                    }
                    needs_decref = true;
                    p
                }
            }
        } else {
            obj_ptr
        };

        let num_fields = fields.len();
        let start_pos = item_start + 1;

        let py_none_ptr = self.py_none.as_ptr();
        let py_true_ptr = self.py_true.as_ptr();
        let py_false_ptr = self.py_false.as_ptr();
        let fast_abi = self.fast_cpython_abi;
        let ascii_str_offset = self.ascii_str_offset;
        let long_tag_zero = self.long_tag_zero;
        let long_tag_one = self.long_tag_one;

        // Ultra-fast path: Direct CPython combined-dict entry table walk (zero PyDict_Next FFI calls!)
        if entry_stride != 0 {
            let direct_ok = unsafe {
                if (*dict_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyDict_Type)
                    && *(dict_ptr.byte_add(16) as *const usize) == num_fields
                    && *(dict_ptr.byte_add(40) as *const usize) == 0
                {
                    let ma_keys = *(dict_ptr.byte_add(32) as *const *const u8);
                    if !ma_keys.is_null()
                        && (((*(ma_keys.add(8) as *const u32) & 0x00FF_FFFF) | 0x8000_0000) as u64)
                            == dk_header_word
                        && *(ma_keys.add(24) as *const usize) == num_fields
                    {
                        let mut entry_ptr = ma_keys.add(entries_offset);
                        let mut ok = true;
                        for f_info in fields {
                            let k_ptr = *(entry_ptr as *const *mut pyo3::ffi::PyObject);
                            let v_ptr = *(entry_ptr.add(8) as *const *mut pyo3::ffi::PyObject);
                            if k_ptr != f_info.key_ptr
                                || !Self::write_val_ptr_to_vec(
                                    &mut self.work_buffer,
                                    py_none_ptr,
                                    py_true_ptr,
                                    py_false_ptr,
                                    fast_abi,
                                    ascii_str_offset,
                                    long_tag_zero,
                                    long_tag_one,
                                    u32::from_ne_bytes(f_info.id_bytes),
                                    v_ptr,
                                    f_info.kind,
                                )
                            {
                                ok = false;
                                break;
                            }
                            entry_ptr = entry_ptr.add(entry_stride);
                        }
                        ok
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if direct_ok {
                if needs_decref {
                    unsafe {
                        pyo3::ffi::Py_DECREF(dict_ptr);
                    }
                }
                let cur_len = self.work_buffer.len();
                if self.work_buffer.capacity() - cur_len < 1 {
                    self.work_buffer.reserve(4096);
                }
                unsafe {
                    *self.work_buffer.as_mut_ptr().add(cur_len) = 0x7F;
                    self.work_buffer.set_len(cur_len + 1);
                }
                return true;
            }
            self.work_buffer.truncate(start_pos);
        }

        let mut pos: pyo3::ffi::Py_ssize_t = 0;
        let mut key_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut val_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut matched_in_order = true;
        let mut field_idx = 0;

        // Fast-path: Walk dense dictionary entries via PyDict_Next (zero hash lookups!)
        while unsafe { pyo3::ffi::PyDict_Next(dict_ptr, &mut pos, &mut key_ptr, &mut val_ptr) } != 0
        {
            if field_idx < num_fields && key_ptr == fields[field_idx].key_ptr {
                let f_info = &fields[field_idx];
                let id_u32 = u32::from_ne_bytes(f_info.id_bytes);
                if !Self::write_val_ptr_to_vec(
                    &mut self.work_buffer,
                    py_none_ptr,
                    py_true_ptr,
                    py_false_ptr,
                    fast_abi,
                    ascii_str_offset,
                    long_tag_zero,
                    long_tag_one,
                    id_u32,
                    val_ptr,
                    f_info.kind,
                ) {
                    matched_in_order = false;
                    break;
                }
                field_idx += 1;
            } else {
                matched_in_order = false;
                break;
            }
        }

        if matched_in_order && field_idx == num_fields {
            if needs_decref {
                unsafe {
                    pyo3::ffi::Py_DECREF(dict_ptr);
                }
            }
            let cur_len = self.work_buffer.len();
            if self.work_buffer.capacity() - cur_len < 1 {
                self.work_buffer.reserve(4096);
            }
            unsafe {
                *self.work_buffer.as_mut_ptr().add(cur_len) = 0x7F;
                self.work_buffer.set_len(cur_len + 1);
            }
            return true;
        }

        self.serialize_pydantic_fallback_ptr(dict_ptr, needs_decref, start_pos, fields, py)
    }

    #[inline(never)]
    fn serialize_pydantic_fallback_ptr(
        &mut self,
        dict_ptr: *mut pyo3::ffi::PyObject,
        needs_decref: bool,
        start_pos: usize,
        fields: &[FastFieldInfo],
        py: Python<'_>,
    ) -> bool {
        // Fallback: If dictionary keys were out of order, missing, or had dynamic types
        self.work_buffer.truncate(start_pos);
        for f in fields {
            self.work_buffer.extend_from_slice(&f.id_bytes);

            let item_ptr = unsafe { pyo3::ffi::PyDict_GetItemWithError(dict_ptr, f.key_ptr) };
            if !item_ptr.is_null() {
                let val = unsafe { Bound::from_borrowed_ptr(py, item_ptr) };
                if self.serialize_value_fast(&val).is_err() {
                    if needs_decref {
                        unsafe {
                            pyo3::ffi::Py_DECREF(dict_ptr);
                        }
                    }
                    return false;
                }
            } else {
                unsafe { pyo3::ffi::PyErr_Clear() };
                self.work_buffer.push(0x10);
            }
        }

        if needs_decref {
            unsafe {
                pyo3::ffi::Py_DECREF(dict_ptr);
            }
        }

        self.work_buffer.push(0x7F);
        true
    }

    pub(crate) fn prepare_homogeneous_template(
        &mut self,
        first_item: &Bound<'_, PyAny>,
    ) -> Option<HomogeneousTemplate> {
        let first_ptr = first_item.as_ptr();
        let dict_attr_ptr = self.dict_attr.as_ptr();

        let (is_pydantic, dict) = {
            let dict_ptr_ptr = unsafe { _PyObject_GetDictPtr(first_ptr) };
            if !dict_ptr_ptr.is_null() && unsafe { !(*dict_ptr_ptr).is_null() } {
                let bound = unsafe { Bound::from_borrowed_ptr(first_item.py(), *dict_ptr_ptr) };
                if let Ok(d) = bound.downcast_into::<PyDict>() {
                    (true, d)
                } else {
                    return None;
                }
            } else {
                let p = unsafe { pyo3::ffi::PyObject_GetAttr(first_ptr, dict_attr_ptr) };
                if !p.is_null() {
                    let bound = unsafe { Bound::from_owned_ptr(first_item.py(), p) };
                    if let Ok(d) = bound.downcast_into::<PyDict>() {
                        (true, d)
                    } else {
                        return None;
                    }
                } else {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    if let Ok(d) = first_item.downcast::<PyDict>() {
                        (false, d.clone())
                    } else {
                        return None;
                    }
                }
            }
        };

        let dict_len = dict.len();
        if dict_len == 0 || dict_len > 64 {
            return None;
        }

        let mut fields = Vec::with_capacity(dict_len);
        let mut prefix = Vec::with_capacity(6 + dict_len * 16);
        prefix.extend_from_slice(&[0u8; 6]); // placeholder for header

        let mut field_id = 0u32;
        for (k, val) in dict.iter() {
            let name = if let Ok(s) = k.downcast::<PyString>() {
                s.to_str().unwrap_or("")
            } else {
                return None;
            };
            let s_bytes = name.as_bytes();
            if s_bytes.len() > 255 {
                return None;
            }
            prefix.push(s_bytes.len() as u8);
            prefix.extend_from_slice(s_bytes);

            let kind = self.detect_fast_field_kind(val.as_ptr());
            if kind == FastFieldKind::Generic {
                return None;
            }

            fields.push(TemplateField {
                key: k.clone().unbind(),
                id_bytes: field_id.to_le_bytes(),
                kind,
            });
            field_id += 1;
        }

        prefix[0..2].copy_from_slice(b"BF");
        prefix[2] = 0x00; // flags (uncompressed)
        prefix[3] = 0x01; // version
        let count_bytes = (field_id as u16).to_le_bytes();
        prefix[4..6].copy_from_slice(&count_bytes);

        Some(HomogeneousTemplate {
            prefix,
            fields,
            is_pydantic,
        })
    }

    pub(crate) fn encode_homogeneous_frame(
        &self,
        item_ptr: *mut pyo3::ffi::PyObject,
        _py: Python<'_>,
        template: &HomogeneousTemplate,
        out: &mut Vec<u8>,
    ) -> bool {
        let frame_start = out.len();
        let prefix_len = template.prefix.len();
        let header_and_prefix = 7 + prefix_len;
        if out.capacity() - frame_start < header_and_prefix + 64 {
            out.reserve(header_and_prefix + 4096);
        }
        unsafe {
            let base = out.as_mut_ptr().add(frame_start);
            Self::copy_bytes_short(base.add(6), template.prefix.as_ptr(), prefix_len);
            *base.add(6 + prefix_len) = 0x70;
            out.set_len(frame_start + header_and_prefix);
        }

        let mut needs_decref = false;
        let dict_ptr = if template.is_pydantic {
            let dict_ptr_ptr = unsafe { _PyObject_GetDictPtr(item_ptr) };
            if !dict_ptr_ptr.is_null() && unsafe { !(*dict_ptr_ptr).is_null() } {
                unsafe { *dict_ptr_ptr }
            } else {
                let p = unsafe { pyo3::ffi::PyObject_GetAttr(item_ptr, self.dict_attr.as_ptr()) };
                if p.is_null() {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    out.truncate(frame_start);
                    return false;
                }
                needs_decref = true;
                p
            }
        } else if unsafe {
            pyo3::ffi::PyDict_CheckExact(item_ptr) != 0 || pyo3::ffi::PyDict_Check(item_ptr) != 0
        } {
            item_ptr
        } else {
            out.truncate(frame_start);
            return false;
        };

        let mut pos: pyo3::ffi::Py_ssize_t = 0;
        let mut key_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut val_ptr: *mut pyo3::ffi::PyObject = ptr::null_mut();
        let mut field_idx = 0;
        let num_fields = template.fields.len();

        let py_none_ptr = self.py_none.as_ptr();
        let py_true_ptr = self.py_true.as_ptr();
        let py_false_ptr = self.py_false.as_ptr();
        let fast_abi = self.fast_cpython_abi;
        let ascii_str_offset = self.ascii_str_offset;
        let long_tag_zero = self.long_tag_zero;
        let long_tag_one = self.long_tag_one;

        while unsafe { pyo3::ffi::PyDict_Next(dict_ptr, &mut pos, &mut key_ptr, &mut val_ptr) } != 0
        {
            if field_idx < num_fields {
                let target_field = &template.fields[field_idx];
                let target_ptr = target_field.key.as_ptr();
                let is_match = key_ptr == target_ptr
                    || unsafe {
                        let eq = pyo3::ffi::PyObject_RichCompareBool(
                            key_ptr,
                            target_ptr,
                            pyo3::ffi::Py_EQ,
                        );
                        if eq < 0 {
                            pyo3::ffi::PyErr_Clear();
                            false
                        } else {
                            eq == 1
                        }
                    };

                if is_match {
                    let id_u32 = u32::from_ne_bytes(target_field.id_bytes);
                    if !Self::write_val_ptr_to_vec(
                        out,
                        py_none_ptr,
                        py_true_ptr,
                        py_false_ptr,
                        fast_abi,
                        ascii_str_offset,
                        long_tag_zero,
                        long_tag_one,
                        id_u32,
                        val_ptr,
                        target_field.kind,
                    ) {
                        if needs_decref {
                            unsafe {
                                pyo3::ffi::Py_DECREF(dict_ptr);
                            }
                        }
                        out.truncate(frame_start);
                        return false;
                    }
                    field_idx += 1;
                } else {
                    if needs_decref {
                        unsafe {
                            pyo3::ffi::Py_DECREF(dict_ptr);
                        }
                    }
                    out.truncate(frame_start);
                    return false;
                }
            } else {
                if needs_decref {
                    unsafe {
                        pyo3::ffi::Py_DECREF(dict_ptr);
                    }
                }
                out.truncate(frame_start);
                return false;
            }
        }

        if needs_decref {
            unsafe {
                pyo3::ffi::Py_DECREF(dict_ptr);
            }
        }

        if field_idx != num_fields {
            out.truncate(frame_start);
            return false;
        }

        let cur_len = out.len();
        if out.capacity() - cur_len < 1 {
            out.reserve(4096);
        }
        unsafe {
            *out.as_mut_ptr().add(cur_len) = 0x7F;
            out.set_len(cur_len + 1);
            let payload_len = (cur_len + 1 - frame_start - 6) as u32;
            let hdr_ptr = out.as_mut_ptr().add(frame_start);
            ptr::write_unaligned(hdr_ptr as *mut u32, payload_len.to_le());
            *hdr_ptr.add(4) = crate::streaming::FRAME_DATA;
            *hdr_ptr.add(5) = 0x00;
        }

        true
    }

    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn write_val_ptr_to_vec(
        out: &mut Vec<u8>,
        py_none_ptr: *mut pyo3::ffi::PyObject,
        py_true_ptr: *mut pyo3::ffi::PyObject,
        py_false_ptr: *mut pyo3::ffi::PyObject,
        fast_abi: bool,
        ascii_str_offset: usize,
        long_tag_zero: usize,
        long_tag_one: usize,
        id_u32: u32,
        val_ptr: *mut pyo3::ffi::PyObject,
        expected_kind: FastFieldKind,
    ) -> bool {
        let cur_len = out.len();
        if val_ptr == py_none_ptr {
            if out.capacity() - cur_len < 5 {
                out.reserve(4096);
            }
            unsafe {
                let dst = out.as_mut_ptr().add(cur_len);
                ptr::write_unaligned(dst as *mut u32, id_u32);
                *dst.add(4) = 0x10;
                out.set_len(cur_len + 5);
            }
            return true;
        }

        match expected_kind {
            FastFieldKind::Bool => {
                let b_byte = if val_ptr == py_true_ptr {
                    0x21
                } else if val_ptr == py_false_ptr {
                    0x20
                } else {
                    return false;
                };
                if out.capacity() - cur_len < 5 {
                    out.reserve(4096);
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    *dst.add(4) = b_byte;
                    out.set_len(cur_len + 5);
                }
                true
            }
            FastFieldKind::Int => {
                if val_ptr == py_true_ptr || val_ptr == py_false_ptr {
                    return false;
                }
                let n = unsafe {
                    if (*val_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyLong_Type) {
                        let tag = *(val_ptr.byte_add(16) as *const usize);
                        if tag == long_tag_one {
                            *(val_ptr.byte_add(24) as *const u32) as i64
                        } else if tag == long_tag_zero {
                            0
                        } else {
                            let mut overflow: std::os::raw::c_int = 0;
                            let v = pyo3::ffi::PyLong_AsLongLongAndOverflow(val_ptr, &mut overflow);
                            if overflow != 0 || (v == -1 && !pyo3::ffi::PyErr_Occurred().is_null())
                            {
                                pyo3::ffi::PyErr_Clear();
                                return false;
                            }
                            v
                        }
                    } else {
                        let mut overflow: std::os::raw::c_int = 0;
                        let v = pyo3::ffi::PyLong_AsLongLongAndOverflow(val_ptr, &mut overflow);
                        if overflow != 0 || (v == -1 && !pyo3::ffi::PyErr_Occurred().is_null()) {
                            pyo3::ffi::PyErr_Clear();
                            return false;
                        }
                        v
                    }
                };
                if out.capacity() - cur_len < 13 {
                    out.reserve(4096);
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    if (n as u64) < 8 {
                        *dst.add(4) = 0x30 | (n as u8);
                        out.set_len(cur_len + 5);
                    } else {
                        *dst.add(4) = 0x38;
                        ptr::write_unaligned(dst.add(5) as *mut i64, n.to_le());
                        out.set_len(cur_len + 13);
                    }
                }
                true
            }
            FastFieldKind::Float => {
                let bits = unsafe {
                    if fast_abi && (*val_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyFloat_Type)
                    {
                        (*(val_ptr.byte_add(16) as *const u64)).to_le()
                    } else {
                        let f = pyo3::ffi::PyFloat_AsDouble(val_ptr);
                        if f == -1.0 && !pyo3::ffi::PyErr_Occurred().is_null() {
                            pyo3::ffi::PyErr_Clear();
                            return false;
                        }
                        f.to_bits().to_le()
                    }
                };
                if out.capacity() - cur_len < 13 {
                    out.reserve(4096);
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    *dst.add(4) = 0x40;
                    ptr::write_unaligned(dst.add(5) as *mut u64, bits);
                    out.set_len(cur_len + 13);
                }
                true
            }
            FastFieldKind::String => {
                let (utf8_ptr, str_len) = unsafe {
                    if ascii_str_offset != 0
                        && (*val_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyUnicode_Type)
                        && (*val_ptr.byte_add(32).cast::<u8>() & 0x7C) == 0x64
                    {
                        let sz = *(val_ptr.byte_add(16) as *const isize);
                        if sz >= 0 {
                            (val_ptr.byte_add(ascii_str_offset) as *const u8, sz as usize)
                        } else {
                            return false;
                        }
                    } else {
                        let mut size: pyo3::ffi::Py_ssize_t = 0;
                        let p = pyo3::ffi::PyUnicode_AsUTF8AndSize(val_ptr, &mut size);
                        if p.is_null() {
                            pyo3::ffi::PyErr_Clear();
                            return false;
                        }
                        (p as *const u8, size as usize)
                    }
                };
                let needed = 9 + str_len;
                if out.capacity() - cur_len < needed {
                    out.reserve(needed.max(4096));
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    *dst.add(4) = 0x50;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (str_len as u32).to_le());
                    Self::copy_bytes_short(dst.add(9), utf8_ptr, str_len);
                    out.set_len(cur_len + needed);
                }
                true
            }
            FastFieldKind::FloatList => {
                unsafe {
                    if fast_abi && (*val_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyList_Type) {
                        let len_py = *(val_ptr.byte_add(16) as *const isize);
                        if len_py < 0 {
                            return false;
                        }
                        let list_len = len_py as usize;
                        let needed = 9 + list_len * 9;
                        if out.capacity() - cur_len < needed {
                            out.reserve(needed.max(4096));
                        }
                        let dst = out.as_mut_ptr().add(cur_len);
                        ptr::write_unaligned(dst as *mut u32, id_u32);
                        *dst.add(4) = 0x60;
                        ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                        let items_ptr =
                            *(val_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject);
                        let float_type = ptr::addr_of_mut!(pyo3::ffi::PyFloat_Type);
                        let mut cur = dst.add(9);
                        for j in 0..list_len {
                            let elem_ptr = *items_ptr.add(j);
                            let bits = if (*elem_ptr).ob_type == float_type {
                                (*(elem_ptr.byte_add(16) as *const u64)).to_le()
                            } else {
                                let f = pyo3::ffi::PyFloat_AsDouble(elem_ptr);
                                if f == -1.0 && !pyo3::ffi::PyErr_Occurred().is_null() {
                                    pyo3::ffi::PyErr_Clear();
                                    return false;
                                }
                                f.to_bits().to_le()
                            };
                            *cur = 0x40;
                            ptr::write_unaligned(cur.add(1) as *mut u64, bits);
                            cur = cur.add(9);
                        }
                        out.set_len(cur_len + needed);
                        return true;
                    }
                }
                let len_py = unsafe { pyo3::ffi::PyList_Size(val_ptr) };
                if len_py < 0 {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    return false;
                }
                let list_len = len_py as usize;
                let needed = 9 + list_len * 9;
                if out.capacity() - cur_len < needed {
                    out.reserve(needed.max(4096));
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    *dst.add(4) = 0x60;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                    let mut cur = dst.add(9);
                    for j in 0..len_py {
                        let elem_ptr = pyo3::ffi::PyList_GetItem(val_ptr, j);
                        let f = pyo3::ffi::PyFloat_AsDouble(elem_ptr);
                        if f == -1.0 && !pyo3::ffi::PyErr_Occurred().is_null() {
                            pyo3::ffi::PyErr_Clear();
                            return false;
                        }
                        *cur = 0x40;
                        ptr::write_unaligned(cur.add(1) as *mut u64, f.to_bits().to_le());
                        cur = cur.add(9);
                    }
                    out.set_len(cur_len + needed);
                }
                true
            }
            FastFieldKind::IntList => {
                unsafe {
                    if fast_abi && (*val_ptr).ob_type == ptr::addr_of_mut!(pyo3::ffi::PyList_Type) {
                        let len_py = *(val_ptr.byte_add(16) as *const isize);
                        if len_py < 0 {
                            return false;
                        }
                        let list_len = len_py as usize;
                        let max_needed = 9 + list_len * 9;
                        if out.capacity() - cur_len < max_needed {
                            out.reserve(max_needed.max(4096));
                        }
                        let dst = out.as_mut_ptr().add(cur_len);
                        ptr::write_unaligned(dst as *mut u32, id_u32);
                        *dst.add(4) = 0x60;
                        ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                        let items_ptr =
                            *(val_ptr.byte_add(24) as *const *const *mut pyo3::ffi::PyObject);
                        let long_type = ptr::addr_of_mut!(pyo3::ffi::PyLong_Type);
                        let mut cur = dst.add(9);
                        for j in 0..list_len {
                            let elem_ptr = *items_ptr.add(j);
                            if elem_ptr == py_true_ptr || elem_ptr == py_false_ptr {
                                return false;
                            }
                            let n = if (*elem_ptr).ob_type == long_type {
                                let tag = *(elem_ptr.byte_add(16) as *const usize);
                                if tag == long_tag_one {
                                    *(elem_ptr.byte_add(24) as *const u32) as i64
                                } else if tag == long_tag_zero {
                                    0
                                } else {
                                    let mut overflow: std::os::raw::c_int = 0;
                                    let v = pyo3::ffi::PyLong_AsLongLongAndOverflow(
                                        elem_ptr,
                                        &mut overflow,
                                    );
                                    if overflow != 0
                                        || (v == -1 && !pyo3::ffi::PyErr_Occurred().is_null())
                                    {
                                        pyo3::ffi::PyErr_Clear();
                                        return false;
                                    }
                                    v
                                }
                            } else {
                                let mut overflow: std::os::raw::c_int = 0;
                                let v = pyo3::ffi::PyLong_AsLongLongAndOverflow(
                                    elem_ptr,
                                    &mut overflow,
                                );
                                if overflow != 0
                                    || (v == -1 && !pyo3::ffi::PyErr_Occurred().is_null())
                                {
                                    pyo3::ffi::PyErr_Clear();
                                    return false;
                                }
                                v
                            };
                            if (n as u64) < 8 {
                                *cur = 0x30 | (n as u8);
                                cur = cur.add(1);
                            } else {
                                *cur = 0x38;
                                ptr::write_unaligned(cur.add(1) as *mut i64, n.to_le());
                                cur = cur.add(9);
                            }
                        }
                        let new_len = cur.offset_from(out.as_ptr()) as usize;
                        out.set_len(new_len);
                        return true;
                    }
                }
                let len_py = unsafe { pyo3::ffi::PyList_Size(val_ptr) };
                if len_py < 0 {
                    unsafe { pyo3::ffi::PyErr_Clear() };
                    return false;
                }
                let list_len = len_py as usize;
                let max_needed = 9 + list_len * 9;
                if out.capacity() - cur_len < max_needed {
                    out.reserve(max_needed.max(4096));
                }
                unsafe {
                    let dst = out.as_mut_ptr().add(cur_len);
                    ptr::write_unaligned(dst as *mut u32, id_u32);
                    *dst.add(4) = 0x60;
                    ptr::write_unaligned(dst.add(5) as *mut u32, (list_len as u32).to_le());
                    let mut cur = dst.add(9);
                    for j in 0..len_py {
                        let elem_ptr = pyo3::ffi::PyList_GetItem(val_ptr, j);
                        if elem_ptr == py_true_ptr || elem_ptr == py_false_ptr {
                            return false;
                        }
                        let mut overflow: std::os::raw::c_int = 0;
                        let n = pyo3::ffi::PyLong_AsLongLongAndOverflow(elem_ptr, &mut overflow);
                        if overflow != 0 || (n == -1 && !pyo3::ffi::PyErr_Occurred().is_null()) {
                            pyo3::ffi::PyErr_Clear();
                            return false;
                        }
                        if (n as u64) < 8 {
                            *cur = 0x30 | (n as u8);
                            cur = cur.add(1);
                        } else {
                            *cur = 0x38;
                            ptr::write_unaligned(cur.add(1) as *mut i64, n.to_le());
                            cur = cur.add(9);
                        }
                    }
                    let new_len = cur.offset_from(out.as_ptr()) as usize;
                    out.set_len(new_len);
                }
                true
            }
            FastFieldKind::Generic => false,
        }
    }

    #[inline(never)]
    fn serialize_value_fast(&mut self, val: &Bound<'_, PyAny>) -> PyResult<()> {
        if val.is_none() {
            self.work_buffer.push(0x10);
            return Ok(());
        }

        if let Ok(b) = val.downcast::<pyo3::types::PyBool>() {
            self.work_buffer.push(if b.is_true() { 0x21 } else { 0x20 });
            return Ok(());
        }

        if let Ok(l) = val.downcast::<pyo3::types::PyInt>() {
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
                self.serialize_value_fast(&item)?;
            }
            return Ok(());
        }

        // Fallback: handle dicts and complex types properly
        self.serialize_any_optimized(val)
    }

    fn serialize_pydantic_complex(
        &mut self,
        obj: &Bound<'_, PyAny>,
        field_keys: &[Bound<'_, PyAny>],
        field_ids: &[u32],
        dict_attr_ptr: *mut pyo3::ffi::PyObject,
    ) -> PyResult<()> {
        // Complex path: handles all types including datetime, UUID, Decimal
        self.work_buffer.push(0x70);

        let dict_ptr = unsafe {
            let p = pyo3::ffi::PyObject_GetAttr(obj.as_ptr(), dict_attr_ptr);
            if p.is_null() {
                pyo3::ffi::PyErr_Clear();
                self.work_buffer.push(0x7F);
                return Ok(());
            }
            p
        };

        let dict = unsafe { Bound::from_borrowed_ptr(obj.py(), dict_ptr) };
        let dict = match dict.downcast::<PyDict>() {
            Ok(d) => d,
            Err(_) => {
                unsafe {
                    pyo3::ffi::Py_DECREF(dict_ptr);
                }
                self.work_buffer.push(0x7F);
                return Ok(());
            }
        };

        for (i, key) in field_keys.iter().enumerate() {
            self.work_buffer
                .extend_from_slice(&field_ids[i].to_le_bytes());

            if let Some(value) = dict.get_item(key)? {
                self.serialize_value_ultra_fast(&value)?;
            } else {
                self.work_buffer.push(0x10);
            }
        }

        unsafe {
            pyo3::ffi::Py_DECREF(dict_ptr);
        }

        self.work_buffer.push(0x7F);
        Ok(())
    }

    fn serialize_value_ultra_fast(&mut self, val: &Bound<'_, PyAny>) -> PyResult<()> {
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
        if let Ok(l) = val.downcast::<pyo3::types::PyInt>() {
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
        if let Ok(name_obj) = val.get_type().name() {
            let type_name = name_obj.to_str().unwrap_or("");
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
                return self.serialize_value_ultra_fast(&enum_val);
            }
        }

        self.serialize_any_optimized(val)
    }

    #[inline(always)]
    fn hash_str_deterministic(bytes: &[u8]) -> u64 {
        let len = bytes.len();
        let p = bytes.as_ptr();
        let (lo, hi) = unsafe {
            if len >= 8 {
                (
                    ptr::read_unaligned(p as *const u64),
                    ptr::read_unaligned(p.add(len - 8) as *const u64),
                )
            } else if len >= 4 {
                (
                    ptr::read_unaligned(p as *const u32) as u64,
                    ptr::read_unaligned(p.add(len - 4) as *const u32) as u64,
                )
            } else if len > 0 {
                let b0 = *p as u64;
                let b1 = *p.add(len >> 1) as u64;
                let b2 = *p.add(len - 1) as u64;
                (b0 | (b1 << 8) | (b2 << 16), len as u64)
            } else {
                (0, 0)
            }
        };
        let mixed =
            (lo ^ hi.rotate_left(29) ^ ((len as u64) << 56)).wrapping_mul(0x517cc1b727220a95);
        mixed ^ (mixed >> 32)
    }

    #[inline(always)]
    fn get_or_create_string_id_fast(&mut self, key_str: &str) -> u32 {
        let bytes = key_str.as_bytes();
        let hash = Self::hash_str_deterministic(bytes);
        let slot = (hash as usize) & 63;

        if bytes.len() <= 16 {
            if let Some((cached_hash, id)) = self.key_cache[slot] {
                if cached_hash == hash {
                    return id;
                }
            }
        }

        self.get_or_create_string_id_slow(key_str, hash, slot)
    }

    #[inline(never)]
    fn get_or_create_string_id_slow(&mut self, key_str: &str, hash: u64, slot: usize) -> u32 {
        if let Some(&existing_id) = self.string_table.get(key_str) {
            self.key_cache[slot] = Some((hash, existing_id));
            return existing_id;
        }

        let new_id = self.next_id;
        self.string_table.insert(key_str.to_owned(), new_id);
        let bytes = key_str.as_bytes();
        self.string_table_bytes.push(bytes.len() as u8);
        self.string_table_bytes.extend_from_slice(bytes);
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
            let count = self.next_id as u16;
            ptr::write_unaligned(header.add(4) as *mut u16, count.to_le());
        }
    }

    #[inline(always)]
    fn write_string_table_vectorized(&mut self) -> PyResult<()> {
        if !self.string_table_bytes.is_empty() {
            self.work_buffer.extend_from_slice(&self.string_table_bytes);
        }
        Ok(())
    }

    fn serialize_any_optimized(&mut self, val: &Bound<'_, PyAny>) -> PyResult<()> {
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
        if let Ok(l) = val.downcast::<pyo3::types::PyInt>() {
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
                self.serialize_any_optimized(&v)?;
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
                let item = unsafe { Bound::from_borrowed_ptr(py, item_ptr) };
                self.serialize_any_optimized(&item)?;
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
                let item = unsafe { Bound::from_borrowed_ptr(py, item_ptr) };
                self.serialize_any_optimized(&item)?;
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
                self.serialize_any_optimized(&item)?;
            }
            return Ok(());
        }

        if let Ok(frozenset) = val.downcast::<PyFrozenSet>() {
            self.work_buffer.push(0x60);
            let len = frozenset.len();
            self.work_buffer
                .extend_from_slice(&(len as u32).to_le_bytes());

            for item in frozenset.iter() {
                self.serialize_any_optimized(&item)?;
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
        if let Ok(name_obj) = val.get_type().name() {
            let type_name = name_obj.to_str().unwrap_or("");
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
            let type_name_obj = val.get_type().name()?;
            let type_name = type_name_obj.to_str().unwrap_or("");

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
            if let Ok(type_name_obj) = val.get_type().name() {
                if type_name_obj.to_str().unwrap_or("") == "UUID" {
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
                return self.serialize_any_optimized(&enum_val);
            }
        }

        // 15. DataFrame & Series support (Polars, Pandas, PyArrow) - check BEFORE __dict__
        if let Ok(name_obj) = val.get_type().name() {
            let type_name = name_obj.to_str().unwrap_or("");
            if type_name == "DataFrame" {
                if val.hasattr("to_dicts")? {
                    // Polars DataFrame
                    let records = val.call_method0("to_dicts")?;
                    return self.serialize_any_optimized(&records);
                } else if val.hasattr("to_dict")? {
                    // Pandas DataFrame
                    let records = val.call_method1("to_dict", ("records",))?;
                    return self.serialize_any_optimized(&records);
                }
            } else if type_name == "LazyFrame" {
                if val.hasattr("collect")? {
                    let df = val.call_method0("collect")?;
                    if df.hasattr("to_dicts")? {
                        let records = df.call_method0("to_dicts")?;
                        return self.serialize_any_optimized(&records);
                    }
                }
            } else if (type_name == "Series" || type_name == "Index") && val.hasattr("to_list")? {
                let list_vals = val.call_method0("to_list")?;
                return self.serialize_any_optimized(&list_vals);
            } else if (type_name == "Table" || type_name == "RecordBatch")
                && val.hasattr("to_pylist")?
            {
                let records = val.call_method0("to_pylist")?;
                return self.serialize_any_optimized(&records);
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
                    self.serialize_any_optimized(&v)?;
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
fn _b_fast(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<BFast>()?;
    m.add_class::<lazy::BFastLazyView>()?;
    m.add_class::<streaming::BFastStreamDecoder>()?;
    m.add_class::<streaming::BFastStreamEncoder>()?;
    m.add_class::<native::BFastNativeSender>()?;
    m.add_function(wrap_pyfunction!(native::is_native_available, m)?)?;
    m.add_function(wrap_pyfunction!(native::send_native_socket, m)?)?;
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
    string_table: &'a [Bound<'py, PyString>],
    datetime_fromisoformat: Option<Bound<'py, PyAny>>,
    date_fromisoformat: Option<Bound<'py, PyAny>>,
    time_fromisoformat: Option<Bound<'py, PyAny>>,
    uuid_class: Option<Bound<'py, PyAny>>,
    decimal_class: Option<Bound<'py, PyAny>>,
    py_none: &'a PyObject,
    py_true: &'a PyObject,
    py_false: &'a PyObject,
    small_ints: &'a [PyObject; 16],
    recursion_depth: usize,
}

impl<'a, 'py> BFastParser<'a, 'py> {
    fn get_datetime_fromisoformat(&mut self) -> PyResult<&Bound<'py, PyAny>> {
        if self.datetime_fromisoformat.is_none() {
            let func = self
                .py
                .import("datetime")?
                .getattr("datetime")?
                .getattr("fromisoformat")?;
            self.datetime_fromisoformat = Some(func);
        }
        Ok(self.datetime_fromisoformat.as_ref().unwrap())
    }

    fn get_date_fromisoformat(&mut self) -> PyResult<&Bound<'py, PyAny>> {
        if self.date_fromisoformat.is_none() {
            let func = self
                .py
                .import("datetime")?
                .getattr("date")?
                .getattr("fromisoformat")?;
            self.date_fromisoformat = Some(func);
        }
        Ok(self.date_fromisoformat.as_ref().unwrap())
    }

    fn get_time_fromisoformat(&mut self) -> PyResult<&Bound<'py, PyAny>> {
        if self.time_fromisoformat.is_none() {
            let func = self
                .py
                .import("datetime")?
                .getattr("time")?
                .getattr("fromisoformat")?;
            self.time_fromisoformat = Some(func);
        }
        Ok(self.time_fromisoformat.as_ref().unwrap())
    }

    fn get_uuid_class(&mut self) -> PyResult<&Bound<'py, PyAny>> {
        if self.uuid_class.is_none() {
            let cls = self.py.import("uuid")?.getattr("UUID")?;
            self.uuid_class = Some(cls);
        }
        Ok(self.uuid_class.as_ref().unwrap())
    }

    fn get_decimal_class(&mut self) -> PyResult<&Bound<'py, PyAny>> {
        if self.decimal_class.is_none() {
            let cls = self.py.import("decimal")?.getattr("Decimal")?;
            self.decimal_class = Some(cls);
        }
        Ok(self.decimal_class.as_ref().unwrap())
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

                let key = &self.string_table[key_id];
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
            return Ok(obj.unbind());
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
            return Ok(obj.unbind());
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
            return Ok(obj.unbind());
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
            return Ok(obj.unbind());
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
            return Ok(obj.unbind());
        }

        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown tag: 0x{:02x}",
            tag
        )))
    }
}
