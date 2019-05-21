//!Brotli encoder.

pub use compu_brotli_sys as sys;

use core::ptr;

#[derive(Copy, Clone)]
///Encoding mode
pub enum BrotliEncoderMode {
    ///Default mode. No assumptions about content.
    Generic,
    ///Text mode. UTF-8.
    Text,
    ///WOFF 2.0 mode
    Font
}

///!Brotli options
#[derive(Default, Clone)]
pub struct BrotliOptions {
    quality: Option<u32>,
    mode: Option<BrotliEncoderMode>,
}

impl BrotliOptions {
    #[inline(always)]
    ///Sets quality
    ///
    ///Allowed values are from 1 to 11.
    ///See brotli API docs for details.
    ///
    ///Default value is 11.
    pub fn quality(mut self, quality: u32) -> Self {
        self.quality = Some(quality);
        self
    }

    #[inline(always)]
    ///Sets mode
    pub fn mode(mut self, mode: BrotliEncoderMode) -> Self {
        self.mode = Some(mode);
        self
    }

    fn apply(&self, state: *mut sys::BrotliEncoderState) {
        unsafe {
            if let Some(quality) = self.quality {
                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_QUALITY, quality) != 0);
            }
            if let Some(mode) = self.mode {
                let mode = match mode {
                    BrotliEncoderMode::Generic => sys::BrotliEncoderMode_BROTLI_MODE_GENERIC,
                    BrotliEncoderMode::Text => sys::BrotliEncoderMode_BROTLI_MODE_TEXT,
                    BrotliEncoderMode::Font => sys::BrotliEncoderMode_BROTLI_MODE_FONT,
                } as u32;

                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_MODE, mode) != 0);
            }
        }
    }
}

///Brotli encoder.
pub struct BrotliEncoder {
    state: *mut sys::BrotliEncoderState,
}

impl super::Encoder for BrotliEncoder {
    type Options = BrotliOptions;

    fn new(opts: &Self::Options) -> Self {
        let state = unsafe {
            sys::BrotliEncoderCreateInstance(None, None, ptr::null_mut())
        };

        assert!(!state.is_null(), "Unable to create brotli encoder");

        opts.apply(state);

        Self {
            state
        }
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8], finish: bool) -> (usize, usize,  bool) {
        let mut avail_in = input.len();
        let mut avail_out = output.len();
        let mut input_ptr = input.as_ptr();
        let mut output_ptr = output.as_mut_ptr();

        let op = match finish {
            true => sys::BrotliEncoderOperation_BROTLI_OPERATION_FINISH,
            false => sys::BrotliEncoderOperation_BROTLI_OPERATION_PROCESS,
        };

        let result = unsafe {
            sys::BrotliEncoderCompressStream(self.state, op,
                                             &mut avail_in as *mut _, &mut input_ptr as *mut _,
                                             &mut avail_out as *mut _, &mut output_ptr as *mut _,
                                             ptr::null_mut())
        };

        (avail_in, avail_out, result != 0)
    }

    fn output<'a>(&'a mut self) -> Option<&'a [u8]> {
        let mut size = 0;

        let result = unsafe {
            sys::BrotliEncoderTakeOutput(self.state, &mut size)
        };

        match result.is_null() {
            true => None,
            false => Some(unsafe { core::slice::from_raw_parts(result, size) }),
        }
    }

    #[inline]
    fn compress_size_hint(&self, size: usize) -> usize {
        unsafe {
            sys::BrotliEncoderMaxCompressedSize(size) as usize
        }
    }

    #[inline]
    fn is_finished(&self) -> bool {
        unsafe {
            sys::BrotliEncoderIsFinished(self.state) != 0
        }
    }
}
