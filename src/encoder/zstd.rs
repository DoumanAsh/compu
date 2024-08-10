//! `zstd` interface implementation

use zstd_sys as sys;

use core::ptr;

use super::{Encode, EncodeOp, EncodeStatus, Encoder, Interface};
use crate::mem::compu_free_with_state;
use crate::mem::compu_malloc_with_state;

static ZSTD: Interface = Interface {
    drop_fn,
    reset_fn,
    encode_fn,
};

extern "C" {
    pub fn ZSTD_getErrorCode(result: usize) -> i32;
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
#[repr(i32)]
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
///ZSTD options.
///
///For details refer to their crappy documentation: `http://facebook.github.io/zstd/zstd_manual.html#Chapter5`
pub struct ZstdOptions {
    level: i32,
    strategy: ZstdStrategy,
    window_log: i32,
}

impl ZstdOptions {
    #[inline(always)]
    ///Creates new default value
    pub const fn new() -> Self {
        Self {
            level: sys::ZSTD_CLEVEL_DEFAULT as _,
            strategy: ZstdStrategy::Default,
            window_log: sys::ZSTD_WINDOWLOG_LIMIT_DEFAULT as _,
        }
    }

    #[inline(always)]
    ///Sets level
    pub const fn level(mut self, level: i32) -> Self {
        assert!(level <= sys::ZSTD_TARGETLENGTH_MAX as i32);
        assert!(level >= -(sys::ZSTD_TARGETLENGTH_MAX as i32));
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

impl Interface {
    #[inline]
    ///Creates encoder with `zstd` interface
    ///
    ///Returns `None` if unable to initialize it (likely due to lack of memory)
    pub fn zstd(opts: ZstdOptions) -> Option<Encoder> {
        let allocator = sys::ZSTD_customMem {
            customAlloc: Some(compu_malloc_with_state),
            customFree: Some(compu_free_with_state),
            opaque: ptr::null_mut(),
        };
        let ctx = unsafe {
            sys::ZSTD_createCStream_advanced(allocator)
        };
        match ptr::NonNull::new(ctx).and_then(|ctx| opts.apply(ctx)) {
            Some(ctx) => Some(ZSTD.inner_encoder(ctx.cast(), [0; 2])),
            None => None,
        }
    }
}

unsafe fn encode_fn(state: ptr::NonNull<u8>, input: *const u8, input_remain: usize, output: *mut u8, output_remain: usize, op: EncodeOp) -> Encode {
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
        sys::ZSTD_compressStream2(state.cast().as_ptr(), &mut output, &mut input, op.into_zstd())
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
            size if sys::ZSTD_isError(size) == 0 => {
                if output.pos == output.size {
                    EncodeStatus::NeedOutput
                } else {
                    EncodeStatus::Continue
                }
            }
            size => match ZSTD_getErrorCode(size) {
                //https://github.com/facebook/zstd/blob/dev/lib/zstd_errors.h#L64
                70 | 80 => EncodeStatus::NeedOutput,
                _ => EncodeStatus::Error,
            },
        },
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>, _: [u8; 2]) -> Option<ptr::NonNull<u8>> {
    let result = unsafe {
        sys::ZSTD_CCtx_reset(state.cast().as_ptr(), sys::ZSTD_ResetDirective::ZSTD_reset_session_only)
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
        sys::ZSTD_freeCStream(state.cast().as_ptr())
    };
    debug_assert_eq!(result, 0);
}
