//! `brotli` interface implementation

use compu_brotli_sys as sys;

use core::ptr;

use super::{Interface, Encoder, Encode, EncodeStatus, EncodeOp};
use crate::mem::{compu_malloc_with_state, compu_free_with_state};

static BROTLI_C: Interface = Interface {
    drop_fn,
    reset_fn,
    encode_fn,
};

#[repr(u8)]
#[derive(Copy, Clone)]
///Encoding mode
pub enum BrotliEncoderMode {
    ///Default mode. No assumptions about content.
    Generic = 1,
    ///Text mode. UTF-8.
    Text,
    ///WOFF 2.0 mode
    Font
}

///!Brotli options
#[derive(Default, Clone)]
pub struct BrotliOptions {
    inner: [u8; 2],
}

impl BrotliOptions {
    const QUALITY_IDX: usize = 0;
    const MODE_IDX: usize = 1;

    #[inline(always)]
    ///Creates default instance
    pub const fn new() -> Self {
        Self::from_raw([0; 2])
    }

    #[inline(always)]
    ///Creates default instance
    const fn from_raw(inner: [u8; 2]) -> Self {
        Self {
            inner,
        }
    }

    #[inline(always)]
    ///Sets quality
    ///
    ///Allowed values are from 1 to 11.
    ///See brotli API docs for details.
    ///
    ///Default value is 11.
    pub const fn quality(mut self, quality: u8) -> Self {
        assert!(quality > 0);
        assert!(quality <= 11);

        self.inner[Self::QUALITY_IDX] = quality;
        self
    }

    #[inline(always)]
    ///Sets mode
    pub const fn mode(mut self, mode: BrotliEncoderMode) -> Self {
        self.inner[Self::MODE_IDX] = mode as u8;
        self
    }

    fn apply(&self, state: *mut sys::BrotliEncoderState) {
        unsafe {
            let quality = self.inner[Self::QUALITY_IDX];
            if quality > 0 {
                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_QUALITY, quality as _) != 0);
            }
            let mode = self.inner[Self::MODE_IDX];
            if mode > 0 {
                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_MODE, mode as _) != 0);
            }
        }
    }
}

impl EncodeOp {
    #[inline(always)]
    const fn into_brotli(self) -> sys::BrotliEncoderOperation {
        match self {
            Self::Process => sys::BrotliEncoderOperation_BROTLI_OPERATION_PROCESS,
            Self::Flush => sys::BrotliEncoderOperation_BROTLI_OPERATION_FLUSH,
            Self::Finish => sys::BrotliEncoderOperation_BROTLI_OPERATION_FINISH,
        }
    }
}

#[inline]
fn new_encoder() -> Option<ptr::NonNull<u8>> {
    let result = unsafe {
        sys::BrotliEncoderCreateInstance(Some(compu_malloc_with_state), Some(compu_free_with_state), ptr::null_mut())
    };

    ptr::NonNull::new(result as *mut u8)
}

impl Interface {
    #[inline]
    ///Creates encoder with `brotli-c` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn brotli_c(options: BrotliOptions) -> Option<Encoder> {
        match new_encoder() {
            Some(ptr) => {
                options.apply(ptr.as_ptr() as _);
                Some(Encoder {
                    instance: ptr.cast(),
                    interface: &BROTLI_C,
                    opts: options.inner,
                })
            },
            None => None,
        }
    }
}

unsafe fn encode_fn(state: ptr::NonNull<u8>, mut input: *const u8, mut input_remain: usize, mut output: *mut u8, mut output_remain: usize, op: EncodeOp) -> Encode {
    let result = unsafe {
        sys::BrotliEncoderCompressStream(state.as_ptr() as _, op.into_brotli(), &mut input_remain, &mut input, &mut output_remain, &mut output, ptr::null_mut())
    };
    Encode {
        input_remain,
        output_remain,
        status: match result {
            0 => EncodeStatus::Error,
            _ => {
                let result = unsafe {
                    sys::BrotliEncoderHasMoreOutput(state.as_ptr() as _)
                };
                if result == 1 {
                    EncodeStatus::NeedOutput
                } else if op == EncodeOp::Finish {
                    EncodeStatus::Finished
                } else {
                    EncodeStatus::Continue
                }
            }
        }
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>, opts: [u8; 2]) -> Option<ptr::NonNull<u8>> {
    let options = BrotliOptions::from_raw(opts);
    match new_encoder() {
        Some(ptr) => {
            drop_fn(state);
            options.apply(ptr.as_ptr() as _);
            Some(ptr)
        },
        None => None,
    }
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    unsafe {
        sys::BrotliEncoderDestroyInstance(state.as_ptr() as _);
    }
}
