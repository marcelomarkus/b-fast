use std::ffi::{c_char, c_int, c_void, CStr};
use std::slice;

use pyo3::prelude::*;
use pyo3::types::PyAny;

/// ABI struct matching ArrowSchema from Apache Arrow C Data Interface
#[repr(C)]
pub struct ArrowSchema {
    pub format: *const c_char,
    pub name: *const c_char,
    pub metadata: *const c_char,
    pub flags: i64,
    pub n_children: i64,
    pub children: *mut *mut ArrowSchema,
    pub dictionary: *mut ArrowSchema,
    pub release: Option<unsafe extern "C" fn(*mut ArrowSchema)>,
    pub private_data: *mut c_void,
}

pub struct SafeSchema(pub ArrowSchema);

impl Drop for SafeSchema {
    fn drop(&mut self) {
        if let Some(rel) = self.0.release.take() {
            unsafe { rel(&mut self.0) };
        }
    }
}

/// ABI struct matching ArrowArray from Apache Arrow C Data Interface
#[repr(C)]
pub struct ArrowArray {
    pub length: i64,
    pub null_count: i64,
    pub offset: i64,
    pub n_buffers: i64,
    pub n_children: i64,
    pub buffers: *mut *const c_void,
    pub children: *mut *mut ArrowArray,
    pub dictionary: *mut ArrowArray,
    pub release: Option<unsafe extern "C" fn(*mut ArrowArray)>,
    pub private_data: *mut c_void,
}

pub struct SafeArray(pub ArrowArray);

impl Drop for SafeArray {
    fn drop(&mut self) {
        if let Some(rel) = self.0.release.take() {
            unsafe { rel(&mut self.0) };
        }
    }
}

/// ABI struct matching ArrowArrayStream from Apache Arrow C Stream Interface
#[repr(C)]
pub struct ArrowArrayStream {
    pub get_schema: Option<unsafe extern "C" fn(*mut ArrowArrayStream, *mut ArrowSchema) -> c_int>,
    pub get_next: Option<unsafe extern "C" fn(*mut ArrowArrayStream, *mut ArrowArray) -> c_int>,
    pub get_last_error: Option<unsafe extern "C" fn(*mut ArrowArrayStream) -> *const c_char>,
    pub release: Option<unsafe extern "C" fn(*mut ArrowArrayStream)>,
    pub private_data: *mut c_void,
}

/// Arrow StringView (16 bytes layout from Arrow 15+ / Polars)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArrowStringView {
    pub length: u32,
    pub prefix: [u8; 4],
    pub buffer_index: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowTimeUnit {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowColumnType {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
    Boolean,
    StringView,
    Utf8,
    LargeUtf8,
    BinaryView,
    Binary,
    LargeBinary,
    Date32,
    Date64,
    Timestamp(ArrowTimeUnit),
    Unsupported,
}

impl ArrowColumnType {
    pub fn parse(format_str: &str) -> Self {
        match format_str {
            "c" => ArrowColumnType::Int8,
            "C" => ArrowColumnType::UInt8,
            "s" => ArrowColumnType::Int16,
            "S" => ArrowColumnType::UInt16,
            "i" => ArrowColumnType::Int32,
            "I" => ArrowColumnType::UInt32,
            "l" => ArrowColumnType::Int64,
            "L" => ArrowColumnType::UInt64,
            "f" => ArrowColumnType::Float32,
            "g" => ArrowColumnType::Float64,
            "b" => ArrowColumnType::Boolean,
            "vu" => ArrowColumnType::StringView,
            "u" => ArrowColumnType::Utf8,
            "U" => ArrowColumnType::LargeUtf8,
            "vz" => ArrowColumnType::BinaryView,
            "z" => ArrowColumnType::Binary,
            "Z" => ArrowColumnType::LargeBinary,
            "tdD" => ArrowColumnType::Date32,
            "tdm" => ArrowColumnType::Date64,
            s if s.starts_with("tss") => ArrowColumnType::Timestamp(ArrowTimeUnit::Second),
            s if s.starts_with("tsm") => ArrowColumnType::Timestamp(ArrowTimeUnit::Millisecond),
            s if s.starts_with("tsu") => ArrowColumnType::Timestamp(ArrowTimeUnit::Microsecond),
            s if s.starts_with("tsn") => ArrowColumnType::Timestamp(ArrowTimeUnit::Nanosecond),
            _ => ArrowColumnType::Unsupported,
        }
    }
}

/// Howard Hinnant's civil date algorithm to convert days since 1970-01-01 to (year, month, day)
#[inline(always)]
pub fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1028 + doe / 1461 - doe / 146096) / 365;
    let y = (yoe as i32) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[inline(always)]
pub fn write_ymd(buf: &mut [u8; 10], year: i32, month: u32, day: u32) {
    let y = year.max(0) as u32;
    buf[0] = b'0' + ((y / 1000) % 10) as u8;
    buf[1] = b'0' + ((y / 100) % 10) as u8;
    buf[2] = b'0' + ((y / 10) % 10) as u8;
    buf[3] = b'0' + (y % 10) as u8;
    buf[4] = b'-';
    buf[5] = b'0' + ((month / 10) % 10) as u8;
    buf[6] = b'0' + (month % 10) as u8;
    buf[7] = b'-';
    buf[8] = b'0' + ((day / 10) % 10) as u8;
    buf[9] = b'0' + (day % 10) as u8;
}

/// # Safety
/// Caller must ensure `bitmap` is either null or points to a valid allocation of at least `(idx >> 3) + 1` bytes.
#[inline(always)]
pub unsafe fn is_valid(bitmap: *const u8, idx: usize) -> bool {
    if bitmap.is_null() {
        return true;
    }
    let byte = *bitmap.add(idx >> 3);
    (byte & (1 << (idx & 7))) != 0
}

/// # Safety
/// Caller must ensure `views_ptr` and `variadic_buffers` point to valid memory according to Arrow StringView spec.
#[inline(always)]
pub unsafe fn get_string_view<'a>(
    views_ptr: *const ArrowStringView,
    variadic_buffers: &[*const u8],
    idx: usize,
) -> &'a [u8] {
    let view = &*views_ptr.add(idx);
    let len = view.length as usize;
    if len <= 12 {
        let raw_view_bytes = views_ptr.add(idx) as *const u8;
        slice::from_raw_parts(raw_view_bytes.add(4), len)
    } else {
        let buf_idx = view.buffer_index as usize;
        let offset = view.offset as usize;
        let buf = variadic_buffers[buf_idx];
        slice::from_raw_parts(buf.add(offset), len)
    }
}

pub struct ColumnBatchReader {
    pub col_type: ArrowColumnType,
    pub null_count: i64,
    pub offset: usize,
    pub null_bitmap: *const u8,
    pub data_ptr: *const u8,
    pub offsets32_ptr: *const i32,
    pub offsets64_ptr: *const i64,
    pub views_ptr: *const ArrowStringView,
    pub variadic_buffers: Vec<*const u8>,
}

impl ColumnBatchReader {
    /// # Safety
    /// Caller must ensure `col_arr` points to a valid `ArrowArray` conforming to the Arrow C Data Interface specification.
    pub unsafe fn from_array(col_arr: &ArrowArray, col_type: &ArrowColumnType) -> Option<Self> {
        let offset = col_arr.offset as usize;
        let null_count = col_arr.null_count;
        let n_buffers = col_arr.n_buffers as usize;
        let buffers = col_arr.buffers;

        let null_bitmap = if n_buffers > 0 && !buffers.is_null() {
            *buffers as *const u8
        } else {
            std::ptr::null()
        };

        let mut data_ptr = std::ptr::null();
        let mut offsets32_ptr = std::ptr::null();
        let mut offsets64_ptr = std::ptr::null();
        let mut views_ptr = std::ptr::null();
        let mut variadic_buffers = Vec::new();

        match col_type {
            ArrowColumnType::Boolean
            | ArrowColumnType::Int8
            | ArrowColumnType::UInt8
            | ArrowColumnType::Int16
            | ArrowColumnType::UInt16
            | ArrowColumnType::Int32
            | ArrowColumnType::UInt32
            | ArrowColumnType::Int64
            | ArrowColumnType::UInt64
            | ArrowColumnType::Float32
            | ArrowColumnType::Float64
            | ArrowColumnType::Date32
            | ArrowColumnType::Date64
            | ArrowColumnType::Timestamp(_) => {
                if n_buffers >= 2 && !buffers.is_null() {
                    data_ptr = *buffers.add(1) as *const u8;
                }
                if data_ptr.is_null() {
                    return None;
                }
            }
            ArrowColumnType::StringView | ArrowColumnType::BinaryView => {
                if n_buffers >= 2 && !buffers.is_null() {
                    views_ptr = *buffers.add(1) as *const ArrowStringView;
                    for i in 2..n_buffers {
                        variadic_buffers.push(*buffers.add(i) as *const u8);
                    }
                }
                if views_ptr.is_null() {
                    return None;
                }
            }
            ArrowColumnType::Utf8 | ArrowColumnType::Binary => {
                if n_buffers >= 3 && !buffers.is_null() {
                    offsets32_ptr = *buffers.add(1) as *const i32;
                    data_ptr = *buffers.add(2) as *const u8;
                }
                if offsets32_ptr.is_null() || data_ptr.is_null() {
                    return None;
                }
            }
            ArrowColumnType::LargeUtf8 | ArrowColumnType::LargeBinary => {
                if n_buffers >= 3 && !buffers.is_null() {
                    offsets64_ptr = *buffers.add(1) as *const i64;
                    data_ptr = *buffers.add(2) as *const u8;
                }
                if offsets64_ptr.is_null() || data_ptr.is_null() {
                    return None;
                }
            }
            ArrowColumnType::Unsupported => return None,
        }

        Some(ColumnBatchReader {
            col_type: col_type.clone(),
            null_count,
            offset,
            null_bitmap,
            data_ptr,
            offsets32_ptr,
            offsets64_ptr,
            views_ptr,
            variadic_buffers,
        })
    }

    /// # Safety
    /// Caller must ensure `row` is within the valid bounds of the column batch and buffer pointers are valid.
    #[inline(always)]
    pub unsafe fn write_value_to(&self, row: usize, bfast: &mut crate::BFast) {
        let abs_idx = self.offset + row;

        if self.null_count > 0 && !is_valid(self.null_bitmap, abs_idx) {
            bfast.write_arrow_null();
            return;
        }

        match &self.col_type {
            ArrowColumnType::Int64 => {
                let val = *(self.data_ptr as *const i64).add(abs_idx);
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::Int32 => {
                let val = *(self.data_ptr as *const i32).add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::Int16 => {
                let val = *(self.data_ptr as *const i16).add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::Int8 => {
                let val = *(self.data_ptr as *const i8).add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::UInt64 => {
                let val = *(self.data_ptr as *const u64).add(abs_idx);
                bfast.write_arrow_int(val as i64);
            }
            ArrowColumnType::UInt32 => {
                let val = *(self.data_ptr as *const u32).add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::UInt16 => {
                let val = *(self.data_ptr as *const u16).add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::UInt8 => {
                let val = *self.data_ptr.add(abs_idx) as i64;
                bfast.write_arrow_int(val);
            }
            ArrowColumnType::Float64 => {
                let val = *(self.data_ptr as *const f64).add(abs_idx);
                bfast.write_arrow_float(val);
            }
            ArrowColumnType::Float32 => {
                let val = *(self.data_ptr as *const f32).add(abs_idx) as f64;
                bfast.write_arrow_float(val);
            }
            ArrowColumnType::Boolean => {
                let val = is_valid(self.data_ptr, abs_idx);
                bfast.write_arrow_bool(val);
            }
            ArrowColumnType::StringView | ArrowColumnType::BinaryView => {
                let bytes = get_string_view(self.views_ptr, &self.variadic_buffers, abs_idx);
                bfast.write_arrow_str(bytes);
            }
            ArrowColumnType::Utf8 | ArrowColumnType::Binary => {
                let start = *self.offsets32_ptr.add(abs_idx) as usize;
                let end = *self.offsets32_ptr.add(abs_idx + 1) as usize;
                let bytes = slice::from_raw_parts(self.data_ptr.add(start), end - start);
                bfast.write_arrow_str(bytes);
            }
            ArrowColumnType::LargeUtf8 | ArrowColumnType::LargeBinary => {
                let start = *self.offsets64_ptr.add(abs_idx) as usize;
                let end = *self.offsets64_ptr.add(abs_idx + 1) as usize;
                let bytes = slice::from_raw_parts(self.data_ptr.add(start), end - start);
                bfast.write_arrow_str(bytes);
            }
            ArrowColumnType::Date32 => {
                let days = *(self.data_ptr as *const i32).add(abs_idx);
                let (y, m, d) = days_to_ymd(days);
                let mut buf = [0u8; 10];
                write_ymd(&mut buf, y, m, d);
                bfast.write_arrow_date(&buf);
            }
            ArrowColumnType::Date64 => {
                let ms = *(self.data_ptr as *const i64).add(abs_idx);
                let days = (ms / 86_400_000) as i32;
                let (y, m, d) = days_to_ymd(days);
                let mut buf = [0u8; 10];
                write_ymd(&mut buf, y, m, d);
                bfast.write_arrow_date(&buf);
            }
            ArrowColumnType::Timestamp(unit) => {
                let ts = *(self.data_ptr as *const i64).add(abs_idx);
                let (sec, usec) = match unit {
                    ArrowTimeUnit::Second => (ts, 0u32),
                    ArrowTimeUnit::Millisecond => {
                        (ts.div_euclid(1_000), (ts.rem_euclid(1_000) * 1_000) as u32)
                    }
                    ArrowTimeUnit::Microsecond => (
                        ts.div_euclid(1_000_000),
                        ts.rem_euclid(1_000_000) as u32,
                    ),
                    ArrowTimeUnit::Nanosecond => (
                        ts.div_euclid(1_000_000_000),
                        (ts.rem_euclid(1_000_000_000) / 1_000) as u32,
                    ),
                };
                let days = sec.div_euclid(86_400) as i32;
                let day_sec = sec.rem_euclid(86_400) as u32;
                let (y, m, d) = days_to_ymd(days);
                let hour = day_sec / 3600;
                let min = (day_sec % 3600) / 60;
                let s = day_sec % 60;

                let iso_str = if usec == 0 {
                    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{s:02}")
                } else {
                    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{s:02}.{usec:06}")
                };
                bfast.write_arrow_datetime(iso_str.as_bytes());
            }
            ArrowColumnType::Unsupported => {
                bfast.write_arrow_null();
            }
        }
    }
}

/// Stream serializer: directly serializes an Arrow C Stream into BFast's work_buffer
pub fn serialize_arrow_stream(
    bfast: &mut crate::BFast,
    obj: &Bound<'_, PyAny>,
    orient: &str,
) -> PyResult<bool> {
    if !obj.hasattr("__arrow_c_stream__")? {
        return Ok(false);
    }

    // Retain capsule in local scope while reading stream
    let capsule = match obj.call_method0("__arrow_c_stream__") {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };

    let stream_ptr = unsafe {
        let name = c"arrow_array_stream".as_ptr();
        pyo3::ffi::PyCapsule_GetPointer(capsule.as_ptr(), name) as *mut ArrowArrayStream
    };

    if stream_ptr.is_null() {
        unsafe { pyo3::ffi::PyErr_Clear() };
        return Ok(false);
    }

    let stream = unsafe { &mut *stream_ptr };

    let mut raw_schema = unsafe { std::mem::zeroed::<ArrowSchema>() };
    let res = unsafe {
        match stream.get_schema {
            Some(f) => f(stream_ptr, &mut raw_schema),
            None => return Ok(false),
        }
    };

    if res != 0 || raw_schema.release.is_none() {
        return Ok(false);
    }

    let schema = SafeSchema(raw_schema);

    let format_cstr = unsafe {
        if schema.0.format.is_null() {
            return Ok(false);
        }
        CStr::from_ptr(schema.0.format)
    };
    let format_str = match format_cstr.to_str() {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    let is_struct = format_str == "+s";
    let mut col_specs = Vec::new();
    let mut col_key_ids = Vec::new();
    let mut single_col_type = ArrowColumnType::Unsupported;

    if is_struct {
        let n_cols = schema.0.n_children as usize;
        col_specs.reserve(n_cols);
        col_key_ids.reserve(n_cols);

        for i in 0..n_cols {
            let child_schema_ptr = unsafe { *schema.0.children.add(i) };
            if child_schema_ptr.is_null() {
                return Ok(false);
            }
            let child_schema = unsafe { &*child_schema_ptr };
            if child_schema.name.is_null() || child_schema.format.is_null() {
                return Ok(false);
            }
            let name_cstr = unsafe { CStr::from_ptr(child_schema.name) };
            let name_str = match name_cstr.to_str() {
                Ok(s) => s,
                Err(_) => return Ok(false),
            };
            let child_format_cstr = unsafe { CStr::from_ptr(child_schema.format) };
            let child_format_str = match child_format_cstr.to_str() {
                Ok(s) => s,
                Err(_) => return Ok(false),
            };
            let col_type = ArrowColumnType::parse(child_format_str);
            if col_type == ArrowColumnType::Unsupported {
                return Ok(false);
            }
            let key_id = bfast.get_or_create_string_id_fast(name_str);
            col_specs.push(col_type);
            col_key_ids.push(key_id);
        }
    } else {
        single_col_type = ArrowColumnType::parse(format_str);
        if single_col_type == ArrowColumnType::Unsupported {
            return Ok(false);
        }
    }

    // Release schema early as we're done extracting column metadata
    drop(schema);

    // Collect batches into SafeArray wrappers
    let mut batches: Vec<SafeArray> = Vec::new();
    let mut total_rows: usize = 0;

    loop {
        let mut raw_batch = unsafe { std::mem::zeroed::<ArrowArray>() };
        let res = unsafe {
            match stream.get_next {
                Some(f) => f(stream_ptr, &mut raw_batch),
                None => return Ok(false),
            }
        };
        if res != 0 || raw_batch.release.is_none() {
            break;
        }
        total_rows += raw_batch.length as usize;
        batches.push(SafeArray(raw_batch));
    }

    let mut result_ok = true;

    if is_struct {
        let n_cols = col_specs.len();

        if orient == "columns" {
            bfast.ensure_buffer_capacity(2 + n_cols * (5 + total_rows * 9));
            bfast.push_work_byte(0x70); // dict

            for (c, key_id) in col_key_ids.iter().enumerate().take(n_cols) {
                bfast.extend_work_bytes(&key_id.to_le_bytes());
                bfast.push_work_byte(0x60); // list
                bfast.extend_work_bytes(&(total_rows as u32).to_le_bytes());

                for safe_batch in &batches {
                    let batch = &safe_batch.0;
                    let col_arr_ptr = unsafe { *batch.children.add(c) };
                    let reader = unsafe {
                        match ColumnBatchReader::from_array(&*col_arr_ptr, &col_specs[c]) {
                            Some(r) => r,
                            None => {
                                result_ok = false;
                                break;
                            }
                        }
                    };
                    let b_len = batch.length as usize;
                    for row in 0..b_len {
                        unsafe { reader.write_value_to(row, bfast) };
                    }
                }
                if !result_ok {
                    break;
                }
            }
            if result_ok {
                bfast.push_work_byte(0x7F); // end of dict
            }
        } else {
            // "records" orientation (default)
            let est_row_size = 1 + n_cols * 12 + 1;
            bfast.ensure_buffer_capacity(5 + total_rows * est_row_size);

            bfast.push_work_byte(0x60); // list
            bfast.extend_work_bytes(&(total_rows as u32).to_le_bytes());

            for safe_batch in &batches {
                let batch = &safe_batch.0;
                let mut readers = Vec::with_capacity(n_cols);
                for (c, spec) in col_specs.iter().enumerate().take(n_cols) {
                    let col_arr_ptr = unsafe { *batch.children.add(c) };
                    let maybe_reader = unsafe {
                        ColumnBatchReader::from_array(&*col_arr_ptr, spec)
                    };
                    match maybe_reader {
                        Some(r) => readers.push(r),
                        None => {
                            result_ok = false;
                            break;
                        }
                    }
                }

                if !result_ok {
                    break;
                }

                let b_len = batch.length as usize;
                for row in 0..b_len {
                    bfast.push_work_byte(0x70); // dict
                    for c in 0..n_cols {
                        bfast.extend_work_bytes(&col_key_ids[c].to_le_bytes());
                        unsafe { readers[c].write_value_to(row, bfast) };
                    }
                    bfast.push_work_byte(0x7F); // end of dict
                }
            }
        }
    } else {
        // Series (single column)
        bfast.ensure_buffer_capacity(5 + total_rows * 9);
        bfast.push_work_byte(0x60); // list
        bfast.extend_work_bytes(&(total_rows as u32).to_le_bytes());

        for safe_batch in &batches {
            let batch = &safe_batch.0;
            let reader = unsafe {
                match ColumnBatchReader::from_array(batch, &single_col_type) {
                    Some(r) => r,
                    None => {
                        result_ok = false;
                        break;
                    }
                }
            };
            let b_len = batch.length as usize;
            for row in 0..b_len {
                unsafe { reader.write_value_to(row, bfast) };
            }
        }
    }

    Ok(result_ok)
}
