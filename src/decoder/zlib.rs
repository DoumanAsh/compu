//! `zlib` wrapper

extern crate alloc;

use libz_sys as sys;

use alloc::boxed::Box;
use core::ffi::c_int;
use core::{mem, ptr};

use super::zlib_common::ZlibMode;
use super::{Decode, Decoder, DecodeStatus, DecodeError};
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

impl Decoder for ZlibC {
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
