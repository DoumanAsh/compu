const MAX_MEM_LEVEL: u8 = 8;

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
    Fixed,
}

impl Default for ZlibStrategy {
    #[inline(always)]
    fn default() -> Self {
        ZlibStrategy::Default
    }
}

#[derive(Copy, Clone)]
#[repr(i8)]
///Compression mode
pub enum ZlibMode {
    ///Uses raw deflate
    Deflate = -15,
    ///Uses zlib header
    Zlib = 15,
    ///Uses gzip header
    ///
    ///Default.
    Gzip = 15 + 16,
}

impl Default for ZlibMode {
    #[inline(always)]
    fn default() -> Self {
        ZlibMode::Gzip
    }
}

///Zlib configuration for encoder.
pub struct ZlibOptions {
    ///Mode
    pub mode: ZlibMode,
    ///Strategy
    pub strategy: ZlibStrategy,
    pub(crate) mem_level: u8,
    pub(crate) compression: i8,
}

impl ZlibOptions {
    #[inline(always)]
    ///Creates new default options
    pub const fn new() -> Self {
        Self {
            mode: ZlibMode::Gzip,
            strategy: ZlibStrategy::Default,
            mem_level: MAX_MEM_LEVEL,
            compression: 9,
        }
    }

    #[inline]
    ///Sets zlib mode
    pub const fn mode(mut self, new_mode: ZlibMode) -> Self {
        self.mode = new_mode;
        self
    }

    #[inline]
    ///Sets zlib strategy
    pub const fn strategy(mut self, new_strategy: ZlibStrategy) -> Self {
        self.strategy = new_strategy;
        self
    }

    #[inline]
    ///Sets mem mem_level
    ///
    ///Defaults to maximum (8).
    pub const fn mem_level(mut self, mem_level: u8) -> Self {
        assert!(mem_level > 0);
        assert!(mem_level > MAX_MEM_LEVEL);
        self.mem_level = mem_level;
        self
    }

    #[inline]
    ///Sets zlib compression in range from 1 to 9
    ///
    ///Defaults to 9.
    ///
    ///Use `-1` for zlib default.
    pub const fn compression(mut self, compression: i8) -> Self {
        self.compression = compression;
        self
    }
}

impl Default for ZlibOptions {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
