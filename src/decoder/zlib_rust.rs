//! `zlib-rs` wrapper

use core::{mem, ptr};

use super::zlib_common::ZlibMode;
use super::{Decode, Decoder, DecodeStatus, DecodeError};

mod sys {
    pub use zlib_rs::c_api::z_stream;
    pub use zlib_rs::InflateFlush;
    pub use zlib_rs::inflate::*;
    pub use zlib_rs::ReturnCode;
    pub use zlib_rs::ReturnCode::Ok as Z_OK;
    pub use zlib_rs::ReturnCode::StreamEnd as Z_STREAM_END;
    pub use zlib_rs::ReturnCode::BufError as Z_BUF_ERROR;
}

const DEFAULT_INFLATE: sys::InflateFlush = sys::InflateFlush::NoFlush;

fn describe_error_fn(code: i32) -> Option<&'static str> {
    match sys::ReturnCode::try_from_c_int(code as _) {
        Some(sys::ReturnCode::Ok) => Some("ok"),
        Some(sys::ReturnCode::StreamEnd) => Some("stream end"),
        Some(sys::ReturnCode::NeedDict) => Some("need dictionary"),
        Some(sys::ReturnCode::ErrNo) => Some("file error"),
        Some(sys::ReturnCode::StreamError) => Some("stream error"),
        Some(sys::ReturnCode::DataError) => Some("data error"),
        Some(sys::ReturnCode::MemError) => Some("insufficient memory"),
        Some(sys::ReturnCode::BufError) => Some("buffer error"),
        Some(sys::ReturnCode::VersionError) => Some("incompatible version"),
        _ => Some("impossible error"),
    }
}

#[repr(transparent)]
///Decoder backed by [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs)
pub struct ZlibRust {
    inner: sys::z_stream
}

impl ZlibRust {
    ///Creates new instance unless underlying library is unable to create instance
    pub fn new(mode: ZlibMode) -> Option<Self> {
        let mut this = Self {
            inner: sys::z_stream {
                next_in: ptr::null_mut(),
                avail_in: 0,
                total_in: 0,
                next_out: ptr::null_mut(),
                avail_out: 0,
                total_out: 0,
                msg: ptr::null_mut(),
                state: ptr::null_mut(),
                zalloc: None,
                zfree: None,
                opaque: ptr::null_mut(),
                data_type: 0,
                adler: 0,
                reserved: 0,
            }
        };
        this.inner.configure_default_rust_allocator();

        let config = sys::InflateConfig {
            window_bits: mode.max_bits(),
        };
        if sys::init(&mut this.inner, config) == sys::ReturnCode::Ok {
            Some(this)
        } else {
            None
        }
    }

    //z_stream has the same layout as DeflateStream,
    #[inline(always)]
    fn as_mut(&mut self) -> &mut sys::InflateStream<'_> {
        unsafe {
            mem::transmute(&mut self.inner)
        }
    }
}

impl Decoder for ZlibRust {
    #[inline]
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        self.inner.avail_out = output.len() as _;
        self.inner.next_out = output.as_mut_ptr() as *mut u8;

        self.inner.avail_in = input.len() as _;
        self.inner.next_in = input.as_ptr() as *mut _;

        let result = unsafe {
            sys::inflate(self.as_mut(), DEFAULT_INFLATE)
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
                code => Err(DecodeError {
                    code: code as _,
                    describe_error_fn,
                })
            }
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        sys::reset(self.as_mut()) == sys::Z_OK
    }
}

impl Drop for ZlibRust {
    #[inline(always)]
    fn drop(&mut self) {
        sys::end(self.as_mut());
    }
}

//These gets invalidated due to pointer usage, but in fact we're totally fine
unsafe impl Send for ZlibRust {}
unsafe impl Sync for ZlibRust {}
