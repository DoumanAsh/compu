//!Custom malloc implementation which uses Rust's allocator and provides common interface required by compression libraries
use core::ffi::{c_void, c_uint};

extern crate alloc;

use core::{mem, ptr};
use alloc::alloc::Layout;
pub use alloc::boxed::Box;

//Linux & win 32 bit are 8
#[cfg(not(any(target_os = "macos", all(windows, target_pointer_width = "64"))))]
const MIN_ALIGN: usize = 8;
//Mac and  win 64 bit are 16
#[cfg(any(target_os = "macos", all(windows, target_pointer_width = "64")))]
const MIN_ALIGN: usize = 16;

const LAYOUT_OFFSET: usize = mem::size_of::<usize>();

#[cold]
#[inline(never)]
fn unlikely_null() -> *mut c_void {
    ptr::null_mut()
}


#[inline]
///`malloc` impl with Rust allocator
pub unsafe extern "C" fn compu_malloc(size: usize) -> *mut c_void {
    if let Ok(layout) = Layout::from_size_align(size + LAYOUT_OFFSET, MIN_ALIGN) {
        let mem = alloc::alloc::alloc(layout);
        if !mem.is_null() {
            ptr::write(mem as *mut usize, size);
            return mem.add(LAYOUT_OFFSET) as _
        }
    }

    unlikely_null()
}

#[inline]
///`free` impl with Rust allocator
pub unsafe extern "C" fn compu_free(mem: *mut c_void) {
    if !mem.is_null() {
        let mem = (mem as *mut u8).offset(-(LAYOUT_OFFSET as isize));
        let size = ptr::read(mem as *const usize);
        let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
        alloc::alloc::dealloc(mem, layout);
    }
}

#[allow(unused)]
///`malloc` alternative with Rust allocator
pub(crate) unsafe extern "C" fn compu_malloc_with_state(_: *mut c_void, size: usize) -> *mut c_void {
    compu_malloc(size)
}

#[allow(unused)]
///`alloc` alternative with Rust allocator
pub(crate) unsafe extern "C" fn compu_alloc(_: *mut c_void, items: c_uint, size: c_uint) -> *mut c_void {
    let size = match (items as usize).checked_mul(size as usize) {
        Some(0) | None => return unlikely_null(),
        Some(size) => size,
    };
    compu_malloc(size)
}

#[allow(unused)]
pub(crate) unsafe extern "C" fn compu_free_with_state(_: *mut c_void, mem: *mut c_void) {
    compu_free(mem)
}
