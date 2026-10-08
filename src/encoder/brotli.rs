//! `brotli` interface implementation

use super::brotli_common::BrotliOptions;
use super::{Encode, EncodeOp, EncodeStatus, Encoder};
use crate::mem::brotli_rust::BrotliAllocator;
use core::slice;

pub(crate) type Instance = brotli::enc::encode::BrotliEncoderStateStruct<BrotliAllocator>;

///Encoder backed by [brotli](https://github.com/dropbox/rust-brotli)
pub struct BrotliRust {
    inner: Instance,
    options: BrotliOptions,
}

impl BrotliRust {
    #[inline(always)]
    ///Creates new instance
    pub fn new(options: BrotliOptions) -> Self {
        let mut inner = Instance::new(Default::default());
        options.apply_rust(&mut inner);
        Self {
            inner,
            options,
        }
    }
}

impl Encoder for BrotliRust {
    #[inline]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [core::mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        let mut input_remain = input.len();
        let mut output_remain = output.len();
        //Potential UB but it is non-issue
        //Complain here
        //https://github.com/dropbox/rust-brotli/issues/177
        let output = unsafe {
            slice::from_raw_parts_mut(output.as_mut_ptr() as *mut _, output_remain)
        };

        let result = self.inner.compress_stream(op.into_rust_brotli(),
            &mut input_remain, input, &mut 0,
            &mut output_remain, output, &mut 0,
            &mut None, &mut |_a, _b, _c, _d| ()
        );
        let has_more_output = self.inner.has_more_output();

        Encode {
            input_remain,
            output_remain,
            status: match result {
                false => match has_more_output {
                    false => EncodeStatus::Error,
                    true => EncodeStatus::NeedOutput,
                },
                true => {
                    if has_more_output {
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
        *self = Self::new(self.options);
        true
    }
}

impl EncodeOp {
    #[inline(always)]
    const fn into_rust_brotli(self) -> brotli::enc::encode::BrotliEncoderOperation {
        match self {
            Self::Process => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_PROCESS,
            Self::Flush => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_FLUSH,
            Self::Finish => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
        }
    }
}
