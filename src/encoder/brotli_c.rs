//! `brotli` interface implementation

use compu_brotli_sys as sys;

use core::ptr;

use super::{Interface, Encoder, Encode, EncodeStatus, EncodeOp};
use crate::mem::{compu_malloc_with_state, compu_free_with_state};
use super::brotli_common::BrotliOptions;

static BROTLI_C: Interface = Interface::new(
    reset_fn,
    encode_fn,
    drop_fn,
);

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
                options.apply_c(ptr.as_ptr() as _);
                Some(BROTLI_C.inner_encoder(ptr.cast(), options.inner))
            },
            None => None,
        }
    }
}

unsafe fn encode_fn(state: ptr::NonNull<u8>, mut input: *const u8, mut input_remain: usize, mut output: *mut u8, mut output_remain: usize, op: EncodeOp) -> Encode {
    let result = unsafe {
        sys::BrotliEncoderCompressStream(state.as_ptr() as _, op.into_brotli(), &mut input_remain, &mut input, &mut output_remain, &mut output, ptr::null_mut())
    };

    let has_more_output = unsafe {
        sys::BrotliEncoderHasMoreOutput(state.as_ptr() as _)
    };
    Encode {
        input_remain,
        output_remain,
        status: match result {
            0 => match has_more_output {
                0 => EncodeStatus::Error,
                _ => EncodeStatus::NeedOutput,
            },
            _ => {
                if has_more_output != 0 {
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
            options.apply_c(ptr.as_ptr() as _);
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
