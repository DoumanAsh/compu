#[repr(u8)]
#[derive(Copy, Clone)]
///Encoding mode
pub enum BrotliEncoderMode {
    ///Default mode. No assumptions about content.
    Generic = 1,
    ///Text mode. UTF-8.
    Text,
    ///WOFF 2.0 mode
    Font
}

///Brotli options
#[derive(Default, Clone)]
pub struct BrotliOptions {
    pub(crate) inner: [u8; 2],
}

impl BrotliOptions {
    const QUALITY_IDX: usize = 0;
    const MODE_IDX: usize = 1;

    #[inline(always)]
    ///Creates default instance
    pub const fn new() -> Self {
        Self::from_raw([0; 2])
    }

    #[inline(always)]
    ///Creates default instance
    pub(crate) const fn from_raw(inner: [u8; 2]) -> Self {
        Self {
            inner,
        }
    }

    #[inline(always)]
    ///Sets quality
    ///
    ///Allowed values are from 1 to 11.
    ///See brotli API docs for details.
    ///
    ///Default value is 11.
    pub const fn quality(mut self, quality: u8) -> Self {
        assert!(quality > 0);
        assert!(quality <= 11);

        self.inner[Self::QUALITY_IDX] = quality;
        self
    }

    #[inline(always)]
    ///Sets mode
    pub const fn mode(mut self, mode: BrotliEncoderMode) -> Self {
        self.inner[Self::MODE_IDX] = mode as u8;
        self
    }

    #[cfg(feature = "brotli-c")]
    pub(crate) fn apply_c(&self, state: *mut compu_brotli_sys::BrotliEncoderState) {
        use compu_brotli_sys as sys;

        unsafe {
            let quality = self.inner[Self::QUALITY_IDX];
            if quality > 0 {
                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_QUALITY, quality as _) != 0);
            }
            let mode = self.inner[Self::MODE_IDX];
            if mode > 0 {
                debug_assert!(sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_MODE, mode as _) != 0);
            }
        }
    }

    #[cfg(feature = "brotli-rust")]
    pub(crate) fn apply_rust(&self, state: &mut crate::encoder::brotli::Instance) {
        let quality = self.inner[Self::QUALITY_IDX];
        if quality > 0 {
            debug_assert_ne!(
                brotli::enc::encode::BrotliEncoderSetParameter(
                    state,
                    brotli::enc::encode::BrotliEncoderParameter::BROTLI_PARAM_QUALITY,
                    quality as _,
                ),
                0
            )
        }

        let mode = self.inner[Self::MODE_IDX];
        if mode > 0 {
            debug_assert_ne!(
                brotli::enc::encode::BrotliEncoderSetParameter(
                    state,
                    brotli::enc::encode::BrotliEncoderParameter::BROTLI_PARAM_MODE,
                    mode as _,
                ),
                0
            )
        }
    }
}
