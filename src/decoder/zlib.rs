//! `zlib` wrapper

extern crate alloc;

use libz_sys as sys;

use core::{mem, ptr};
use core::ffi::c_int;
use alloc::boxed::Box;

use super::{Interface, Decoder, Decode};
use super::zlib_common::ZlibMode;
use crate::mem::{compu_alloc, compu_free_with_state};

extern "C" {
    pub fn zError(code: c_int) -> *const i8;
}

///`zlib` interface
static ZLIB: Interface = Interface {
    drop_fn,
    reset_fn,
    decode_fn,
    describe_error_fn,
};

#[repr(transparent)]
struct State {
    inner: sys::z_stream,
}

impl State {
    fn reset(&mut self) -> bool {
        unsafe {
            sys::inflateReset(&mut self.inner) == sys::Z_OK
        }
    }
}

impl Drop for State {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            sys::inflateEnd(&mut self.inner);
        }
    }
}

impl Interface {
    ///Creates decoder with `zlib` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zlib(mode: ZlibMode) -> Option<Decoder> {
        let mut instance = Box::new(State {
            inner: sys::z_stream {
                next_in: ptr::null_mut(),
                avail_in: 0,
                total_in: 0,
                next_out: ptr::null_mut(),
                avail_out: 0,
                total_out: 0,
                msg: ptr::null_mut(),
                state: ptr::null_mut(),
                zalloc: compu_alloc,
                zfree: compu_free_with_state,
                opaque: ptr::null_mut(),
                data_type: 0,
                adler: 0,
                reserved: 0,
            },
        });
        let result = unsafe {
            sys::inflateInit2_(&mut instance.inner, mode.max_bits(), sys::zlibVersion(), mem::size_of::<sys::z_stream>() as _)
        };

        if result == 0 {
            let instance = unsafe {
                ptr::NonNull::new_unchecked(Box::into_raw(instance)).cast()
            };
            Some(Decoder {
                instance,
                interface: &ZLIB,
            })
        } else {
            None
        }
    }
}

#[inline]
unsafe fn decode_fn(state: ptr::NonNull<u8>, input: *const u8, input_remain: usize, output: *mut u8, output_remain: usize) -> Decode {
    internal_zlib_impl_decode!(state, input, input_remain, output, output_remain)
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>> {
    let result = unsafe {
        (*(state.as_ptr() as *mut State)).reset()
    };
    if result {
        Some(state)
    } else {
        None
    }
}

#[inline]
fn drop_fn(data: ptr::NonNull<u8>) {
    unsafe {
        drop(Box::from_raw(data.as_ptr() as *mut State));
    }
}

#[inline]
fn describe_error_fn(code: i32) -> Option<&'static str> {
    let result = unsafe {
        zError(code)
    };
    crate::utils::convert_c_str(result)
}
