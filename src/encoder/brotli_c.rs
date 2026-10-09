//! `brotli` interface implementation

use compu_brotli_sys as sys;

use core::ptr;

use super::brotli_common::BrotliOptions;
use super::{Encode, EncodeOp, EncodeStatus, Encoder};
use crate::mem::{compu_free_with_state, compu_malloc_with_state};

///Encoder backed by [brotli](https://github.com/DoumanAsh/compu-brotli-sys)
pub struct BrotliC {
    inner: ptr::NonNull<sys::BrotliEncoderState>,
    options: BrotliOptions,
}

impl BrotliC {
    #[inline(always)]
    ///Creates new instance unless underlying C library is unable to create instance
    pub fn new(options: BrotliOptions) -> Option<Self> {
        let result = unsafe {
            sys::BrotliEncoderCreateInstance(Some(compu_malloc_with_state), Some(compu_free_with_state), ptr::null_mut())
        };

        ptr::NonNull::new(result).map(|inner| {
            options.apply_c(inner.as_ptr());
            Self {
                inner,
                options
            }
        })
    }
}

impl Encoder for BrotliC {
    #[inline]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [core::mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        let mut input_remain = input.len();
        let mut output_remain = output.len();
        let result = unsafe {
            sys::BrotliEncoderCompressStream(
                self.inner.as_ptr() as _, op.into_brotli(),
                &mut input_remain, &mut input.as_ptr(),
                &mut output_remain, &mut (output.as_mut_ptr() as _),
                ptr::null_mut(),
            )
        };

        let has_more_output = unsafe {
            sys::BrotliEncoderHasMoreOutput(self.inner.as_ptr() as _)
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
            },
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        match Self::new(self.options) {
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
            sys::BrotliEncoderDestroyInstance(self.inner.as_ptr());
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

//These gets invalidated due to pointer usage, but in fact we're totally fine
unsafe impl Send for BrotliC {}
unsafe impl Sync for BrotliC {}
