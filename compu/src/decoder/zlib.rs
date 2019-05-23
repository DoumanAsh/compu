//! Zlib decoder

use cloudflare_zlib_sys as sys;

use super::DecoderResult;

use core::{mem};

#[derive(Copy, Clone)]
///Decompression mode
pub enum ZlibMode {
    ///Assumes raw deflate
    Deflate,
    ///Assumes zlib header
    Zlib,
    ///Assumes gzip header
    Gzip,
    ///Automatically detect header.
    ///
    ///Default value.
    Auto,
}

impl Default for ZlibMode {
    fn default() -> Self {
        ZlibMode::Auto
    }
}

#[derive(Default)]
///Zlib configuration for decoder.
pub struct ZlibOptions {
    mode: ZlibMode,
}

impl ZlibOptions {
    #[inline]
    ///Sets zlib mode
    pub fn mode(mut self, new_mode: ZlibMode) -> Self {
        self.mode = new_mode;
        self
    }
}

///Zlib decompressor
///
///Zlib doesn't have internal buffers so decoder can use
///own buffer to compensate, but it is not necessary
pub struct ZlibDecoder {
    state: sys::z_stream,
    is_finished: bool,
}

impl super::Decoder for ZlibDecoder {
    const HAS_INTERNAL_BUFFER: bool = false;
    type Options = ZlibOptions;

    fn new(options: &Self::Options) -> Self {
        let mut state = unsafe { mem::zeroed() };

        let max_bits = match options.mode {
            ZlibMode::Auto => 15 + 32,
            ZlibMode::Deflate => 15,
            ZlibMode::Zlib => -15,
            ZlibMode::Gzip => 15 + 16,
        };

        let result = unsafe {
            sys::inflateInit2(&mut state, max_bits)
        };

        assert_eq!(result, 0);

        Self {
            state,
            is_finished: false,
        }
    }

    fn output<'a>(&'a mut self) -> Option<&'a [u8]> {
        None
    }

    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize, DecoderResult) {
        self.state.avail_out = output.len() as u32;
        self.state.next_out = output.as_mut_ptr();

        self.state.avail_in = input.len() as u32;
        self.state.next_in = input.as_ptr() as *mut _;

        let result = unsafe {
            sys::inflate(&mut self.state, 0)
        };

        let remaining_input = self.state.avail_in as usize;
        let remaining_output = self.state.avail_out as usize;

        let result = match result {
            sys::Z_OK => match remaining_input > 0 {
                //TODO: check if output is zero?
                //      it seems if output is insufficient then zlib exits
                //      but as there was some progress it returns Z_OK
                true => DecoderResult::NeedOutput,
                false => DecoderResult::NeedInput,
            },
            sys::Z_STREAM_END => {
                self.is_finished = true;
                DecoderResult::Finished
            },
            sys::Z_BUF_ERROR => DecoderResult::NeedOutput,
            error => DecoderResult::Other(error)
        };

        (remaining_input, remaining_output, result)
    }

    #[inline(always)]
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}

impl Drop for ZlibDecoder {
    fn drop(&mut self) {
        unsafe {
            sys::inflateEnd(&mut self.state);
        }
    }
}
