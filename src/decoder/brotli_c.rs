//! `brotli` interface implementation

use compu_brotli_sys as sys;

use core::ptr;

use super::{Decode, DecodeError, DecodeStatus, Decoder, Interface};
use crate::mem::{compu_free_with_state, compu_malloc_with_state};

static BROTLI_C: Interface = Interface::new(
    decode_fn,
    reset_fn,
    drop_fn,
    describe_error_fn
);

impl Interface {
    #[inline]
    ///Creates decoder with `brotli-c` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn brotli_c() -> Option<Decoder> {
        match new_decoder() {
            Some(ptr) => Some(BROTLI_C.inner_decoder(ptr.cast())),
            None => None,
        }
    }
}

#[inline]
fn new_decoder() -> Option<ptr::NonNull<u8>> {
    let result = unsafe {
        sys::BrotliDecoderCreateInstance(Some(compu_malloc_with_state), Some(compu_free_with_state), ptr::null_mut())
    };

    ptr::NonNull::new(result as *mut u8)
}

#[inline]
unsafe fn decode_fn(state: ptr::NonNull<u8>, mut input: *const u8, mut input_remain: usize, mut output: *mut u8, mut output_remain: usize) -> Decode {
    let state = state.as_ptr() as _;
    let result = unsafe {
        sys::BrotliDecoderDecompressStream(state, &mut input_remain, &mut input, &mut output_remain, &mut output, ptr::null_mut())
    };

    Decode {
        input_remain,
        output_remain,
        status: match result {
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_ERROR => {
                let code = unsafe {
                    sys::BrotliDecoderGetErrorCode(state)
                };
                Err(DecodeError(code as _))
            }
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_SUCCESS => Ok(DecodeStatus::Finished),
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_INPUT => Ok(DecodeStatus::NeedInput),
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_OUTPUT => Ok(DecodeStatus::NeedOutput),
            other => Err(DecodeError(other)),
        },
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>> {
    match new_decoder() {
        Some(new) => {
            drop_fn(state);
            Some(new)
        }
        None => None,
    }
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    unsafe {
        sys::BrotliDecoderDestroyInstance(state.as_ptr() as _);
    }
}

#[inline]
fn describe_error_fn(code: i32) -> Option<&'static str> {
    let result = unsafe {
        sys::BrotliDecoderErrorString(code as _)
    };
    crate::utils::convert_c_str(result)
}
