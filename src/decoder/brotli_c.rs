//! `brotli` interface implementation

use compu_brotli_sys as sys;

use core::{ptr, mem};

use super::{Decode, DecodeError, DecodeStatus, Decoder};
use crate::mem::{Box, Unique, compu_free_with_state, compu_malloc_with_state};

fn describe_error_fn(code: i32) -> Option<&'static str> {
    let result = unsafe {
        sys::BrotliDecoderErrorString(code as _)
    };
    crate::utils::convert_c_str(result)
}

#[repr(transparent)]
///Decoder backed by [brotli](https://github.com/DoumanAsh/compu-brotli-sys)
pub struct BrotliC {
    inner: ptr::NonNull<sys::BrotliDecoderState>
}

impl BrotliC {
    #[inline(always)]
    ///Creates new instance unless underlying C library is unable to create instance
    pub fn new() -> Option<Self> {
        let result = unsafe {
            sys::BrotliDecoderCreateInstance(Some(compu_malloc_with_state), Some(compu_free_with_state), ptr::null_mut())
        };

        ptr::NonNull::new(result).map(|inner| Self {
            inner
        })
    }
}

impl Decoder for BrotliC {
    #[inline]
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        let mut input_remain = input.len();
        let mut input = input.as_ptr();
        let mut output_remain = output.len();
        let mut output = output.as_mut_ptr() as *mut u8;
        let result = unsafe {
            sys::BrotliDecoderDecompressStream(self.inner.as_ptr(), &mut input_remain, &mut input, &mut output_remain, &mut output, ptr::null_mut())
        };

        Decode {
            input_remain,
            output_remain,
            status: match result {
                sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_ERROR => {
                    let code = unsafe {
                        sys::BrotliDecoderGetErrorCode(self.inner.as_ptr())
                    };
                    Err(DecodeError {
                        code: code as _,
                        describe_error_fn,
                    })
                }
                sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_SUCCESS => Ok(DecodeStatus::Finished),
                sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_INPUT => Ok(DecodeStatus::NeedInput),
                sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_OUTPUT => Ok(DecodeStatus::NeedOutput),
                code => Err(DecodeError {
                    code,
                    describe_error_fn,
                })
            },
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        match Self::new() {
            Some(new_instance) => {
                *self = new_instance;
                true
            },
            None => false,
        }
    }
}

impl Drop for BrotliC {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            sys::BrotliDecoderDestroyInstance(self.inner.as_ptr());
        }
    }
}

//These gets invalidated due to pointer usage, but in fact we're totally fine
unsafe impl Send for BrotliC {}
unsafe impl Sync for BrotliC {}

impl From<BrotliC> for Unique<dyn Decoder + Send + Sync> {
    #[inline(always)]
    fn from(value: BrotliC) -> Self {
        Unique::from_box(Box::new(value) as Box<dyn Decoder + Send + Sync>)
    }
}
