use lz4_flex::block::decompress_into;
use std::alloc::{alloc as sys_alloc, dealloc as sys_dealloc, Layout};

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
