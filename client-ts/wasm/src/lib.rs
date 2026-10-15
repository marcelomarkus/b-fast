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
                let val = i64::from_le_bytes(
                    self.data[self.offset..self.offset + 8].try_into().unwrap(),
                );
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
                let val = f64::from_le_bytes(
                    self.data[self.offset..self.offset + 8].try_into().unwrap(),
                );
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
                let len = u32::from_le_bytes(
                    self.data[self.offset..self.offset + 4].try_into().unwrap(),
                ) as usize;
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
                let len = u32::from_le_bytes(
                    self.data[self.offset..self.offset + 4].try_into().unwrap(),
                ) as usize;
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
                let len = u32::from_le_bytes(
                    self.data[self.offset..self.offset + 4].try_into().unwrap(),
                ) as usize;
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
                let len = u32::from_le_bytes(
                    self.data[self.offset..self.offset + 4].try_into().unwrap(),
                ) as usize;
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
                let len = u32::from_le_bytes(
                    self.data[self.offset..self.offset + 4].try_into().unwrap(),
                ) as usize;
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
