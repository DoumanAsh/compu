use std::os::raw::c_uint;

//Linux & win 32 bit is 8
//Mac and  win 64 bit is 16
#[cfg(not(any(target_os = "macos", all(windows, target_pointer_width = "64"))))]
const MIN_ALIGN: usize = 8;
#[cfg(any(target_os = "macos", all(windows, target_pointer_width = "64")))]
const MIN_ALIGN: usize = 16;

use core::{mem, ptr};
use std::alloc::Layout;

#[cold]
#[inline(never)]
fn unlikely_null() -> *mut core::ffi::c_void {
    ptr::null_mut()
}

const LAYOUT_OFFSET: usize = mem::size_of::<usize>();

pub(crate) unsafe extern "C" fn compu_custom_malloc(_: *mut core::ffi::c_void, size: usize) -> *mut core::ffi::c_void {
    let layout = match Layout::from_size_align(size + LAYOUT_OFFSET, MIN_ALIGN) {
        Ok(layout) => layout,
        _ => return unlikely_null(),
    };

    let mem = std::alloc::alloc(layout);

    if mem.is_null() {
        return unlikely_null();
    }

    ptr::write(mem as *mut usize, size);
    mem.add(LAYOUT_OFFSET) as _
}

pub(crate) unsafe extern "C" fn compu_custom_alloc(op: *mut core::ffi::c_void, items: c_uint, size: c_uint) -> *mut core::ffi::c_void {
    let size = match (items as usize).checked_mul(size as usize) {
        Some(0) | None => return unlikely_null(),
        Some(size) => size,
    };
    compu_custom_malloc(op, size)
}

pub(crate) unsafe extern "C" fn compu_custom_free(_: *mut core::ffi::c_void, mem: *mut core::ffi::c_void) {
    if !mem.is_null() {
        let mem = (mem as *mut u8).offset(-(LAYOUT_OFFSET as isize));
        let size = ptr::read(mem as *const usize);
        let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
        std::alloc::dealloc(mem, layout);
    }
}
