use core::{slice, mem};

use super::{Decode, DecodeError, DecodeStatus, Decoder};
use crate::mem::brotli_rust::BrotliAllocator;
pub(crate) type Instance = brotli::BrotliState<BrotliAllocator, BrotliAllocator, BrotliAllocator>;

#[repr(transparent)]
///Decoder backed by [brotli](https://github.com/dropbox/rust-brotli)
pub struct BrotliRust {
    inner: Instance
}

impl BrotliRust {
    #[inline]
    ///Creates new instance
    pub fn new() -> Self {
        Self {
            inner: Instance::new(Default::default(), Default::default(), Default::default())
        }
    }
}

impl Decoder for BrotliRust {
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        let mut input_remain = input.len();
        let mut output_remain = output.len();
        //Potential UB but it is non-issue
        //Complain here
        //https://github.com/dropbox/rust-brotli/issues/177
        let output = unsafe {
            slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output_remain)
        };

        let result = brotli::BrotliDecompressStream(&mut input_remain, &mut 0, input, &mut output_remain, &mut 0, output, &mut 0, &mut self.inner);
        Decode {
            input_remain,
            output_remain,
            status: match result {
                brotli::BrotliResult::ResultSuccess => Ok(DecodeStatus::Finished),
                brotli::BrotliResult::NeedsMoreInput => Ok(DecodeStatus::NeedInput),
                brotli::BrotliResult::NeedsMoreOutput => Ok(DecodeStatus::NeedOutput),
                brotli::BrotliResult::ResultFailure => Err(DecodeError(self.inner.error_code as _)),
            },
        }
    }

    fn reset(&mut self) -> bool {
        *self = Self::new();
        true
    }

    fn describe_error(&self, code: DecodeError) -> Option<&'static str> {
        match code.0 {
            0 => Some("NO_ERROR"),
            //1 => Some("SUCCESS"),
            //2 => Some("NEEDS_MORE_INPUT"),
            //3 => Some("NEEDS_MORE_OUTPUT"),

            /* Errors caused by invalid input */
            -1 => Some("ERROR_FORMAT_EXUBERANT_NIBBLE"),
            -2 => Some("ERROR_FORMAT_RESERVED"),
            -3 => Some("ERROR_FORMAT_EXUBERANT_META_NIBBLE"),
            -4 => Some("ERROR_FORMAT_SIMPLE_HUFFMAN_ALPHABET"),
            -5 => Some("ERROR_FORMAT_SIMPLE_HUFFMAN_SAME"),
            -6 => Some("ERROR_FORMAT_FL_SPACE"),
            -7 => Some("ERROR_FORMAT_HUFFMAN_SPACE"),
            -8 => Some("ERROR_FORMAT_CONTEXT_MAP_REPEAT"),
            -9 => Some("ERROR_FORMAT_BLOCK_LENGTH_1"),
            -10 => Some("ERROR_FORMAT_BLOCK_LENGTH_2"),
            -11 => Some("ERROR_FORMAT_TRANSFORM"),
            -12 => Some("ERROR_FORMAT_DICTIONARY"),
            -13 => Some("ERROR_FORMAT_WINDOW_BITS"),
            -14 => Some("ERROR_FORMAT_PADDING_1"),
            -15 => Some("ERROR_FORMAT_PADDING_2"),
            -16 => Some("ERROR_FORMAT_DISTANCE"),

            /* -17..-18 codes are reserved */
            -19 => Some("ERROR_DICTIONARY_NOT_SET"),
            -20 => Some("ERROR_INVALID_ARGUMENTS"),

            /* Memory allocation problems */
            -21 => Some("ERROR_ALLOC_CONTEXT_MODES"),
            /* Literal => insert and distance trees together */
            -22 => Some("ERROR_ALLOC_TREE_GROUPS"),
            /* -23..-24 codes are reserved for distinct tree groups */
            -25 => Some("ERROR_ALLOC_CONTEXT_MAP"),
            -26 => Some("ERROR_ALLOC_RING_BUFFER_1"),
            -27 => Some("ERROR_ALLOC_RING_BUFFER_2"),
            /* -28..-29 codes are reserved for dynamic ring-buffer allocation */
            -30 => Some("ERROR_ALLOC_BLOCK_TYPE_TREES"),

            /* "Impossible" states */
            -31 => Some("ERROR_UNREACHABLE"),
            _ => None,
        }
    }
}
