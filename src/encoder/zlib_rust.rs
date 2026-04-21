//! zlib-rust module

extern crate alloc;

use alloc::boxed::Box;
use core::{ptr, mem};

use super::{Encode, EncodeOp, Encoder, Interface, ZlibOptions, ZlibStrategy};

mod sys {
    pub use zlib_rs::c_api::z_stream;
    pub use zlib_rs::deflate::*;
    pub use zlib_rs::ReturnCode;
    pub use zlib_rs::ReturnCode::Ok as Z_OK;
    pub use zlib_rs::ReturnCode::StreamEnd as Z_STREAM_END;
    pub use zlib_rs::ReturnCode::BufError as Z_BUF_ERROR;

    pub use zlib_rs::DeflateFlush::NoFlush as Z_NO_FLUSH;
    pub use zlib_rs::DeflateFlush::SyncFlush as Z_SYNC_FLUSH;
    pub use zlib_rs::DeflateFlush::Finish as Z_FINISH;
}

static ZLIB: Interface = Interface {
    drop_fn,
    reset_fn,
    encode_fn,
};

#[repr(transparent)]
pub struct State {
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
    pub fn reset(&mut self) -> sys::ReturnCode {
        sys::reset(self.as_mut())
    }

    //z_stream has the same layout as DeflateStream,
    //but for some reason guy is doing some bullshit requiring to transmute it
    #[inline(always)]
    fn as_mut(&mut self) -> &mut sys::DeflateStream<'_> {
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
    #[inline]
    ///Creates encoder with `zlib-rs` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zlib_rust(opts: ZlibOptions) -> Option<Encoder> {
        let mut instance = Box::new(State::new());

        let strategy = match opts.strategy {
            ZlibStrategy::Default => sys::Strategy::Default,
            ZlibStrategy::Filtered => sys::Strategy::Filtered,
            ZlibStrategy::HuffmanOnly => sys::Strategy::HuffmanOnly,
            ZlibStrategy::Rle => sys::Strategy::Rle,
            ZlibStrategy::Fixed => sys::Strategy::Fixed,
        };

        let config = sys::DeflateConfig {
            level: opts.compression as _,
            method: sys::Method::Deflated,
            window_bits: opts.mode as _,
            strategy,
            mem_level: opts.mem_level as _,
        };

        let result = sys::init(&mut instance.inner, config);

        if result == sys::ReturnCode::Ok {
            let instance = ptr::NonNull::from(Box::leak(instance)).cast();
            Some(ZLIB.inner_encoder(instance, [0; 2]))
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
    match result {
        sys::ReturnCode::Ok => Some(state),
        _ => None,
    }
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    unsafe {
        drop(Box::from_raw(state.as_ptr() as *mut State));
    }
}
