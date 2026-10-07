//! zlib-rust module

extern crate alloc;

use core::{ptr, mem};

use super::{Encode, EncodeOp, EncodeStatus, Encoder, ZlibOptions, ZlibStrategy};

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

#[repr(transparent)]
///Encoder backed by [zlib-rs](https://github.com/trifectatechfoundation/zlib-rs)
pub struct ZlibRust {
    inner: sys::z_stream,
}

impl ZlibRust {
    #[inline(always)]
    ///Creates new instance unless underlying library is unable to create instance
    pub fn new(opts: ZlibOptions) -> Option<Self> {
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

        let result = sys::init(&mut this.inner, config);

        if result == sys::ReturnCode::Ok {
            Some(this)
        } else {
            None
        }
    }

    //z_stream has the same layout as DeflateStream,
    #[inline(always)]
    fn as_mut(&mut self) -> &mut sys::DeflateStream<'_> {
        unsafe {
            mem::transmute(&mut self.inner)
        }
    }
}

impl Encoder for ZlibRust {
    #[inline]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        let op = match op {
            EncodeOp::Process => sys::Z_NO_FLUSH,
            EncodeOp::Flush => sys::Z_SYNC_FLUSH,
            EncodeOp::Finish => sys::Z_FINISH
        };

        self.inner.avail_out = output.len() as _;
        self.inner.next_out = output.as_mut_ptr() as *mut _;

        self.inner.avail_in = input.len() as _;
        self.inner.next_in = input.as_ptr();

        let result = sys::deflate(self.as_mut(), op);

        Encode {
            input_remain: self.inner.avail_in as usize,
            output_remain: self.inner.avail_out as usize,
            status: match result {
                sys::Z_STREAM_END => EncodeStatus::Finished,
                //If it is final chunk, zlib may report OK while it needs more output (specifically in case of GZIP)
                sys::Z_OK => if op == sys::Z_FINISH {
                    EncodeStatus::NeedOutput
                } else {
                    EncodeStatus::Continue
                },
                sys::Z_BUF_ERROR => EncodeStatus::NeedOutput,
                _ => EncodeStatus::Error,
            }
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        sys::reset(self.as_mut()) == sys::ReturnCode::Ok
    }
}
