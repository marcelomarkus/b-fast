#![allow(static_mut_refs)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use lz4_flex::block::decompress_into;
use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};
use std::convert::TryInto;

// Constants for protocol tags
const TAG_NULL: u8 = 0x10;
const TAG_FALSE: u8 = 0x20;
const TAG_TRUE: u8 = 0x21;
const TAG_INT64: u8 = 0x38;
const TAG_FLOAT64: u8 = 0x40;
const TAG_STRING: u8 = 0x50;
const TAG_LIST: u8 = 0x60;
const TAG_OBJECT: u8 = 0x70;
const TAG_OBJECT_END: u8 = 0x7F;
const TAG_BYTES: u8 = 0x80;
const TAG_NUMPY: u8 = 0x90;
const TAG_DATETIME: u8 = 0xD1;
const TAG_DATE: u8 = 0xD2;
const TAG_TIME: u8 = 0xD3;
const TAG_UUID: u8 = 0xD4;
const TAG_DECIMAL: u8 = 0xD5;

const STATUS_ERROR: i32 = -1;
const STATUS_EXTENDED_TYPES: i32 = -2;
const MAX_RECURSION_DEPTH: usize = 128;

// Static reusable buffers for zero-allocation WebAssembly execution
static mut DECOMPRESS_BUFFER: Vec<u8> = Vec::new();
static mut JSON_BUFFER: Vec<u8> = Vec::new();

#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    unsafe {
        let layout = Layout::from_size_align_unchecked(size.max(1), 8);
        sys_alloc(layout)
    }
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    unsafe {
        let layout = Layout::from_size_align_unchecked(size.max(1), 8);
        sys_dealloc(ptr, layout);
    }
}

#[no_mangle]
pub extern "C" fn decompress(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
) -> i32 {
    let src = unsafe { std::slice::from_raw_parts(src_ptr, src_len) };
    let dst = unsafe { std::slice::from_raw_parts_mut(dst_ptr, dst_len) };
    match decompress_into(src, dst) {
        Ok(decompressed_len) => decompressed_len as i32,
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn decompress_parallel(
    src_ptr: *const u8,
    src_len: usize,
    dst_ptr: *mut u8,
    dst_len: usize,
) -> i32 {
    let src = unsafe { std::slice::from_raw_parts(src_ptr, src_len) };
    let dst = unsafe { std::slice::from_raw_parts_mut(dst_ptr, dst_len) };

    if src.len() < 8 {
        return -1;
    }

    let uncompressed_size = match src[0..4].try_into() {
        Ok(b) => u32::from_le_bytes(b) as usize,
        Err(_) => return -1,
    };
    let chunks_count = match src[4..8].try_into() {
        Ok(b) => u32::from_le_bytes(b) as usize,
        Err(_) => return -1,
    };

    if dst.len() < uncompressed_size {
        return -1;
    }

    let mut offset = 8;
    let mut write_offset = 0;

    for _ in 0..chunks_count {
        if offset + 4 > src.len() {
            return -1;
        }
        let chunk_len = u32::from_le_bytes(src[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + chunk_len > src.len() || chunk_len < 4 {
            return -1;
        }
        let chunk_data = &src[offset..offset + chunk_len];
        offset += chunk_len;

        let chunk_uncompressed_size =
            u32::from_le_bytes(chunk_data[0..4].try_into().unwrap()) as usize;
        if write_offset + chunk_uncompressed_size > dst.len() {
            return -1;
        }

        match decompress_into(
            &chunk_data[4..],
            &mut dst[write_offset..write_offset + chunk_uncompressed_size],
        ) {
            Ok(decompressed_len) => {
                write_offset += decompressed_len;
            }
            Err(_) => return -1,
        }
    }

    write_offset as i32
}

fn write_json_str(buf: &mut Vec<u8>, s: &[u8]) {
    buf.push(b'"');
    let mut last = 0;
    for (i, &b) in s.iter().enumerate() {
        let esc: &[u8] = match b {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            b'\t' => b"\\t",
            0x08 => b"\\b",
            0x0C => b"\\f",
            _ => continue,
        };
        buf.extend_from_slice(&s[last..i]);
        buf.extend_from_slice(esc);
        last = i + 1;
    }
    buf.extend_from_slice(&s[last..]);
    buf.push(b'"');
}

fn write_uuid(buf: &mut Vec<u8>, hex: &[u8]) {
    buf.push(b'"');
    if hex.len() == 32 {
        buf.extend_from_slice(&hex[0..8]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[8..12]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[12..16]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[16..20]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[20..32]);
    } else {
        buf.extend_from_slice(hex);
    }
    buf.push(b'"');
}

fn format_uuid_bytes(buf: &mut Vec<u8>, hex: &[u8]) {
    if hex.len() == 32 {
        buf.extend_from_slice(&hex[0..8]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[8..12]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[12..16]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[16..20]);
        buf.push(b'-');
        buf.extend_from_slice(&hex[20..32]);
    } else {
        buf.extend_from_slice(hex);
    }
}

struct Parser<'a> {
    data: &'a [u8],
    offset: usize,
    precomputed_keys: Vec<Vec<u8>>,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8]) -> Result<Self, i32> {
        if data.len() < 6 {
            return Err(STATUS_ERROR);
        }
        if data[0] != b'B' || data[1] != b'F' {
            return Err(STATUS_ERROR);
        }

        let string_table_count = u16::from_le_bytes(data[4..6].try_into().unwrap()) as usize;
        let mut offset = 6;
        let mut precomputed_keys = Vec::with_capacity(string_table_count);

        for _ in 0..string_table_count {
            if offset >= data.len() {
                return Err(STATUS_ERROR);
            }
            let length = data[offset] as usize;
            offset += 1;
            if offset + length > data.len() {
                return Err(STATUS_ERROR);
            }
            let str_bytes = &data[offset..offset + length];
            offset += length;

            let mut key_buf = Vec::with_capacity(length + 3);
            write_json_str(&mut key_buf, str_bytes);
            key_buf.push(b':');
            precomputed_keys.push(key_buf);
        }

        Ok(Parser {
            data,
            offset,
            precomputed_keys,
            depth: 0,
        })
    }

    fn parse_value(&mut self, buf: &mut Vec<u8>) -> Result<(), i32> {
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(STATUS_ERROR);
        }
        if self.offset >= self.data.len() {
            return Err(STATUS_ERROR);
        }

        self.depth += 1;
        let tag = self.data[self.offset];
        self.offset += 1;

        let res = match tag {
            TAG_NULL => {
                buf.extend_from_slice(b"null");
                Ok(())
            }
            TAG_FALSE => {
                buf.extend_from_slice(b"false");
                Ok(())
            }
            TAG_TRUE => {
                buf.extend_from_slice(b"true");
                Ok(())
            }
            TAG_INT64 => {
                if self.offset + 8 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let val =
                    i64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
                self.offset += 8;
                let mut itoa_buf = itoa::Buffer::new();
                buf.extend_from_slice(itoa_buf.format(val).as_bytes());
                Ok(())
            }
            t if (t & 0xF0) == 0x30 => {
                let val = t & 0x0F;
                if val < 10 {
                    buf.push(b'0' + val);
                } else {
                    buf.push(b'1');
                    buf.push(b'0' + (val - 10));
                }
                Ok(())
            }
            TAG_FLOAT64 => {
                if self.offset + 8 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let val =
                    f64::from_le_bytes(self.data[self.offset..self.offset + 8].try_into().unwrap());
                self.offset += 8;
                if val.is_nan() || val.is_infinite() {
                    buf.extend_from_slice(b"null");
                } else {
                    let mut ryu_buf = ryu::Buffer::new();
                    buf.extend_from_slice(ryu_buf.format(val).as_bytes());
                }
                Ok(())
            }
            TAG_STRING => {
                if self.offset + 4 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let len =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;
                if self.offset + len > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let s = &self.data[self.offset..self.offset + len];
                self.offset += len;
                write_json_str(buf, s);
                Ok(())
            }
            TAG_LIST => {
                if self.offset + 4 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let len =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;
                buf.push(b'[');
                for i in 0..len {
                    if i > 0 {
                        buf.push(b',');
                    }
                    self.parse_value(buf)?;
                }
                buf.push(b']');
                Ok(())
            }
            TAG_OBJECT => {
                buf.push(b'{');
                let mut first = true;
                while self.offset < self.data.len() && self.data[self.offset] != TAG_OBJECT_END {
                    if self.offset + 4 > self.data.len() {
                        return Err(STATUS_ERROR);
                    }
                    let key_id = u32::from_le_bytes(
                        self.data[self.offset..self.offset + 4].try_into().unwrap(),
                    ) as usize;
                    self.offset += 4;
                    if key_id >= self.precomputed_keys.len() {
                        return Err(STATUS_ERROR);
                    }
                    if !first {
                        buf.push(b',');
                    }
                    first = false;
                    buf.extend_from_slice(&self.precomputed_keys[key_id]);
                    self.parse_value(buf)?;
                }
                if self.offset >= self.data.len() {
                    return Err(STATUS_ERROR);
                }
                self.offset += 1; // Skip TAG_OBJECT_END
                buf.push(b'}');
                Ok(())
            }
            TAG_UUID => {
                if self.offset + 4 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let len =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;
                if self.offset + len > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let hex = &self.data[self.offset..self.offset + len];
                self.offset += len;
                write_uuid(buf, hex);
                Ok(())
            }
            TAG_DECIMAL => {
                if self.offset + 4 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let len =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;
                if self.offset + len > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let dec = &self.data[self.offset..self.offset + len];
                self.offset += len;
                buf.extend_from_slice(dec);
                Ok(())
            }
            TAG_TIME => {
                if self.offset + 4 > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let len =
                    u32::from_le_bytes(self.data[self.offset..self.offset + 4].try_into().unwrap())
                        as usize;
                self.offset += 4;
                if self.offset + len > self.data.len() {
                    return Err(STATUS_ERROR);
                }
                let time_bytes = &self.data[self.offset..self.offset + len];
                self.offset += len;
                write_json_str(buf, time_bytes);
                Ok(())
            }
            TAG_DATETIME | TAG_DATE | TAG_BYTES | TAG_NUMPY => Err(STATUS_EXTENDED_TYPES),
            _ => Err(STATUS_ERROR),
        };

        self.depth -= 1;
        res
    }
}

#[no_mangle]
pub extern "C" fn decode_to_json(src_ptr: *const u8, src_len: usize) -> i32 {
    let src = unsafe { std::slice::from_raw_parts(src_ptr, src_len) };
    if src.len() < 2 {
        return STATUS_ERROR;
    }

    // Determine decompressed slice
    let data: &[u8] = if src[0] == b'B' && src[1] == b'F' {
        src
    } else {
        // Needs decompression
        if src.len() < 4 {
            return STATUS_ERROR;
        }
        let uncompressed_size = match src[0..4].try_into() {
            Ok(b) => u32::from_le_bytes(b) as usize,
            Err(_) => return STATUS_ERROR,
        };

        let decompress_buf = unsafe { &mut DECOMPRESS_BUFFER };
        decompress_buf.clear();
        decompress_buf.resize(uncompressed_size, 0);

        // Try single-block LZ4 first
        if let Ok(decompressed_len) = decompress_into(&src[4..], decompress_buf.as_mut_slice()) {
            decompress_buf.truncate(decompressed_len);
            decompress_buf.as_slice()
        } else if src.len() >= 8 {
            // Try parallel chunked LZ4
            let ret = decompress_parallel(
                src.as_ptr(),
                src.len(),
                decompress_buf.as_mut_ptr(),
                uncompressed_size,
            );
            if ret >= 0 {
                decompress_buf.truncate(ret as usize);
                decompress_buf.as_slice()
            } else {
                return STATUS_ERROR;
            }
        } else {
            return STATUS_ERROR;
        }
    };

    let mut parser = match Parser::new(data) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let json_buf = unsafe { &mut JSON_BUFFER };
    json_buf.clear();
    json_buf.reserve(data.len() * 2);

    match parser.parse_value(json_buf) {
        Ok(()) => json_buf.len() as i32,
        Err(e) => e,
    }
}

#[no_mangle]
pub extern "C" fn get_result_ptr() -> *const u8 {
    unsafe { JSON_BUFFER.as_ptr() }
}

#[no_mangle]
pub extern "C" fn get_result_len() -> usize {
    unsafe { JSON_BUFFER.len() }
}

#[no_mangle]
pub extern "C" fn get_decompressed_ptr() -> *const u8 {
    unsafe { DECOMPRESS_BUFFER.as_ptr() }
}

#[no_mangle]
pub extern "C" fn get_decompressed_len() -> usize {
    unsafe { DECOMPRESS_BUFFER.len() }
}

// =========================================================================
// BFastTable: Zero-GC Columnar Engine for WebAssembly
// =========================================================================

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColumnType {
    Unknown = 0,
    Float64 = 1,
    Int64 = 2,
    Boolean = 3,
    String = 4,
}

pub struct Column {
    pub name: Vec<u8>,
    pub col_type: ColumnType,
    pub data: Vec<u8>,
    pub offsets: Vec<u32>,
}

pub struct Table {
    pub row_count: usize,
    pub columns: Vec<Column>,
}

static mut FILTER_BUFFER: Vec<u32> = Vec::new();

fn parse_table_from_slice(data: &[u8]) -> Result<Box<Table>, i32> {
    if data.len() < 6 || data[0] != b'B' || data[1] != b'F' {
        return Err(STATUS_ERROR);
    }

    let string_table_count = u16::from_le_bytes(data[4..6].try_into().unwrap()) as usize;
    let mut offset = 6;
    let mut string_table: Vec<&[u8]> = Vec::with_capacity(string_table_count);

    for _ in 0..string_table_count {
        if offset >= data.len() {
            return Err(STATUS_ERROR);
        }
        let length = data[offset] as usize;
        offset += 1;
        if offset + length > data.len() {
            return Err(STATUS_ERROR);
        }
        string_table.push(&data[offset..offset + length]);
        offset += length;
    }

    if offset >= data.len() {
        return Err(STATUS_ERROR);
    }

    let payload_tag = data[offset];
    offset += 1;

    if payload_tag == TAG_LIST {
        // List of records / objects
        if offset + 4 > data.len() {
            return Err(STATUS_ERROR);
        }
        let row_count = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if row_count == 0 {
            return Ok(Box::new(Table {
                row_count: 0,
                columns: Vec::new(),
            }));
        }

        let first_row_start = offset;
        if offset >= data.len() || data[offset] != TAG_OBJECT {
            return Err(STATUS_ERROR);
        }
        offset += 1;

        // Inspect first row schema
        let mut col_field_ids: Vec<u32> = Vec::new();
        let mut columns: Vec<Column> = Vec::new();

        while offset < data.len() && data[offset] != TAG_OBJECT_END {
            if offset + 4 > data.len() {
                return Err(STATUS_ERROR);
            }
            let field_id = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            offset += 4;

            if offset >= data.len() {
                return Err(STATUS_ERROR);
            }
            let val_tag = data[offset];
            offset += 1;

            let col_type = match val_tag {
                TAG_FLOAT64 => {
                    offset += 8;
                    ColumnType::Float64
                }
                TAG_INT64 => {
                    offset += 8;
                    ColumnType::Int64
                }
                t if (t & 0xF0) == 0x30 => ColumnType::Int64,
                TAG_FALSE | TAG_TRUE => ColumnType::Boolean,
                TAG_STRING => {
                    if offset + 4 > data.len() {
                        return Err(STATUS_ERROR);
                    }
                    let s_len =
                        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4 + s_len;
                    ColumnType::String
                }
                TAG_BYTES | TAG_DATETIME => {
                    if offset + 4 > data.len() {
                        return Err(STATUS_ERROR);
                    }
                    let s_len =
                        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4 + s_len;
                    ColumnType::String
                }
                TAG_UUID => {
                    if offset + 4 > data.len() {
                        return Err(STATUS_ERROR);
                    }
                    let s_len =
                        u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4 + s_len;
                    ColumnType::String
                }
                _ => ColumnType::Unknown,
            };

            let name = if (field_id as usize) < string_table.len() {
                string_table[field_id as usize].to_vec()
            } else {
                Vec::new()
            };

            col_field_ids.push(field_id);
            let mut col = Column {
                name,
                col_type,
                data: Vec::new(),
                offsets: Vec::new(),
            };
            match col_type {
                ColumnType::Float64 | ColumnType::Int64 => col.data.reserve(row_count * 8),
                ColumnType::Boolean => col.data.reserve(row_count),
                ColumnType::String => {
                    col.offsets.reserve(row_count + 1);
                    col.offsets.push(0);
                }
                _ => {}
            }
            columns.push(col);
        }

        // Fast row parsing
        let mut cur_offset = first_row_start;
        let num_cols = columns.len();

        for _ in 0..row_count {
            if cur_offset >= data.len() || data[cur_offset] != TAG_OBJECT {
                return Err(STATUS_ERROR);
            }
            cur_offset += 1;

            let mut col_written = vec![false; num_cols];

            while cur_offset < data.len() && data[cur_offset] != TAG_OBJECT_END {
                if cur_offset + 4 > data.len() {
                    return Err(STATUS_ERROR);
                }
                let field_id =
                    u32::from_le_bytes(data[cur_offset..cur_offset + 4].try_into().unwrap());
                cur_offset += 4;

                if cur_offset >= data.len() {
                    return Err(STATUS_ERROR);
                }
                let val_tag = data[cur_offset];
                cur_offset += 1;

                let col_idx = col_field_ids.iter().position(|&id| id == field_id);
                if let Some(idx) = col_idx {
                    col_written[idx] = true;
                    let col = &mut columns[idx];
                    match col.col_type {
                        ColumnType::Float64 => match val_tag {
                            TAG_FLOAT64 => {
                                if cur_offset + 8 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                col.data
                                    .extend_from_slice(&data[cur_offset..cur_offset + 8]);
                                cur_offset += 8;
                            }
                            TAG_INT64 => {
                                if cur_offset + 8 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let val = i64::from_le_bytes(
                                    data[cur_offset..cur_offset + 8].try_into().unwrap(),
                                );
                                cur_offset += 8;
                                let f = val as f64;
                                col.data.extend_from_slice(&f.to_le_bytes());
                            }
                            t if (t & 0xF0) == 0x30 => {
                                let f = (t & 0x07) as f64;
                                col.data.extend_from_slice(&f.to_le_bytes());
                            }
                            TAG_NULL => {
                                col.data.extend_from_slice(&f64::NAN.to_le_bytes());
                            }
                            TAG_STRING | TAG_BYTES | TAG_DATETIME => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.extend_from_slice(&f64::NAN.to_le_bytes());
                            }
                            TAG_UUID => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.extend_from_slice(&f64::NAN.to_le_bytes());
                            }
                            _ => {
                                col.data.extend_from_slice(&f64::NAN.to_le_bytes());
                            }
                        },
                        ColumnType::Int64 => match val_tag {
                            TAG_INT64 => {
                                if cur_offset + 8 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                col.data
                                    .extend_from_slice(&data[cur_offset..cur_offset + 8]);
                                cur_offset += 8;
                            }
                            t if (t & 0xF0) == 0x30 => {
                                let n = (t & 0x07) as i64;
                                col.data.extend_from_slice(&n.to_le_bytes());
                            }
                            TAG_FLOAT64 => {
                                // Dynamic promotion: column was int64, but encountered a float!
                                col.col_type = ColumnType::Float64;
                                for chunk in col.data.chunks_exact_mut(8) {
                                    let int_val = i64::from_le_bytes(chunk.try_into().unwrap());
                                    let float_val = int_val as f64;
                                    chunk.copy_from_slice(&float_val.to_le_bytes());
                                }
                                if cur_offset + 8 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                col.data
                                    .extend_from_slice(&data[cur_offset..cur_offset + 8]);
                                cur_offset += 8;
                            }
                            TAG_NULL => {
                                col.data.extend_from_slice(&0i64.to_le_bytes());
                            }
                            TAG_STRING | TAG_BYTES | TAG_DATETIME => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.extend_from_slice(&0i64.to_le_bytes());
                            }
                            TAG_UUID => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.extend_from_slice(&0i64.to_le_bytes());
                            }
                            _ => {
                                col.data.extend_from_slice(&0i64.to_le_bytes());
                            }
                        },
                        ColumnType::Boolean => match val_tag {
                            TAG_TRUE => {
                                col.data.push(1);
                            }
                            TAG_FALSE | TAG_NULL => {
                                col.data.push(0);
                            }
                            TAG_FLOAT64 | TAG_INT64 => {
                                cur_offset += 8;
                                col.data.push(0);
                            }
                            TAG_STRING | TAG_BYTES | TAG_DATETIME => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.push(0);
                            }
                            TAG_UUID => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                                col.data.push(0);
                            }
                            _ => {
                                col.data.push(0);
                            }
                        },
                        ColumnType::String => match val_tag {
                            TAG_STRING => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let s_len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4;
                                if cur_offset + s_len > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                col.data
                                    .extend_from_slice(&data[cur_offset..cur_offset + s_len]);
                                col.offsets.push(col.data.len() as u32);
                                cur_offset += s_len;
                            }
                            TAG_BYTES | TAG_DATETIME => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4;
                                if cur_offset + len > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                col.data
                                    .extend_from_slice(&data[cur_offset..cur_offset + len]);
                                col.offsets.push(col.data.len() as u32);
                                cur_offset += len;
                            }
                            TAG_UUID => {
                                if cur_offset + 4 > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4;
                                if cur_offset + len > data.len() {
                                    return Err(STATUS_ERROR);
                                }
                                let hex = &data[cur_offset..cur_offset + len];
                                cur_offset += len;
                                format_uuid_bytes(&mut col.data, hex);
                                col.offsets.push(col.data.len() as u32);
                            }
                            TAG_FLOAT64 | TAG_INT64 => {
                                cur_offset += 8;
                                col.offsets.push(col.data.len() as u32);
                            }
                            _ => {
                                col.offsets.push(col.data.len() as u32);
                            }
                        },
                        _ => match val_tag {
                            TAG_FLOAT64 | TAG_INT64 => cur_offset += 8,
                            TAG_STRING | TAG_BYTES | TAG_DATETIME => {
                                if cur_offset + 4 <= data.len() {
                                    let len = u32::from_le_bytes(
                                        data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                    ) as usize;
                                    cur_offset += 4 + len;
                                }
                            }
                            TAG_UUID => {
                                if cur_offset + 4 <= data.len() {
                                    let len = u32::from_le_bytes(
                                        data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                    ) as usize;
                                    cur_offset += 4 + len;
                                }
                            }
                            _ => {}
                        },
                    }
                } else {
                    match val_tag {
                        TAG_FLOAT64 | TAG_INT64 => cur_offset += 8,
                        t if (t & 0xF0) == 0x30
                            || t == TAG_FALSE
                            || t == TAG_TRUE
                            || t == TAG_NULL => {}
                        TAG_STRING | TAG_BYTES | TAG_DATETIME => {
                            if cur_offset + 4 <= data.len() {
                                let s_len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + s_len;
                            }
                        }
                        TAG_UUID => {
                            if cur_offset + 4 <= data.len() {
                                let len = u32::from_le_bytes(
                                    data[cur_offset..cur_offset + 4].try_into().unwrap(),
                                ) as usize;
                                cur_offset += 4 + len;
                            }
                        }
                        _ => {}
                    }
                }
            }

            if cur_offset < data.len() && data[cur_offset] == TAG_OBJECT_END {
                cur_offset += 1;
            }

            // Fill default for any unwritten columns in this row
            for (idx, &written) in col_written.iter().enumerate() {
                if !written {
                    let col = &mut columns[idx];
                    match col.col_type {
                        ColumnType::Float64 => col.data.extend_from_slice(&f64::NAN.to_le_bytes()),
                        ColumnType::Int64 => col.data.extend_from_slice(&0i64.to_le_bytes()),
                        ColumnType::Boolean => col.data.push(0),
                        ColumnType::String => col.offsets.push(col.data.len() as u32),
                        _ => {}
                    }
                }
            }
        }

        Ok(Box::new(Table { row_count, columns }))
    } else {
        Err(STATUS_ERROR)
    }
}

#[no_mangle]
pub extern "C" fn table_create(src_ptr: *const u8, src_len: usize) -> *mut Table {
    let src = unsafe { std::slice::from_raw_parts(src_ptr, src_len) };
    if src.len() < 2 {
        return std::ptr::null_mut();
    }

    let data: &[u8] = if src[0] == b'B' && src[1] == b'F' {
        src
    } else {
        if src.len() < 4 {
            return std::ptr::null_mut();
        }
        let uncompressed_size = match src[0..4].try_into() {
            Ok(b) => u32::from_le_bytes(b) as usize,
            Err(_) => return std::ptr::null_mut(),
        };
        let decompress_buf = unsafe { &mut DECOMPRESS_BUFFER };
        decompress_buf.clear();
        decompress_buf.resize(uncompressed_size, 0);

        if let Ok(decompressed_len) = decompress_into(&src[4..], decompress_buf.as_mut_slice()) {
            decompress_buf.truncate(decompressed_len);
            decompress_buf.as_slice()
        } else if src.len() >= 8 {
            let ret = decompress_parallel(
                src.as_ptr(),
                src.len(),
                decompress_buf.as_mut_ptr(),
                uncompressed_size,
            );
            if ret >= 0 {
                decompress_buf.truncate(ret as usize);
                decompress_buf.as_slice()
            } else {
                return std::ptr::null_mut();
            }
        } else {
            return std::ptr::null_mut();
        }
    };

    match parse_table_from_slice(data) {
        Ok(table) => Box::into_raw(table),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn table_destroy(table_ptr: *mut Table) {
    if !table_ptr.is_null() {
        unsafe {
            drop(Box::from_raw(table_ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn table_row_count(table_ptr: *const Table) -> usize {
    if table_ptr.is_null() {
        0
    } else {
        unsafe { (*table_ptr).row_count }
    }
}

#[no_mangle]
pub extern "C" fn table_column_count(table_ptr: *const Table) -> usize {
    if table_ptr.is_null() {
        0
    } else {
        unsafe { (*table_ptr).columns.len() }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_name_ptr(table_ptr: *const Table, col_idx: usize) -> *const u8 {
    if table_ptr.is_null() {
        std::ptr::null()
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].name.as_ptr()
        } else {
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_name_len(table_ptr: *const Table, col_idx: usize) -> usize {
    if table_ptr.is_null() {
        0
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].name.len()
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_type(table_ptr: *const Table, col_idx: usize) -> u8 {
    if table_ptr.is_null() {
        0
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].col_type as u8
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_data_ptr(table_ptr: *const Table, col_idx: usize) -> *const u8 {
    if table_ptr.is_null() {
        std::ptr::null()
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].data.as_ptr()
        } else {
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_data_len(table_ptr: *const Table, col_idx: usize) -> usize {
    if table_ptr.is_null() {
        0
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].data.len()
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_offsets_ptr(
    table_ptr: *const Table,
    col_idx: usize,
) -> *const u32 {
    if table_ptr.is_null() {
        std::ptr::null()
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].offsets.as_ptr()
        } else {
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn table_get_column_offsets_len(table_ptr: *const Table, col_idx: usize) -> usize {
    if table_ptr.is_null() {
        0
    } else {
        let table = unsafe { &*table_ptr };
        if col_idx < table.columns.len() {
            table.columns[col_idx].offsets.len()
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn table_aggregate_f64(
    table_ptr: *const Table,
    col_idx: usize,
    agg_type: u8,
) -> f64 {
    if table_ptr.is_null() {
        return f64::NAN;
    }
    let table = unsafe { &*table_ptr };
    if col_idx >= table.columns.len() {
        return f64::NAN;
    }
    let col = &table.columns[col_idx];
    if col.col_type != ColumnType::Float64 || col.data.len() < 8 {
        return f64::NAN;
    }

    let f64_slice: &[f64] =
        unsafe { std::slice::from_raw_parts(col.data.as_ptr() as *const f64, col.data.len() / 8) };

    match agg_type {
        1 => {
            // SUM
            f64_slice.iter().filter(|v| !v.is_nan()).sum()
        }
        2 => {
            // AVG
            let mut sum = 0.0;
            let mut count = 0usize;
            for &v in f64_slice {
                if !v.is_nan() {
                    sum += v;
                    count += 1;
                }
            }
            if count > 0 {
                sum / (count as f64)
            } else {
                f64::NAN
            }
        }
        3 => {
            // MIN
            f64_slice
                .iter()
                .filter(|v| !v.is_nan())
                .fold(f64::INFINITY, |acc, &x| acc.min(x))
        }
        4 => {
            // MAX
            f64_slice
                .iter()
                .filter(|v| !v.is_nan())
                .fold(f64::NEG_INFINITY, |acc, &x| acc.max(x))
        }
        5 => {
            // COUNT
            f64_slice.iter().filter(|v| !v.is_nan()).count() as f64
        }
        _ => f64::NAN,
    }
}

#[no_mangle]
pub extern "C" fn table_aggregate_i64(
    table_ptr: *const Table,
    col_idx: usize,
    agg_type: u8,
) -> i64 {
    if table_ptr.is_null() {
        return 0;
    }
    let table = unsafe { &*table_ptr };
    if col_idx >= table.columns.len() {
        return 0;
    }
    let col = &table.columns[col_idx];
    if col.col_type != ColumnType::Int64 || col.data.len() < 8 {
        return 0;
    }

    let i64_slice: &[i64] =
        unsafe { std::slice::from_raw_parts(col.data.as_ptr() as *const i64, col.data.len() / 8) };

    match agg_type {
        1 => {
            // SUM
            i64_slice.iter().sum()
        }
        2 => {
            // AVG
            if !i64_slice.is_empty() {
                i64_slice.iter().sum::<i64>() / (i64_slice.len() as i64)
            } else {
                0
            }
        }
        3 => {
            // MIN
            i64_slice.iter().copied().min().unwrap_or(0)
        }
        4 => {
            // MAX
            i64_slice.iter().copied().max().unwrap_or(0)
        }
        5 => {
            // COUNT
            i64_slice.len() as i64
        }
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn table_filter_f64(
    table_ptr: *const Table,
    col_idx: usize,
    op: u8,
    target: f64,
) -> usize {
    if table_ptr.is_null() {
        return 0;
    }
    let table = unsafe { &*table_ptr };
    if col_idx >= table.columns.len() {
        return 0;
    }
    let col = &table.columns[col_idx];
    if col.col_type != ColumnType::Float64 || col.data.len() < 8 {
        return 0;
    }

    let f64_slice: &[f64] =
        unsafe { std::slice::from_raw_parts(col.data.as_ptr() as *const f64, col.data.len() / 8) };

    let filter_buf = unsafe { &mut FILTER_BUFFER };
    filter_buf.clear();
    filter_buf.reserve(f64_slice.len());

    for (i, &val) in f64_slice.iter().enumerate() {
        let matches = match op {
            1 => (val - target).abs() < f64::EPSILON,  // ==
            2 => (val - target).abs() >= f64::EPSILON, // !=
            3 => val < target,                         // <
            4 => val <= target,                        // <=
            5 => val > target,                         // >
            6 => val >= target,                        // >=
            _ => false,
        };
        if matches {
            filter_buf.push(i as u32);
        }
    }

    filter_buf.len()
}

#[no_mangle]
pub extern "C" fn table_get_filter_indices_ptr() -> *const u32 {
    unsafe { FILTER_BUFFER.as_ptr() }
}
