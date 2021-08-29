//! Zlib encoder

use core::ptr;

use crate::zlib_sys_wrap as sys;
use super::EncoderOp;

#[derive(Copy, Clone)]
///Compression strategy
pub enum ZlibStrategy {
    ///Default strategy.
    Default,
    ///Filtered strategy for data produced from filter.
    Filtered,
    ///Forces using Huffman encoding only, ignoring string matching.
    HuffmanOnly,
    ///Strategy optimized for PNG image.
    Rle,
    ///Prevents using dynamic Huffman codes.
    Fixed
}

impl Default for ZlibStrategy {
    fn default() -> Self {
        ZlibStrategy::Default
    }
}

#[derive(Copy, Clone)]
///Compression mode
pub enum ZlibMode {
    ///Uses raw deflate
    Deflate,
    ///Uses zlib header
    Zlib,
    ///Uses gzip header
    ///
    ///Default.
    Gzip,
}

impl Default for ZlibMode {
    fn default() -> Self {
        ZlibMode::Gzip
    }
}

///Zlib configuration for encoder.
pub struct ZlibOptions {
    mode: ZlibMode,
    strategy: ZlibStrategy,
    compression: i8,
}

impl ZlibOptions {
    #[inline]
    ///Sets zlib mode
    pub fn mode(mut self, new_mode: ZlibMode) -> Self {
        self.mode = new_mode;
        self
    }

    #[inline]
    ///Sets zlib strategy
    pub fn strategy(mut self, new_strategy: ZlibStrategy) -> Self {
        self.strategy = new_strategy;
        self
    }

    #[inline]
    ///Sets zlib compression in range from 1 to 9
    ///
    ///Defaults to 9.
    ///
    ///Use `-1` for zlib default.
    pub fn compression(mut self, compression: i8) -> Self {
        self.compression = compression;
        self
    }
}

impl Default for ZlibOptions {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            compression: 9,
            strategy: Default::default(),
        }
    }
}

///Zlib compressor
///
///Zlib doesn't have internal buffers so decoder can use
///own buffer to compensate, but it is not necessary
pub struct ZlibEncoder {
    state: sys::z_stream,
    is_finished: bool,
}

impl ZlibEncoder {
    fn update_state(&mut self) {
        #[cfg(not(feature = "zlib-opt"))]
        unsafe {
            let internal = self.state.state as *mut sys::inflate_state;
            (*internal).strm = &mut self.state as *mut _;
        }
    }
}

impl super::Encoder for ZlibEncoder {
    const HAS_INTERNAL_BUFFER: bool = false;
    type Options = ZlibOptions;

    fn new(opts: &Self::Options) -> Self {
        let mut state = sys::z_stream {
            next_in: ptr::null_mut(),
            avail_in: 0,
            total_in: 0,
            next_out: ptr::null_mut(),
            avail_out: 0,
            total_out: 0,
            msg: ptr::null_mut(),
            state: ptr::null_mut(),
            zalloc: crate::utils::compu_custom_alloc,
            zfree: crate::utils::compu_custom_free,
            opaque: ptr::null_mut(),
            data_type: 0,
            adler: 0,
            reserved: 0,
        };

        let max_bits = match opts.mode {
            ZlibMode::Deflate => -15,
            ZlibMode::Zlib => 15,
            ZlibMode::Gzip => 15 + 16,
        };

        let strategy = match opts.strategy {
            ZlibStrategy::Default => sys::Z_DEFAULT_STRATEGY,
            ZlibStrategy::Filtered => sys::Z_FILTERED,
            ZlibStrategy::HuffmanOnly => sys::Z_HUFFMAN_ONLY,
            ZlibStrategy::Rle => sys::Z_RLE,
            ZlibStrategy::Fixed => sys::Z_FIXED
        };

        let result = unsafe {
            sys::deflateInit2(&mut state, opts.compression as i32, sys::Z_DEFLATED, max_bits, sys::MAX_MEM_LEVEL, strategy)
        };

        assert_eq!(result, 0);

        Self {
            state,
            is_finished: false,
        }
    }

    fn encode(&mut self, input: &[u8], output: &mut [u8], op: EncoderOp) -> (usize, usize,  bool) {
        let op = match op {
            EncoderOp::Process => sys::Z_NO_FLUSH,
            EncoderOp::Flush => sys::Z_SYNC_FLUSH,
            EncoderOp::Finish => sys::Z_FINISH
        };

        self.state.avail_out = output.len() as u32;
        self.state.next_out = output.as_mut_ptr();

        self.state.avail_in = input.len() as u32;
        self.state.next_in = input.as_ptr() as *mut _;

        self.update_state();
        let result = unsafe {
            sys::deflate(&mut self.state, op)
        };

        let remaining_input = self.state.avail_in as usize;
        let remaining_output = self.state.avail_out as usize;

        let result = match result {
            sys::Z_STREAM_END => {
                self.is_finished = true;
                true
            },
            sys::Z_OK | sys::Z_BUF_ERROR => true,
            _ => false,
        };

        (remaining_input, remaining_output, result)
    }

    #[inline(always)]
    fn output<'a>(&'a mut self) -> Option<&'a [u8]> {
        None
    }

    #[inline]
    fn compress_size_hint(&self, _: usize) -> usize {
        0
    }

    #[inline(always)]
    fn is_finished(&self) -> bool {
        self.is_finished
    }
}

unsafe impl Send for ZlibEncoder {}

impl Drop for ZlibEncoder {
    fn drop(&mut self) {
        self.update_state();
        unsafe {
            sys::deflateEnd(&mut self.state);
        }
    }
}
