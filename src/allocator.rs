use pyo3::ffi::{PyMem_Malloc, PyMem_Free, PyMem_Realloc};
use std::alloc::{GlobalAlloc, Layout};

pub struct PyMemAllocator;

unsafe impl GlobalAlloc for PyMemAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        PyMem_Malloc(layout.size()).cast()
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        PyMem_Free(ptr.cast());
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        PyMem_Realloc(ptr.cast(), new_size).cast()
    }
}

#[global_allocator]
static GLOBAL: PyMemAllocator = PyMemAllocator;
