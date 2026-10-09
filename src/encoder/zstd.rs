//! `zstd` interface implementation

use zstd_sys as sys;

use core::ptr;

use super::{Encode, EncodeOp, EncodeStatus, Encoder};
use crate::mem::compu_free_with_state;
use crate::mem::compu_malloc_with_state;

extern "C" {
    pub fn ZSTD_getErrorCode(result: usize) -> i32;
}

#[repr(transparent)]
///Encoder backed by [zstd](https://github.com/gyscos/zstd-rs)
pub struct ZstdC {
    inner: ptr::NonNull<sys::ZSTD_CCtx>
}

impl ZstdC {
    #[inline]
    ///Creates new instance unless underlying C library is unable to create instance
    pub fn new(opts: ZstdOptions) -> Option<Self> {
        let allocator = sys::ZSTD_customMem {
            customAlloc: Some(compu_malloc_with_state),
            customFree: Some(compu_free_with_state),
            opaque: ptr::null_mut(),
        };
        let ctx = unsafe {
            sys::ZSTD_createCStream_advanced(allocator)
        };
        match ptr::NonNull::new(ctx).and_then(|ctx| opts.apply(ctx)) {
            Some(inner) => Some(ZstdC {
                inner
            }),
            None => None,
        }
    }
}

impl Encoder for ZstdC {
    #[inline]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [core::mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
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
            sys::ZSTD_compressStream2(self.inner.as_ptr(), &mut output, &mut input, op.into_zstd())
        };
        Encode {
            input_remain: input.size - input.pos,
            output_remain: output.size - output.pos,
            status: match result {
                //0 always mean there is nothing else to do.
                //so if user requested finish, then frame is done
                0 => match op {
                    EncodeOp::Finish => EncodeStatus::Finished,
                    _ => EncodeStatus::Continue,
                },
                //Made some progress, but not completely
                //Try to guess what it means, especially problematic for `EncodeOp::Process` as zstd is
                //allowed not to consume output as whole
                size if unsafe { sys::ZSTD_isError(size) } == 0 => {
                    if output.pos == output.size {
                        EncodeStatus::NeedOutput
                    } else {
                        EncodeStatus::Continue
                    }
                }
                size => match unsafe { ZSTD_getErrorCode(size) } {
                    //https://github.com/facebook/zstd/blob/dev/lib/zstd_errors.h#L64
                    70 | 80 => EncodeStatus::NeedOutput,
                    _ => EncodeStatus::Error,
                },
            },
        }
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        let result = unsafe {
            sys::ZSTD_CCtx_reset(self.inner.as_ptr(), sys::ZSTD_ResetDirective::ZSTD_reset_session_only)
        };
        result == 0
    }
}

impl Drop for ZstdC {
    #[inline(always)]
    fn drop(&mut self) {
        let result = unsafe {
            sys::ZSTD_freeCStream(self.inner.as_ptr())
        };
        debug_assert_eq!(result, 0);
    }
}

impl EncodeOp {
    #[inline(always)]
    const fn into_zstd(self) -> sys::ZSTD_EndDirective {
        match self {
            Self::Process => sys::ZSTD_EndDirective::ZSTD_e_continue,
            Self::Flush => sys::ZSTD_EndDirective::ZSTD_e_flush,
            Self::Finish => sys::ZSTD_EndDirective::ZSTD_e_end,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(u8)]
///Possible enumeration of strategies from fastest to slowest
pub enum ZstdStrategy {
    ///As name implies
    Default = 0,
    ///ZSTD_fast
    Fast = 1,
    ///ZSTD_dfast
    DFast = 2,
    ///ZSTD_greedy
    Greedy = 3,
    ///ZSTD_lazy
    Lazy = 4,
    ///ZSTD_lazy2
    Lazy2 = 5,
    ///ZSTD_btlazy2
    BtLazy2 = 6,
    ///ZSTD_btopt
    BtOpt = 7,
    ///ZSTD_btultra
    BtUltra = 8,
    ///ZSTD_btultra2
    BtUltra2 = 9,
}

#[derive(Copy, Clone)]
#[repr(u8)]
#[allow(missing_docs)]
///Possible compression levels provided by zstd library from lowest `L1` to highest compression of `L22`
///
///The higher compression level, the bigger CPU and memory requirements are.
pub enum ZstdLevel {
    ///Lowest compression level
    L1 = 1,
    L2 = 2,
    ///Default level
    L3 = 3,
    L4 = 4,
    L5 = 5,
    L6 = 6,
    L7 = 7,
    L8 = 8,
    L9 = 9,
    L10 = 10,
    L11 = 11,
    L12 = 12,
    L13 = 13,
    L14 = 14,
    L15 = 15,
    L16 = 16,
    L17 = 17,
    L18 = 18,
    ///Reasonable maximum compression level recommended for use
    ///
    ///Anything higher is only useful when compression level is all that matters
    L19 = 19,
    L20 = 20,
    L21 = 21,
    ///Highest compression level
    L22 = 22,
}

impl Default for ZstdLevel {
    #[inline(always)]
    fn default() -> Self {
        Self::L3
    }
}

#[derive(Copy, Clone)]
///ZSTD options.
pub struct ZstdOptions {
    level: ZstdLevel,
    strategy: ZstdStrategy,
    window_log: i32,
}

impl ZstdOptions {
    #[inline(always)]
    ///Creates new default value
    pub const fn new() -> Self {
        Self {
            level: ZstdLevel::L3,
            strategy: ZstdStrategy::Default,
            window_log: sys::ZSTD_WINDOWLOG_LIMIT_DEFAULT as _,
        }
    }

    #[inline(always)]
    ///Sets level in range of `1..=22`
    pub const fn level(mut self, level: ZstdLevel) -> Self {
        self.level = level;
        self
    }

    #[inline(always)]
    ///Sets strategy
    pub const fn strategy(mut self, strategy: ZstdStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    #[inline(always)]
    ///Sets window_log
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
    fn apply(&self, ctx: ptr::NonNull<sys::ZSTD_CCtx>) -> Option<ptr::NonNull<sys::ZSTD_CCtx>> {
        macro_rules! set {
            ($field:ident => $param:ident) => {{
                unsafe {
                    let result = sys::ZSTD_isError(sys::ZSTD_CCtx_setParameter(ctx.as_ptr(), sys::ZSTD_cParameter::$param, self.$field as _));
                    if result != 0 {
                        return None;
                    }
                }
            }};
        }

        set!(level => ZSTD_c_compressionLevel);
        set!(strategy => ZSTD_c_strategy);
        set!(window_log => ZSTD_c_windowLog);

        Some(ctx)
    }
}

impl Default for ZstdOptions {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

//These gets invalidated due to pointer usage, but in fact we're totally fine
unsafe impl Send for ZstdC {}
unsafe impl Sync for ZstdC {}
