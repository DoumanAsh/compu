//! `zstd` interface implementation

use zstd_sys as sys;

use core::ptr;

use super::{Decode, DecodeError, DecodeStatus, Decoder, Interface};
use crate::mem::compu_free_with_state;
use crate::mem::compu_malloc_with_state;

static ZSTD: Interface = Interface {
    drop_fn,
    reset_fn,
    decode_fn,
    describe_error_fn,
};

#[derive(Copy, Clone)]
///ZSTD options.
///
///For details refer to their crappy documentation: `http://facebook.github.io/zstd/zstd_manual.html#Chapter6`
pub struct ZstdOptions {
    window_log: i32,
}

impl ZstdOptions {
    #[inline(always)]
    ///Creates new default value
    pub const fn new() -> Self {
        Self {
            window_log: 0
        }
    }

    #[inline(always)]
    ///Sets window_log
    ///
    ///This acts as cap on window_log, refusing to decompress anything above it.
    ///Normally, default value is all you need.
    pub const fn window_log(mut self, window_log: i32) -> Self {
        #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
        assert!(window_log <= sys::ZSTD_WINDOWLOG_MAX_32 as i32);
        #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
        assert!(window_log <= sys::ZSTD_WINDOWLOG_MAX_64 as i32);
        assert!(window_log >= sys::ZSTD_WINDOWLOG_MIN as i32);
        self.window_log = window_log;
        self
    }

    #[inline(always)]
    fn apply(&self, ctx: ptr::NonNull<sys::ZSTD_DCtx>) -> Option<ptr::NonNull<sys::ZSTD_DCtx>> {
        macro_rules! set {
            ($field:ident => $param:ident) => {{
                unsafe {
                    let result = sys::ZSTD_isError(sys::ZSTD_DCtx_setParameter(ctx.as_ptr(), sys::ZSTD_dParameter::$param, self.$field as _));
                    if result != 0 {
                        return None;
                    }
                }
            }};
        }

        set!(window_log => ZSTD_d_windowLogMax);

        Some(ctx)
    }
}

impl Default for ZstdOptions {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl Interface {
    #[inline]
    ///Creates decoder with `zstd` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zstd(opts: ZstdOptions) -> Option<Decoder> {
        let allocator = sys::ZSTD_customMem {
            customAlloc: Some(compu_malloc_with_state),
            customFree: Some(compu_free_with_state),
            opaque: ptr::null_mut(),
        };
        let ctx = unsafe {
            sys::ZSTD_createDStream_advanced(allocator)
        };
        match ptr::NonNull::new(ctx).and_then(|ctx| opts.apply(ctx)) {
            Some(ctx) => Some(ZSTD.inner_decoder(ctx.cast())),
            None => None,
        }
    }
}

#[inline]
unsafe fn decode_fn(state: ptr::NonNull<u8>, input: *const u8, input_remain: usize, output: *mut u8, output_remain: usize) -> Decode {
    let mut input = sys::ZSTD_inBuffer_s {
        src: input as _,
        size: input_remain,
        pos: 0,
    };
    let mut output = sys::ZSTD_outBuffer_s {
        dst: output as _,
        size: output_remain,
        pos: 0,
    };
    let result = unsafe {
        sys::ZSTD_decompressStream(state.cast().as_ptr(), &mut output, &mut input)
    };

    Decode {
        input_remain: input.size - input.pos,
        output_remain: output.size - output.pos,
        status: match result {
            0 => Ok(DecodeStatus::Finished),
            //Unfortunately error handling in zstd is shit
            //non-zero return value means that we're not done or it is error.
            //ZSTD_decompressStream() always flushes to maximum, so if there is not enough space,
            //we should check it first, otherwise assume we need more input.
            //Even though they have error code 70 to indicate output not having enough space
            //they do not necessary use it
            size => {
                if output.pos == output.size {
                    Ok(DecodeStatus::NeedOutput)
                } else if sys::ZSTD_isError(size) == 0 {
                    //Not error, means it was able to flush out everything it had
                    Ok(DecodeStatus::NeedInput)
                } else {
                    Err(DecodeError(size as _))
                }
            }
        },
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>> {
    let result = unsafe {
        sys::ZSTD_DCtx_reset(state.cast().as_ptr(), sys::ZSTD_ResetDirective::ZSTD_reset_session_only)
    };
    if result == 0 {
        Some(state)
    } else {
        None
    }
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    let result = unsafe {
        sys::ZSTD_freeDStream(state.cast().as_ptr())
    };
    debug_assert_eq!(result, 0);
}

#[inline]
fn describe_error_fn(code: i32) -> Option<&'static str> {
    let result = unsafe {
        sys::ZSTD_getErrorName(code as _)
    };
    crate::utils::convert_c_str(result)
}
