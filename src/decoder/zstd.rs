//! `zstd` interface implementation

use zstd_sys as sys;

use core::{mem, ptr};

use super::{Decode, DecodeError, DecodeStatus, Decoder};
use crate::mem::{Unique, compu_free_with_state, compu_malloc_with_state};

fn describe_error_fn(code: i32) -> Option<&'static str> {
    let result = unsafe {
        sys::ZSTD_getErrorName(code as _)
    };
    crate::utils::convert_c_str(result)
}

#[inline(always)]
fn dtor<T: ?Sized>(value: ptr::NonNull<T>) {
    let result = unsafe {
        sys::ZSTD_freeDStream(value.cast().as_ptr())
    };
    debug_assert_eq!(result, 0);
}

#[repr(transparent)]
///Decoder backed by [zstd](https://github.com/gyscos/zstd-rs)
pub struct ZstdC {
    inner: ptr::NonNull<sys::ZSTD_DCtx>
}

impl ZstdC {
    #[inline]
    ///Creates new instance unless underlying C library is unable to create instance
    pub fn new(options: ZstdOptions) -> Option<Self> {
       let allocator = sys::ZSTD_customMem {
            customAlloc: Some(compu_malloc_with_state),
            customFree: Some(compu_free_with_state),
            opaque: ptr::null_mut(),
        };
        let ctx = unsafe {
            sys::ZSTD_createDStream_advanced(allocator)
        };
        match ptr::NonNull::new(ctx).and_then(|ctx| options.apply(ctx)) {
            Some(inner) => Some(Self {
                inner
            }),
            None => None,
        }
    }
}

impl Decoder for sys::ZSTD_DCtx {
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        let mut input = sys::ZSTD_inBuffer_s {
            src: input.as_ptr() as _,
            size: input.len(),
            pos: 0,
        };
        let mut output = sys::ZSTD_outBuffer_s {
            dst: output.as_mut_ptr() as _,
            size: output.len(),
            pos: 0,
        };
        let result = unsafe {
            sys::ZSTD_decompressStream(self, &mut output, &mut input)
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
                    } else if unsafe {sys::ZSTD_isError(size) } == 0 {
                        //Not error, means it was able to flush out everything it had
                        Ok(DecodeStatus::NeedInput)
                    } else {
                        Err(DecodeError {
                            code: size as _,
                            describe_error_fn,
                        })
                    }
                }
            },
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        let result = unsafe {
            sys::ZSTD_DCtx_reset(self, sys::ZSTD_ResetDirective::ZSTD_reset_session_only)
        };
        result == 0
    }
}

impl Decoder for ZstdC {
    #[inline(always)]
    fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        unsafe {
            self.inner.as_mut().decode_uninit(input, output)
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        unsafe {
            self.inner.as_mut().reset()
        }
    }
}

impl From<ZstdC> for Unique<dyn Decoder + Send + Sync> {
    #[inline(always)]
    fn from(value: ZstdC) -> Self {
        let ptr = unsafe {
            ptr::NonNull::new_unchecked(value.inner.as_ptr() as *mut (dyn Decoder + Send + Sync))
        };
        let result = Unique::new(ptr, dtor);
        mem::forget(value);
        result
    }
}

impl Drop for ZstdC {
    #[inline(always)]
    fn drop(&mut self) {
        dtor(self.inner);
    }
}

#[derive(Copy, Clone)]
///ZSTD options.
///
///For details refer to documentation: `http://facebook.github.io/zstd/zstd_manual.html#Chapter6`
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
