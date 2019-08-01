//! Zlib encoder

#[cfg(feature = "zlib-opt")]
use cloudflare_zlib_sys as sys;
#[cfg(not(feature = "zlib-opt"))]
mod sys {
    pub use libz_sys::*;
    use std::os::raw::c_int;
    pub const MAX_MEM_LEVEL: c_int = 8;

    #[inline(always)]
    #[allow(non_snake_case)]
    pub unsafe fn deflateInit2(strm: z_streamp, level: c_int, method: c_int, window_bits: c_int, mem_level: c_int, strategy: c_int,) -> c_int {
        deflateInit2_( strm, level, method, window_bits, mem_level, strategy, libz_sys::zlibVersion(), core::mem::size_of::<z_stream>() as c_int)
    }
}

use super::EncoderOp;

use core::mem;

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
    #[cfg(not(feature = "zlib-opt"))]
    state: &'static mut sys::z_stream,
    #[cfg(feature = "zlib-opt")]
    state: sys::z_stream,
    is_finished: bool,
}

impl super::Encoder for ZlibEncoder {
    const HAS_INTERNAL_BUFFER: bool = false;
    type Options = ZlibOptions;

    fn new(opts: &Self::Options) -> Self {
        #[cfg(not(feature = "zlib-opt"))]
        let state = Box::leak(Box::new(unsafe { mem::zeroed() }));
        #[cfg(feature = "zlib-opt")]
        let mut state = unsafe { mem::zeroed() };

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

        #[cfg(not(feature = "zlib-opt"))]
        let result = unsafe {
            sys::deflateInit2(state, opts.compression as i32, sys::Z_DEFLATED, max_bits, sys::MAX_MEM_LEVEL, strategy)
        };
        #[cfg(feature = "zlib-opt")]
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

        #[cfg(not(feature = "zlib-opt"))]
        let result = unsafe {
            sys::deflate(self.state, op)
        };
        #[cfg(feature = "zlib-opt")]
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
            other => {
                println!("Error: {}", other);
                false
            },
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

impl Drop for ZlibEncoder {
    fn drop(&mut self) {
        unsafe {
            #[cfg(feature = "zlib-opt")]
            sys::deflateEnd(&mut self.state);

            #[cfg(not(feature = "zlib-opt"))]
            sys::deflateEnd(self.state);
            #[cfg(not(feature = "zlib-opt"))]
            Box::from_raw(self.state);
        }
    }
}
