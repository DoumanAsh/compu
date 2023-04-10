//Signature:
//
// # Zlib
//
//      FLEVEL: 0       1       2       3
//CINFO:
//     0      08 1D   08 5B   08 99   08 D7
//     1      18 19   18 57   18 95   18 D3
//     2      28 15   28 53   28 91   28 CF
//     3      38 11   38 4F   38 8D   38 CB
//     4      48 0D   48 4B   48 89   48 C7
//     5      58 09   58 47   58 85   58 C3
//     6      68 05   68 43   68 81   68 DE
//     7      78 01   78 5E   78 9C   78 DA
//
// # GZIP
//
// Just constant `1F 8B`

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
