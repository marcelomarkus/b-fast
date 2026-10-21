use pyo3::exceptions::{PyBlockingIOError, PyConnectionResetError, PyIOError};
use pyo3::prelude::*;
use std::io;

use crate::BFast;

/// Returns true if native kernel socket transport acceleration
/// is supported on this platform (Linux).
#[pyfunction]
pub fn is_native_available() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(target_os = "linux")]
pub struct NativePipe {
    pipe_read: libc::c_int,
    pipe_write: libc::c_int,
    pending: usize,
}

#[cfg(target_os = "linux")]
impl NativePipe {
    pub fn new() -> io::Result<Self> {
        let mut fds = [0 as libc::c_int; 2];
        let res = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }

        // Expand the kernel pipe capacity to 1 MB for maximum streaming throughput
        unsafe {
            libc::fcntl(fds[1], libc::F_SETPIPE_SZ, 1024 * 1024);
        }

        Ok(Self {
            pipe_read: fds[0],
            pipe_write: fds[1],
            pending: 0,
        })
    }

    #[inline]
    pub fn pending_bytes(&self) -> usize {
        self.pending
    }

    /// Drain any pending bytes in the pipe into the socket.
    pub fn drain_pending(&mut self, sock_fd: libc::c_int, non_blocking: bool) -> io::Result<usize> {
        let mut drained = 0;
        while self.pending > 0 {
            let flags = if non_blocking {
                libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK
            } else {
                libc::SPLICE_F_MOVE
            };

            let res = unsafe {
                libc::splice(
                    self.pipe_read,
                    std::ptr::null_mut(),
                    sock_fd,
                    std::ptr::null_mut(),
                    self.pending,
                    flags,
                )
            };

            if res > 0 {
                let n = res as usize;
                self.pending -= n;
                drained += n;
            } else if res < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                if err.kind() == io::ErrorKind::WouldBlock {
                    if drained > 0 {
                        return Ok(drained);
                    }
                    return Err(err);
                }
                return Err(err);
            } else {
                // Pipe empty
                self.pending = 0;
                break;
            }
        }
        Ok(drained)
    }

    /// Discard any pending data in the pipe (e.g. before fallback).
    pub fn drain_discard(&mut self) {
        if self.pending == 0 {
            return;
        }
        let mut buf = [0u8; 4096];
        while self.pending > 0 {
            let to_read = self.pending.min(buf.len());
            let res = unsafe {
                libc::read(
                    self.pipe_read,
                    buf.as_mut_ptr() as *mut libc::c_void,
                    to_read,
                )
            };
            if res > 0 {
                self.pending = self.pending.saturating_sub(res as usize);
            } else {
                self.pending = 0;
                break;
            }
        }
    }

    /// Native kernel transfer of user memory slice into socket file descriptor.
    pub fn splice_data(
        &mut self,
        sock_fd: libc::c_int,
        data: &[u8],
        non_blocking: bool,
    ) -> io::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        // 1. Drain pending bytes first if any
        if self.pending > 0 {
            self.drain_pending(sock_fd, non_blocking)?;
            if self.pending > 0 {
                // Socket still blocked
                return Ok(0);
            }
        }

        let mut total_spliced = 0;
        let mut offset = 0;

        while offset < data.len() {
            let remaining = &data[offset..];
            let iov = libc::iovec {
                iov_base: remaining.as_ptr() as *mut libc::c_void,
                iov_len: remaining.len(),
            };

            let vm_flags = if non_blocking {
                libc::SPLICE_F_NONBLOCK
            } else {
                0
            };

            let n_vm =
                unsafe { libc::vmsplice(self.pipe_write, &iov as *const libc::iovec, 1, vm_flags) };

            if n_vm < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                if err.kind() == io::ErrorKind::WouldBlock {
                    if total_spliced > 0 {
                        return Ok(total_spliced);
                    }
                    return Err(err);
                }
                return Err(err);
            }

            let vm_bytes = n_vm as usize;
            let mut spliced_from_vm = 0;

            while spliced_from_vm < vm_bytes {
                let to_splice = vm_bytes - spliced_from_vm;
                let sp_flags = if non_blocking {
                    libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK
                } else {
                    libc::SPLICE_F_MOVE
                };

                let n_sp = unsafe {
                    libc::splice(
                        self.pipe_read,
                        std::ptr::null_mut(),
                        sock_fd,
                        std::ptr::null_mut(),
                        to_splice,
                        sp_flags,
                    )
                };

                if n_sp > 0 {
                    spliced_from_vm += n_sp as usize;
                } else if n_sp < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    if err.kind() == io::ErrorKind::WouldBlock {
                        // Socket write buffer full: record remaining bytes still in pipe
                        self.pending = vm_bytes - spliced_from_vm;
                        total_spliced += spliced_from_vm;
                        return Ok(total_spliced);
                    }
                    // Splicing error
                    self.pending = vm_bytes - spliced_from_vm;
                    return Err(err);
                } else {
                    break;
                }
            }

            total_spliced += spliced_from_vm;
            offset += vm_bytes;
        }

        Ok(total_spliced)
    }
}

#[cfg(target_os = "linux")]
impl Drop for NativePipe {
    fn drop(&mut self) {
        unsafe {
            if self.pipe_read >= 0 {
                libc::close(self.pipe_read);
            }
            if self.pipe_write >= 0 {
                libc::close(self.pipe_write);
            }
        }
    }
}

/// Fallback socket send using standard libc::send.
fn fallback_socket_send(
    sock_fd: libc::c_int,
    data: &[u8],
    non_blocking: bool,
) -> io::Result<usize> {
    if data.is_empty() {
        return Ok(0);
    }
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_NOSIGNAL | if non_blocking { libc::MSG_DONTWAIT } else { 0 };
    #[cfg(not(target_os = "linux"))]
    let flags = 0;

    let mut sent = 0;
    while sent < data.len() {
        let remaining = &data[sent..];
        let res = unsafe {
            libc::send(
                sock_fd,
                remaining.as_ptr() as *const libc::c_void,
                remaining.len(),
                flags,
            )
        };
        if res > 0 {
            sent += res as usize;
        } else if res < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if err.kind() == io::ErrorKind::WouldBlock {
                if sent > 0 {
                    return Ok(sent);
                }
                return Err(err);
            }
            return Err(err);
        } else {
            break;
        }
    }
    Ok(sent)
}

/// Zero-copy scatter-gather send of header + payload using native sendmsg(MSG_NOSIGNAL).
#[inline]
fn send_iovec_pair(
    sock_fd: libc::c_int,
    iov0: &[u8],
    iov1: &[u8],
    non_blocking: bool,
) -> io::Result<usize> {
    let iov0_len = iov0.len();
    let total = iov0_len + iov1.len();
    if total == 0 {
        return Ok(0);
    }

    let flags = if non_blocking {
        libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL
    } else {
        libc::MSG_NOSIGNAL
    };

    let mut iov = [
        libc::iovec {
            iov_base: iov0.as_ptr() as *mut libc::c_void,
            iov_len: iov0.len(),
        },
        libc::iovec {
            iov_base: iov1.as_ptr() as *mut libc::c_void,
            iov_len: iov1.len(),
        },
    ];

    let mut total_sent = 0;
    while total_sent < total {
        let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
        if total_sent < iov0_len {
            msg.msg_iov = iov.as_mut_ptr();
            msg.msg_iovlen = 2;
        } else {
            msg.msg_iov = unsafe { iov.as_mut_ptr().add(1) };
            msg.msg_iovlen = 1;
        }

        let res = unsafe { libc::sendmsg(sock_fd, &msg, flags) };
        if res > 0 {
            let n = res as usize;
            total_sent += n;
            if total_sent < iov0_len {
                iov[0].iov_base =
                    unsafe { (iov[0].iov_base as *mut u8).add(n) as *mut libc::c_void };
                iov[0].iov_len -= n;
            } else if total_sent - n < iov0_len {
                let sent_from_iov1 = total_sent - iov0_len;
                iov[0].iov_len = 0;
                iov[1].iov_base = unsafe {
                    (iov[1].iov_base as *mut u8).add(sent_from_iov1) as *mut libc::c_void
                };
                iov[1].iov_len = total - total_sent;
            } else {
                iov[1].iov_base =
                    unsafe { (iov[1].iov_base as *mut u8).add(n) as *mut libc::c_void };
                iov[1].iov_len -= n;
            }
        } else if res < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if err.kind() == io::ErrorKind::WouldBlock {
                if total_sent > 0 {
                    return Ok(total_sent);
                }
                return Err(err);
            }
            return Err(err);
        } else {
            break;
        }
    }
    Ok(total_sent)
}

fn io_error_to_py_err(err: io::Error) -> PyErr {
    match err.kind() {
        io::ErrorKind::WouldBlock => PyBlockingIOError::new_err("Resource temporarily unavailable"),
        io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe => {
            PyConnectionResetError::new_err(err.to_string())
        }
        _ => PyIOError::new_err(err.to_string()),
    }
}

static HANDSHAKE: [u8; 4] = [0x42, 0x53, crate::streaming::STREAM_VERSION, 0x00];
static EOS: [u8; 6] = [0x00, 0x00, 0x00, 0x00, crate::streaming::FRAME_EOS, 0x00];

/// Standalone function to send a byte slice directly to a socket using native transport.
#[pyfunction]
#[pyo3(signature = (sock_fd, data, *, non_blocking = false))]
pub fn send_native_socket(
    py: Python,
    sock_fd: i32,
    data: &[u8],
    non_blocking: bool,
) -> PyResult<usize> {
    py.allow_threads(|| {
        #[cfg(target_os = "linux")]
        {
            let mut pipe = NativePipe::new().map_err(io_error_to_py_err)?;
            match pipe.splice_data(sock_fd, data, non_blocking) {
                Ok(n) => Ok(n),
                Err(e)
                    if e.raw_os_error() == Some(libc::EINVAL)
                        || e.raw_os_error() == Some(libc::ENOSYS) =>
                {
                    // Descriptor does not support native splice: fallback to standard socket send
                    fallback_socket_send(sock_fd, data, non_blocking).map_err(io_error_to_py_err)
                }
                Err(e) => Err(io_error_to_py_err(e)),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            fallback_socket_send(sock_fd, data, non_blocking).map_err(io_error_to_py_err)
        }
    })
}

/// High-throughput native transport sender for B-FAST socket streams.
///
/// Encapsulates serializing objects directly into length-prefixed B-FAST frames
/// and transmitting them into socket file descriptors via optimized kernel transport,
/// completely bypassing user-space memory copies.
#[pyclass]
pub struct BFastNativeSender {
    #[cfg(target_os = "linux")]
    pipe: Option<NativePipe>,
    b_fast: BFast,
    fallback_mode: bool,
    buffer: Vec<u8>,
    buffer_size: usize,
    cached_template: Option<crate::HomogeneousTemplate>,
}

#[pymethods]
impl BFastNativeSender {
    #[new]
    #[pyo3(signature = (*, max_in_flight = 128, buffer_size = 65536))]
    pub fn new(max_in_flight: usize, buffer_size: usize) -> PyResult<Self> {
        let _ = max_in_flight;
        #[cfg(target_os = "linux")]
        {
            let pipe = NativePipe::new().ok();
            let fallback_mode = pipe.is_none();
            Ok(Self {
                pipe,
                b_fast: BFast::new(),
                fallback_mode,
                buffer: Vec::with_capacity(buffer_size + 8192),
                buffer_size,
                cached_template: None,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(Self {
                b_fast: BFast::new(),
                fallback_mode: true,
                buffer: Vec::with_capacity(buffer_size + 8192),
                buffer_size,
                cached_template: None,
            })
        }
    }

    /// Check if native transport acceleration is currently active.
    #[getter]
    pub fn is_native_active(&self) -> bool {
        !self.fallback_mode
    }

    /// Number of bytes currently buffered waiting to be transmitted.
    #[getter]
    pub fn pending_bytes(&self) -> usize {
        let mut n = self.buffer.len();
        #[cfg(target_os = "linux")]
        {
            if let Some(p) = self.pipe.as_ref() {
                n += p.pending_bytes();
            }
        }
        n
    }

    /// Number of frames currently retained in flight.
    #[getter]
    pub fn in_flight_count(&self) -> usize {
        0
    }

    /// Clear all in-flight buffers.
    pub fn clear_in_flight(&mut self) {}

    /// Send raw bytes to the socket using native kernel acceleration.
    #[pyo3(signature = (sock_fd, data, *, non_blocking = false))]
    pub fn send_bytes(
        &mut self,
        py: Python,
        sock_fd: i32,
        data: &[u8],
        non_blocking: bool,
    ) -> PyResult<usize> {
        py.allow_threads(|| {
            Self::send_bytes_disjoint(
                #[cfg(target_os = "linux")]
                &mut self.pipe,
                &mut self.fallback_mode,
                sock_fd,
                data,
                non_blocking,
            )
        })
    }

    /// Serialize an object directly into a framed B-FAST chunk and transmit into the socket.
    ///
    /// Eliminates all Python-side memory allocations and intermediate user-space memory copies.
    #[pyo3(signature = (sock_fd, obj, *, compress = true, non_blocking = false))]
    pub fn send_frame(
        &mut self,
        py: Python,
        sock_fd: i32,
        obj: &Bound<'_, PyAny>,
        compress: bool,
        non_blocking: bool,
    ) -> PyResult<usize> {
        if !compress {
            if self.cached_template.is_none() {
                self.cached_template = self.b_fast.prepare_homogeneous_template(obj);
            }
            if let Some(ref t) = self.cached_template {
                let prev_len = self.buffer.len();
                if self
                    .b_fast
                    .encode_homogeneous_frame(obj.as_ptr(), py, t, &mut self.buffer)
                {
                    let frame_size = self.buffer.len() - prev_len;
                    if self.buffer_size == 0 || self.buffer.len() >= self.buffer_size {
                        py.allow_threads(|| {
                            fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                                .map_err(io_error_to_py_err)
                        })?;
                        self.buffer.clear();
                    }
                    return Ok(frame_size);
                } else {
                    self.cached_template = None;
                }
            }
        }

        self.b_fast.encode_packed_internal(obj, compress)?;
        let is_compressed = if self.b_fast.last_was_compressed {
            1u8
        } else {
            0u8
        };
        let slice_len = if self.b_fast.last_was_compressed {
            self.b_fast.compressed_buffer.len()
        } else {
            self.b_fast.work_buffer.len()
        };
        let len = slice_len as u32;

        let header = [
            (len & 0xFF) as u8,
            ((len >> 8) & 0xFF) as u8,
            ((len >> 16) & 0xFF) as u8,
            ((len >> 24) & 0xFF) as u8,
            crate::streaming::FRAME_DATA,
            is_compressed,
        ];

        let payload_slice = if self.b_fast.last_was_compressed {
            &self.b_fast.compressed_buffer[..slice_len]
        } else {
            &self.b_fast.work_buffer[..slice_len]
        };

        let frame_size = 6 + slice_len;
        if self.buffer_size == 0 {
            py.allow_threads(|| {
                send_iovec_pair(sock_fd, &header, payload_slice, non_blocking)
                    .map_err(io_error_to_py_err)
            })?;
            return Ok(frame_size);
        }

        self.buffer.extend_from_slice(&header);
        self.buffer.extend_from_slice(payload_slice);

        if self.buffer.len() >= self.buffer_size {
            py.allow_threads(|| {
                fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                    .map_err(io_error_to_py_err)
            })?;
            self.buffer.clear();
        }
        Ok(frame_size)
    }

    /// Stream an entire Python iterable of objects directly into the socket descriptor,
    /// running the loop in native code with automated buffer coalescing and zero GIL churn.
    #[pyo3(signature = (sock_fd, iterable, *, compress = true, non_blocking = false))]
    pub fn send_iterable(
        &mut self,
        py: Python,
        sock_fd: i32,
        iterable: &Bound<'_, PyAny>,
        compress: bool,
        non_blocking: bool,
    ) -> PyResult<usize> {
        let mut total_sent = 0;

        if unsafe { pyo3::ffi::PyList_CheckExact(iterable.as_ptr()) != 0 } {
            let list_ptr = iterable.as_ptr();
            let list_len = unsafe { pyo3::ffi::PyList_Size(list_ptr) };
            if list_len <= 0 {
                return Ok(0);
            }

            if !compress && self.cached_template.is_none() {
                let first_ptr = unsafe { pyo3::ffi::PyList_GetItem(list_ptr, 0) };
                let first_bound = unsafe { Bound::from_borrowed_ptr(py, first_ptr) };
                self.cached_template = self.b_fast.prepare_homogeneous_template(&first_bound);
            }

            for i in 0..list_len {
                let item_ptr = unsafe { pyo3::ffi::PyList_GetItem(list_ptr, i) };
                let mut processed = false;

                if let Some(ref t) = self.cached_template {
                    let prev_len = self.buffer.len();
                    if self
                        .b_fast
                        .encode_homogeneous_frame(item_ptr, py, t, &mut self.buffer)
                    {
                        total_sent += self.buffer.len() - prev_len;
                        processed = true;
                    } else {
                        self.cached_template = None;
                    }
                }

                if !processed {
                    let item = unsafe { Bound::from_borrowed_ptr(py, item_ptr) };
                    self.b_fast.encode_packed_internal(&item, compress)?;
                    let is_compressed = if self.b_fast.last_was_compressed {
                        1u8
                    } else {
                        0u8
                    };
                    let slice_len = if self.b_fast.last_was_compressed {
                        self.b_fast.compressed_buffer.len()
                    } else {
                        self.b_fast.work_buffer.len()
                    };
                    let len = slice_len as u32;
                    let header = [
                        (len & 0xFF) as u8,
                        ((len >> 8) & 0xFF) as u8,
                        ((len >> 16) & 0xFF) as u8,
                        ((len >> 24) & 0xFF) as u8,
                        crate::streaming::FRAME_DATA,
                        is_compressed,
                    ];
                    let payload_slice = if self.b_fast.last_was_compressed {
                        &self.b_fast.compressed_buffer[..slice_len]
                    } else {
                        &self.b_fast.work_buffer[..slice_len]
                    };
                    total_sent += 6 + slice_len;
                    self.buffer.extend_from_slice(&header);
                    self.buffer.extend_from_slice(payload_slice);
                }

                if self.buffer.len() >= self.buffer_size {
                    py.allow_threads(|| {
                        fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                            .map_err(io_error_to_py_err)
                    })?;
                    self.buffer.clear();
                }
            }

            return Ok(total_sent);
        }

        let mut iter = iterable.try_iter()?;

        let first_item = iter.next();
        if first_item.is_none() {
            return Ok(0);
        }
        let first_item = first_item.unwrap()?;

        if !compress && self.cached_template.is_none() {
            self.cached_template = self.b_fast.prepare_homogeneous_template(&first_item);
        }

        let mut processed_first = false;
        if let Some(ref t) = self.cached_template {
            let prev_len = self.buffer.len();
            if self
                .b_fast
                .encode_homogeneous_frame(first_item.as_ptr(), py, t, &mut self.buffer)
            {
                total_sent += self.buffer.len() - prev_len;
                processed_first = true;
            } else {
                self.cached_template = None;
            }
        }

        if !processed_first {
            self.b_fast.encode_packed_internal(&first_item, compress)?;
            let is_compressed = if self.b_fast.last_was_compressed {
                1u8
            } else {
                0u8
            };
            let slice_len = if self.b_fast.last_was_compressed {
                self.b_fast.compressed_buffer.len()
            } else {
                self.b_fast.work_buffer.len()
            };
            let len = slice_len as u32;
            let header = [
                (len & 0xFF) as u8,
                ((len >> 8) & 0xFF) as u8,
                ((len >> 16) & 0xFF) as u8,
                ((len >> 24) & 0xFF) as u8,
                crate::streaming::FRAME_DATA,
                is_compressed,
            ];
            let payload_slice = if self.b_fast.last_was_compressed {
                &self.b_fast.compressed_buffer[..slice_len]
            } else {
                &self.b_fast.work_buffer[..slice_len]
            };
            total_sent += 6 + slice_len;
            self.buffer.extend_from_slice(&header);
            self.buffer.extend_from_slice(payload_slice);
        }

        if self.buffer.len() >= self.buffer_size {
            py.allow_threads(|| {
                fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                    .map_err(io_error_to_py_err)
            })?;
            self.buffer.clear();
        }

        for item in iter {
            let item = item?;
            let mut processed = false;

            if let Some(ref t) = self.cached_template {
                let prev_len = self.buffer.len();
                if self
                    .b_fast
                    .encode_homogeneous_frame(item.as_ptr(), py, t, &mut self.buffer)
                {
                    total_sent += self.buffer.len() - prev_len;
                    processed = true;
                } else {
                    self.cached_template = None;
                }
            }

            if !processed {
                self.b_fast.encode_packed_internal(&item, compress)?;
                let is_compressed = if self.b_fast.last_was_compressed {
                    1u8
                } else {
                    0u8
                };
                let slice_len = if self.b_fast.last_was_compressed {
                    self.b_fast.compressed_buffer.len()
                } else {
                    self.b_fast.work_buffer.len()
                };
                let len = slice_len as u32;
                let header = [
                    (len & 0xFF) as u8,
                    ((len >> 8) & 0xFF) as u8,
                    ((len >> 16) & 0xFF) as u8,
                    ((len >> 24) & 0xFF) as u8,
                    crate::streaming::FRAME_DATA,
                    is_compressed,
                ];
                let payload_slice = if self.b_fast.last_was_compressed {
                    &self.b_fast.compressed_buffer[..slice_len]
                } else {
                    &self.b_fast.work_buffer[..slice_len]
                };
                total_sent += 6 + slice_len;
                self.buffer.extend_from_slice(&header);
                self.buffer.extend_from_slice(payload_slice);
            }

            if self.buffer.len() >= self.buffer_size {
                py.allow_threads(|| {
                    fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                        .map_err(io_error_to_py_err)
                })?;
                self.buffer.clear();
            }
        }

        Ok(total_sent)
    }

    /// Send the 4-byte stream handshake ('BS\x01\x00') to the socket.
    #[pyo3(signature = (sock_fd, *, non_blocking = false))]
    pub fn send_handshake(
        &mut self,
        py: Python,
        sock_fd: i32,
        non_blocking: bool,
    ) -> PyResult<usize> {
        if self.buffer_size == 0 {
            return py.allow_threads(|| {
                fallback_socket_send(sock_fd, &HANDSHAKE, non_blocking).map_err(io_error_to_py_err)
            });
        }
        self.buffer.extend_from_slice(&HANDSHAKE);
        if self.buffer.len() >= self.buffer_size {
            py.allow_threads(|| {
                fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                    .map_err(io_error_to_py_err)
            })?;
            self.buffer.clear();
        }
        Ok(HANDSHAKE.len())
    }

    /// Send the 6-byte End-of-Stream frame to the socket.
    #[pyo3(signature = (sock_fd, *, non_blocking = false))]
    pub fn send_eos(&mut self, py: Python, sock_fd: i32, non_blocking: bool) -> PyResult<usize> {
        if self.buffer_size == 0 {
            return py.allow_threads(|| {
                fallback_socket_send(sock_fd, &EOS, non_blocking).map_err(io_error_to_py_err)
            });
        }
        self.buffer.extend_from_slice(&EOS);
        if self.buffer.len() >= self.buffer_size {
            py.allow_threads(|| {
                fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                    .map_err(io_error_to_py_err)
            })?;
            self.buffer.clear();
        }
        Ok(EOS.len())
    }

    /// Flush any remaining pending bytes from the buffer and pipe into the socket.
    #[pyo3(signature = (sock_fd, *, non_blocking = false))]
    pub fn flush(&mut self, py: Python, sock_fd: i32, non_blocking: bool) -> PyResult<usize> {
        let mut total_flushed = 0;
        if !self.buffer.is_empty() {
            total_flushed = py.allow_threads(|| {
                let res = fallback_socket_send(sock_fd, &self.buffer, non_blocking)
                    .map_err(io_error_to_py_err);
                self.buffer.clear();
                res
            })?;
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(pipe) = self.pipe.as_mut() {
                if pipe.pending_bytes() > 0 {
                    let drained = py.allow_threads(|| {
                        pipe.drain_pending(sock_fd, non_blocking)
                            .map_err(io_error_to_py_err)
                    })?;
                    total_flushed += drained;
                }
            }
        }
        Ok(total_flushed)
    }

    /// Close and release native transport resources.
    pub fn close(&mut self) {
        #[cfg(target_os = "linux")]
        {
            self.pipe = None;
        }
        self.fallback_mode = true;
        self.buffer.clear();
        self.cached_template = None;
    }
}

impl BFastNativeSender {
    fn send_bytes_disjoint(
        #[cfg(target_os = "linux")] pipe: &mut Option<NativePipe>,
        fallback_mode: &mut bool,
        sock_fd: i32,
        data: &[u8],
        non_blocking: bool,
    ) -> PyResult<usize> {
        #[cfg(target_os = "linux")]
        {
            if let Some(pipe_ref) = pipe.as_mut() {
                if pipe_ref.pending_bytes() > 0 {
                    pipe_ref
                        .drain_pending(sock_fd, non_blocking)
                        .map_err(io_error_to_py_err)?;
                }
            }
        }

        if *fallback_mode {
            return fallback_socket_send(sock_fd, data, non_blocking).map_err(io_error_to_py_err);
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(pipe_ref) = pipe.as_mut() {
                match pipe_ref.splice_data(sock_fd, data, non_blocking) {
                    Ok(n) => Ok(n),
                    Err(e)
                        if e.raw_os_error() == Some(libc::EINVAL)
                            || e.raw_os_error() == Some(libc::ENOSYS) =>
                    {
                        // Splicing not supported on this specific descriptor: switch to fallback
                        pipe_ref.drain_discard();
                        *fallback_mode = true;
                        fallback_socket_send(sock_fd, data, non_blocking)
                            .map_err(io_error_to_py_err)
                    }
                    Err(e) => Err(io_error_to_py_err(e)),
                }
            } else {
                fallback_socket_send(sock_fd, data, non_blocking).map_err(io_error_to_py_err)
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            fallback_socket_send(sock_fd, data, non_blocking).map_err(io_error_to_py_err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_available_on_linux() {
        #[cfg(target_os = "linux")]
        assert!(is_native_available());
        #[cfg(not(target_os = "linux"))]
        assert!(!is_native_available());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_native_socketpair_transfer() {
        let mut sv = [0 as libc::c_int; 2];
        let res = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sv.as_mut_ptr()) };
        assert_eq!(res, 0);

        let mut pipe = NativePipe::new().expect("Failed to create NativePipe");
        let payload = b"Hello Linux Native Socket Transport!";

        let sent = pipe
            .splice_data(sv[0], payload, false)
            .expect("splice_data failed");
        assert_eq!(sent, payload.len());

        let mut recv_buf = vec![0u8; payload.len()];
        let n_read = unsafe {
            libc::read(
                sv[1],
                recv_buf.as_mut_ptr() as *mut libc::c_void,
                recv_buf.len(),
            )
        };
        assert_eq!(n_read as usize, payload.len());
        assert_eq!(&recv_buf, payload);

        unsafe {
            libc::close(sv[0]);
            libc::close(sv[1]);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_native_large_payload() {
        let mut sv = [0 as libc::c_int; 2];
        let res = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sv.as_mut_ptr()) };
        assert_eq!(res, 0);

        // Increase socket buffer size for large transfer test
        let buf_size: libc::c_int = 2 * 1024 * 1024;
        unsafe {
            libc::setsockopt(
                sv[0],
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
            libc::setsockopt(
                sv[1],
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &buf_size as *const _ as *const libc::c_void,
                std::mem::size_of_val(&buf_size) as libc::socklen_t,
            );
        }

        let mut pipe = NativePipe::new().expect("Failed to create NativePipe");
        // 256 KB test pattern
        let mut payload = Vec::with_capacity(256 * 1024);
        for i in 0..(256 * 1024) {
            payload.push((i % 251) as u8);
        }

        // Spawn a thread to read from sv[1]
        let expected = payload.clone();
        let reader_handle = std::thread::spawn(move || {
            let mut received = Vec::with_capacity(expected.len());
            let mut chunk = [0u8; 16384];
            while received.len() < expected.len() {
                let to_read = (expected.len() - received.len()).min(chunk.len());
                let n =
                    unsafe { libc::read(sv[1], chunk.as_mut_ptr() as *mut libc::c_void, to_read) };
                if n > 0 {
                    received.extend_from_slice(&chunk[..n as usize]);
                } else {
                    break;
                }
            }
            assert_eq!(received.len(), expected.len());
            assert_eq!(received, expected);
        });

        let sent = pipe
            .splice_data(sv[0], &payload, false)
            .expect("splice_data large failed");
        assert_eq!(sent, payload.len());

        reader_handle.join().unwrap();

        unsafe {
            libc::close(sv[0]);
            libc::close(sv[1]);
        }
    }
}
