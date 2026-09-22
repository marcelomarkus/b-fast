use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::BFast;

pub const STREAM_MAGIC: [u8; 2] = [0x42, 0x53]; // 'BS'
pub const STREAM_VERSION: u8 = 0x01;

pub const FRAME_EOS: u8 = 0x00;
pub const FRAME_DATA: u8 = 0x01;
pub const FRAME_DICT_DELTA: u8 = 0x02;

pub const DEFAULT_MAX_FRAME_SIZE: usize = 64 * 1024 * 1024; // 64 MB

#[pyclass]
pub struct BFastStreamDecoder {
    buffer: Vec<u8>,
    read_pos: usize,
    handshake_received: bool,
    expect_handshake: Option<bool>,
    max_frame_size: usize,
    is_eos: bool,
    b_fast: BFast,
}

#[pymethods]
impl BFastStreamDecoder {
    #[new]
    #[pyo3(signature = (*, max_frame_size = DEFAULT_MAX_FRAME_SIZE, expect_handshake = None))]
    pub fn new(max_frame_size: usize, expect_handshake: Option<bool>) -> Self {
        let handshake_received = matches!(expect_handshake, Some(false));
        BFastStreamDecoder {
            buffer: Vec::with_capacity(8192),
            read_pos: 0,
            handshake_received,
            expect_handshake,
            max_frame_size,
            is_eos: false,
            b_fast: BFast::new(),
        }
    }

    pub fn feed(&mut self, py: Python, chunk: &[u8]) -> PyResult<Vec<PyObject>> {
        if chunk.is_empty() {
            return Ok(Vec::new());
        }

        // Amortized compaction of the read buffer
        if self.read_pos > 0 {
            if self.read_pos == self.buffer.len() {
                self.buffer.clear();
                self.read_pos = 0;
            } else if self.read_pos >= 32768 && self.read_pos >= self.buffer.len() / 2 {
                self.buffer.drain(0..self.read_pos);
                self.read_pos = 0;
            }
        }

        self.buffer.extend_from_slice(chunk);
        let mut results = Vec::new();

        // 1. Process Stream Handshake (if not yet processed)
        if !self.handshake_received {
            let available = self.buffer.len() - self.read_pos;
            if available >= 2 {
                if self.buffer[self.read_pos..self.read_pos + 2] == STREAM_MAGIC {
                    if available < 4 {
                        // Handshake detected but incomplete; await remainder of 4-byte header
                        return Ok(results);
                    }
                    let version = self.buffer[self.read_pos + 2];
                    if version != STREAM_VERSION {
                        return Err(PyValueError::new_err(format!(
                            "Unsupported stream version: {} (expected {})",
                            version, STREAM_VERSION
                        )));
                    }
                    // Flags byte at self.read_pos + 3 is reserved
                    self.read_pos += 4;
                    self.handshake_received = true;
                } else if self.expect_handshake == Some(true) {
                    return Err(PyValueError::new_err(
                        "Expected stream handshake 'BS' not found",
                    ));
                } else {
                    // Not starting with 'BS' and handshake not strictly required: assume raw frames
                    self.handshake_received = true;
                }
            }
        }

        // 2. Process Length-Prefixed Frames
        while !self.is_eos {
            let available = self.buffer.len() - self.read_pos;
            if available < 6 {
                // Incomplete frame header (needs 4B length + 1B type + 1B flags)
                break;
            }

            let payload_len = u32::from_le_bytes(
                self.buffer[self.read_pos..self.read_pos + 4]
                    .try_into()
                    .unwrap(),
            ) as usize;
            let frame_type = self.buffer[self.read_pos + 4];
            let flags = self.buffer[self.read_pos + 5];

            // Security check against out-of-memory DoS attacks
            if payload_len > self.max_frame_size {
                let err_msg = format!(
                    "Frame size {} exceeds maximum allowed {}",
                    payload_len, self.max_frame_size
                );
                self.clear();
                return Err(PyValueError::new_err(err_msg));
            }

            // End-of-Stream frame
            if frame_type == FRAME_EOS {
                self.read_pos += 6 + payload_len;
                self.is_eos = true;
                break;
            }

            if available < 6 + payload_len {
                // Incomplete frame payload, wait for next network chunk
                break;
            }

            let frame_start = self.read_pos + 6;
            let frame_end = frame_start + payload_len;
            let frame_bytes = &self.buffer[frame_start..frame_end];

            if frame_type == FRAME_DATA {
                let is_compressed = (flags & 0x01) != 0;
                let decoded_obj = self.b_fast.decode_packed(py, frame_bytes, is_compressed)?;
                results.push(decoded_obj);
            } else if frame_type == FRAME_DICT_DELTA {
                // Reserved for future progressive dictionary delta frames
            } else {
                let err_msg = format!("Unknown frame type: 0x{:02x}", frame_type);
                self.clear();
                return Err(PyValueError::new_err(err_msg));
            }

            self.read_pos = frame_end;
        }

        Ok(results)
    }

    #[getter]
    pub fn is_eos(&self) -> bool {
        self.is_eos
    }

    #[getter]
    pub fn pending_bytes(&self) -> usize {
        self.buffer.len() - self.read_pos
    }

    #[getter]
    pub fn handshake_received(&self) -> bool {
        self.handshake_received
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.handshake_received = matches!(self.expect_handshake, Some(false));
        self.is_eos = false;
    }

    pub fn reset(&mut self) {
        self.clear();
    }
}

#[pyclass]
pub struct BFastStreamEncoder {
    b_fast: BFast,
}

impl Default for BFastStreamEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl BFastStreamEncoder {
    #[new]
    pub fn new() -> Self {
        BFastStreamEncoder {
            b_fast: BFast::new(),
        }
    }

    #[pyo3(signature = (obj, *, compress = true))]
    pub fn encode_frame(&mut self, obj: &PyAny, compress: bool) -> PyResult<PyObject> {
        self.b_fast.encode_frame(obj, compress)
    }

    #[staticmethod]
    pub fn get_handshake(py: Python) -> PyResult<PyObject> {
        BFast::get_stream_handshake(py)
    }

    #[staticmethod]
    pub fn get_eos_frame(py: Python) -> PyResult<PyObject> {
        BFast::get_eos_frame(py)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_constants() {
        assert_eq!(STREAM_MAGIC, [0x42, 0x53]);
        assert_eq!(STREAM_VERSION, 0x01);
        assert_eq!(FRAME_EOS, 0x00);
        assert_eq!(FRAME_DATA, 0x01);
        assert_eq!(FRAME_DICT_DELTA, 0x02);
        assert_eq!(DEFAULT_MAX_FRAME_SIZE, 64 * 1024 * 1024);
    }
}
