//!Brotli decompressor.

pub use compu_brotli_sys as sys;

use super::DecoderResult;

use core::ptr;

impl Into<DecoderResult> for sys::BrotliDecoderResult {
    fn into(self) -> DecoderResult {
        match self {
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_ERROR => DecoderResult::Error,
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_SUCCESS => DecoderResult::Finished,
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_INPUT => DecoderResult::NeedInput,
            sys::BrotliDecoderResult_BROTLI_DECODER_RESULT_NEEDS_MORE_OUTPUT => DecoderResult::NeedOutput,
            //Safeguard against potential enum expansion
            //Afaik it is UB to match against value outside of C enum, so
            //to be on safe side we'll not use enums
            code => DecoderResult::Other(code),
        }
    }
}

///Brotli decompressor
pub struct BrotliDecoder {
    state: *mut sys::BrotliDecoderState
}

impl super::Decoder for BrotliDecoder {
    const HAS_INTERNAL_BUFFER: bool = true;
    type Options = ();

    fn new(_: &Self::Options) -> Self {
        let state = unsafe {
            sys::BrotliDecoderCreateInstance(None, None, ptr::null_mut())
        };

        assert!(!state.is_null(), "Unable to create brotli decoder");

        Self {
            state,
        }
    }

    fn output<'a>(&'a mut self) -> Option<&'a [u8]> {
        let mut size = 0;

        let result = unsafe {
            sys::BrotliDecoderTakeOutput(self.state, &mut size)
        };

        match result.is_null() {
            true => None,
            false => Some(unsafe { core::slice::from_raw_parts(result, size) }),
        }
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize, DecoderResult) {
        let mut avail_in = input.len();
        let mut avail_out = output.len();
        let mut input_ptr = input.as_ptr();
        let mut output_ptr = output.as_mut_ptr();

        let result = unsafe {
            sys::BrotliDecoderDecompressStream(self.state, &mut avail_in as *mut _, &mut input_ptr as *mut _,
                                                           &mut avail_out as *mut _, &mut output_ptr as *mut _,
                                                           ptr::null_mut())
        };

        (avail_in, avail_out, result.into())
    }

    fn is_finished(&self) -> bool {
        unsafe {
            sys::BrotliDecoderIsFinished(self.state) != 0
        }
    }
}

unsafe impl Send for BrotliDecoder {}

impl Drop for BrotliDecoder {
    fn drop(&mut self) {
        unsafe {
            sys::BrotliDecoderDestroyInstance(self.state);
        }
    }
}
