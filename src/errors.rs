use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BFastError {
    #[error("Invalid magic number: expected 'BF'")]
    InvalidMagic,
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),
    #[error("LZ4 decompression failed")]
    DecompressionFailed,
    #[error("Unexpected end of stream at offset {0}")]
    UnexpectedEOF(usize),
    #[error("String too long for header: {0} (max 255 bytes)")]
    StringTooLong(String),
}

impl From<BFastError> for PyErr {
    fn from(err: BFastError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}
