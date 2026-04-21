//! `zlib-rs` wrapper

extern crate alloc;

use alloc::boxed::Box;

use core::{mem, ptr};

use super::zlib_common::ZlibMode;
use super::{Decode, Decoder, Interface};

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

static ZLIB_RUST: Interface = Interface {
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
    pub fn new() -> Self {
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
        this
    }

    #[inline(always)]
    pub fn reset(&mut self) -> bool {
        sys::reset(self.as_mut()) == sys::Z_OK
    }

    //z_stream has the same layout as DeflateStream,
    //but for some reason guy is doing some bullshit requiring to transmute it
    #[inline(always)]
    fn as_mut(&mut self) -> &mut sys::InflateStream<'_> {
        unsafe {
            mem::transmute(&mut self.inner)
        }
    }
}

impl Drop for State {
    #[inline(always)]
    fn drop(&mut self) {
        let _ = sys::end(self.as_mut());
    }
}

impl Interface {
    ///Creates decoder with `zlib-rust` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zlib_rust(mode: ZlibMode) -> Option<Decoder> {
        let mut instance = Box::new(State::new());
        let config = sys::InflateConfig {
            window_bits: mode.max_bits(),
        };
        let result = sys::init(&mut instance.inner, config);

        if result == sys::ReturnCode::Ok {
            let instance = ptr::NonNull::from(Box::leak(instance)).cast();
            Some(ZLIB_RUST.inner_decoder(instance))
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
