#[cfg(feature = "zlib-opt")]
use cloudflare_zlib_sys as sys;
#[cfg(not(feature = "zlib-opt"))]
mod sys {
    pub use libz_sys::*;
    use std::os::raw::{c_void, c_int, c_uint, c_ulong, c_ushort};

    pub const MAX_MEM_LEVEL: c_int = 8;

    #[repr(C)]
    pub struct inflate_state {
        pub strm: z_streamp,
        mode: c_int,
        last: c_int,
        wrap: c_int,
        havedict: c_int,
        flags: c_int,
        dmax: c_uint,
        check: c_ulong,
        total: c_ulong,
        head: *mut c_void,
        wbits: c_uint,
        wsize: c_uint,
        whave: c_uint,
        wnext: c_uint,
        window: *const u8,
        hold: c_ulong,
        bits: c_uint,
        length: c_uint,
        offset: c_uint,
        extra: c_uint,
        lencode: *const c_void,
        distcode: *const c_void,
        lenbits: c_uint,
        distbits: c_uint,
        ncode: c_uint,
        nlen: c_uint,
        ndist: c_uint,
        have: c_uint,
        next: *mut c_void,
        lens: [c_ushort; 320],
        work: [c_ushort; 288],
        codes: [c_uint; 1444],
        sane: c_int,
        back: c_int,
        was: c_uint,
    }

    #[inline(always)]
    #[allow(non_snake_case)]
    pub unsafe fn inflateInit2(strm: z_streamp, window_bits: c_int) -> c_int {
        inflateInit2_(strm, window_bits, libz_sys::zlibVersion(), core::mem::size_of::<z_stream>() as c_int)
    }

    #[inline(always)]
    #[allow(non_snake_case)]
    pub unsafe fn deflateInit2(strm: z_streamp, level: c_int, method: c_int, window_bits: c_int, mem_level: c_int, strategy: c_int,) -> c_int {
        deflateInit2_( strm, level, method, window_bits, mem_level, strategy, libz_sys::zlibVersion(), core::mem::size_of::<z_stream>() as c_int)
    }
}

pub use sys::*;
