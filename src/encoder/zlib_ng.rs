//! Zlib-ng module

extern crate alloc;

use libz_ng_sys as sys;

use core::ptr;
use alloc::boxed::Box;

use super::{Interface, Encoder, Encode, EncodeOp, ZlibOptions, ZlibStrategy};
use crate::mem::{compu_alloc, compu_free_with_state};

static ZLIB: Interface = Interface {
    drop_fn,
    reset_fn,
    encode_fn,
};

#[repr(transparent)]
struct State {
    inner: sys::z_stream,
}

impl State {
    fn reset(&mut self) -> bool {
        unsafe {
            sys::deflateReset(&mut self.inner) == sys::Z_OK
        }
    }
}

impl Drop for State {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            sys::deflateEnd(&mut self.inner);
        }
    }
}

impl Interface {
    #[inline]
    ///Creates encoder with `zlib-ng` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zlib_ng(opts: ZlibOptions) -> Option<Encoder> {
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
        let max_bits = opts.mode as _;
        let strategy = match opts.strategy {
            ZlibStrategy::Default => sys::Z_DEFAULT_STRATEGY,
            ZlibStrategy::Filtered => sys::Z_FILTERED,
            ZlibStrategy::HuffmanOnly => sys::Z_HUFFMAN_ONLY,
            ZlibStrategy::Rle => sys::Z_RLE,
            ZlibStrategy::Fixed => sys::Z_FIXED
        };
        let result = unsafe {
            sys::deflateInit2_(&mut instance.inner, opts.compression as _, sys::Z_DEFLATED, max_bits, opts.mem_level as _, strategy, sys::zlibVersion(), core::mem::size_of::<sys::z_stream>() as _)
        };

        if result == 0 {
            let instance = unsafe {
                ptr::NonNull::new_unchecked(Box::into_raw(instance)).cast()
            };
            Some(Encoder {
                instance,
                interface: &ZLIB,
                opts: [0; 2],
            })
        } else {
            None
        }
    }
}

unsafe fn encode_fn(state: ptr::NonNull<u8>, input: *const u8, input_remain: usize, output: *mut u8, output_remain: usize, op: EncodeOp) -> Encode {
    internal_zlib_impl_encode!(state, input, input_remain, output, output_remain, op)
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>, _: [u8; 2]) -> Option<ptr::NonNull<u8>> {
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
fn drop_fn(state: ptr::NonNull<u8>) {
    unsafe {
        drop(Box::from_raw(state.as_ptr() as *mut State));
    }

}
