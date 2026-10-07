//! `zlib` wrapper

extern crate alloc;

use libz_sys as sys;

use alloc::boxed::Box;
use core::ffi::c_int;
use core::{mem, ptr};

use super::zlib_common::ZlibMode;
use super::{Decode, Decoder, Interface, DecoderInterface, DecodeStatus, DecodeError};
use crate::mem::{compu_alloc, compu_free_with_state};

const DEFAULT_INFLATE: i32 = 0;

extern "C" {
    pub fn zError(code: c_int) -> *const i8;
}

#[repr(transparent)]
///Decoder backed by [libz](https://github.com/rust-lang/libz-sys)
pub struct ZlibC {
    inner: sys::z_stream
}

impl ZlibC {
    #[inline(always)]
    ///Creates new instance unless underlying C library is unable to create instance
    pub fn new(mode: ZlibMode) -> Option<Box<Self>> {
        //zlib pointers require our state to be unmoved
        let mut instance = Box::new(Self {
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
            sys::inflateInit2_(
                &mut instance.inner,
                mode.max_bits(),
                sys::zlibVersion(),
                mem::size_of::<sys::z_stream>() as _,
            )
        };

        if result == 0 {
            Some(instance)
        } else {
            None
        }
    }
}

impl DecoderInterface for ZlibC {
    #[inline]
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        self.inner.avail_out = output.len() as _;
        self.inner.next_out = output.as_mut_ptr() as *mut u8;

        self.inner.avail_in = input.len() as _;
        self.inner.next_in = input.as_ptr() as *mut _;

        let result = unsafe {
            sys::inflate(&mut self.inner, DEFAULT_INFLATE)
        };

        Decode {
            input_remain: self.inner.avail_in as usize,
            output_remain: self.inner.avail_out as usize,
            status: match result {
                sys::Z_OK => match self.inner.avail_in {
                    0 => Ok(DecodeStatus::NeedInput),
                    _ => Ok(DecodeStatus::NeedOutput),
                },
                sys::Z_STREAM_END => Ok(DecodeStatus::Finished),
                sys::Z_BUF_ERROR => Ok(DecodeStatus::NeedOutput),
                other => Err(crate::decoder::DecodeError(other as _)),
            }
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        unsafe { sys::inflateReset(&mut self.inner) == sys::Z_OK }
    }

    #[inline(always)]
    fn describe_error(&self, code: DecodeError) -> Option<&'static str> {
        let result = unsafe {
            zError(code.0)
        };
        crate::utils::convert_c_str(result)
    }
}

impl Drop for ZlibC {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            sys::inflateEnd(&mut self.inner);
        }
    }
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
    #[inline(always)]
    fn as_mut(&mut self) -> &mut sys::z_stream {
        &mut self.inner
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        unsafe { sys::inflateReset(&mut self.inner) == sys::Z_OK }
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
            sys::inflateInit2_(
                &mut instance.inner,
                mode.max_bits(),
                sys::zlibVersion(),
                mem::size_of::<sys::z_stream>() as _,
            )
        };

        if result == 0 {
            let instance = ptr::NonNull::from(Box::leak(instance)).cast();
            Some(ZLIB.inner_decoder(instance))
        } else {
            None
        }
    }
}

#[inline]
unsafe fn decode_fn(state: ptr::NonNull<u8>, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
    internal_zlib_impl_decode!(state, input, output)
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
