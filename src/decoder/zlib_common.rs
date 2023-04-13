#[derive(Copy, Clone)]
#[repr(i8)]
///Decompression mode
pub enum ZlibMode {
    ///Assumes raw deflate
    Deflate = -15,
    ///Assumes zlib header
    Zlib = 15,
    ///Assumes gzip header
    Gzip = 15 + 16,
    ///Automatically detect header.
    ///
    ///Default value.
    Auto = 15 + 32,
}

impl ZlibMode {
    #[inline(always)]
    pub(crate) const fn max_bits(self) -> core::ffi::c_int {
        self as _
    }
}

impl Default for ZlibMode {
    #[inline(always)]
    fn default() -> Self {
        Self::Auto
    }
}
