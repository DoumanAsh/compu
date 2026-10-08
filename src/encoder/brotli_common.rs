#[repr(u8)]
#[derive(Copy, Clone)]
///Encoding mode
enum BrotliEncoderMode {
    ///Default mode. No assumptions about content.
    Generic = 1,
    ///Text mode. UTF-8.
    Text,
    ///WOFF 2.0 mode
    Font,
}

impl BrotliEncoderMode {
    #[inline(always)]
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Generic)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
///Brotli encoder quality value
pub struct BrotliQuality(u8);

impl BrotliQuality {
    ///One pass over each fragment with static entropy codes: fastest, largest.
    pub const Q0: Self = Self::new_raw(0);
    ///Two passes over each fragment, with entropy codes built per block.
    pub const Q1: Self = Self::new_raw(1);
    ///Greedy matching, with the format’s fixed command and distance codes.
    pub const Q2: Self = Self::new_raw(2);
    ///Greedy matching with one prefix code for the whole stream.
    pub const Q3: Self = Self::new_raw(3);
    ///Adds block splitting, histogram optimisation and distance parameters.
    pub const Q4: Self = Self::new_raw(4);
    ///Adds a delayed search and literal context modelling.
    pub const Q5: Self = Self::new_raw(5);
    ///Deepens the search to thirty-two bucket candidates.
    pub const Q6: Self = Self::new_raw(6);
    ///Deepens it again and adds the three-context literal model.
    pub const Q7: Self = Self::new_raw(7);
    ///Deeper still, with more cached distances checked per position.
    pub const Q8: Self = Self::new_raw(8);
    ///The deepest greedy search: 256 bucket candidates, 16 cached distances.
    pub const Q9: Self = Self::new_raw(9);
    ///A Zopfli search over every match the binary tree finds.
    pub const Q10: Self = Self::new_raw(10);
    ///The same search run harder and re-priced: slowest, smallest.
    pub const Q11: Self = Self::new_raw(11);
    ///Recommended value for real time web applications
    pub const REC: Self = Self::Q6;
    ///Maximum possible compression, not suitable for performance sensitive operations
    pub const MAX: Self = Self::Q11;

    const fn new_raw(quality: u8) -> Self {
        assert!(quality <= 11);
        Self(quality)
    }
}

///Brotli options
#[derive(Copy, Clone)]
pub struct BrotliOptions {
    mode: BrotliEncoderMode,
    quality: BrotliQuality,
}

impl BrotliOptions {
    #[inline(always)]
    ///Creates default option set with [BrotliQuality::REC]
    pub const fn new() -> Self {
        Self {
            mode: BrotliEncoderMode::Generic,
            quality: BrotliQuality::REC,
        }
    }

    #[inline(always)]
    ///Sets quality
    ///
    ///Allowed values are from 1 to 11.
    ///See brotli API docs for details.
    ///
    ///Default value is [BrotliQuality::REC].
    pub const fn quality(mut self, quality: BrotliQuality) -> Self {
        self.quality = quality;
        self
    }

    ///Sets text mode optimized for processing UTF-8 strings
    pub const fn with_text_mode(mut self) -> Self {
        self.mode = BrotliEncoderMode::Text;
        self
    }

    ///Sets font mode optimized for processing WOFF 2.0 web fonts
    pub const fn with_font_mode(mut self) -> Self {
        self.mode = BrotliEncoderMode::Font;
        self
    }

    #[cfg(feature = "brotli-c")]
    #[inline(always)]
    pub(crate) fn apply_c(&self, state: *mut compu_brotli_sys::BrotliEncoderState) {
        use compu_brotli_sys as sys;

        unsafe {
            let result = sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_QUALITY, self.quality.0 as _);
            debug_assert!(result != 0);
            if !self.mode.is_default() {
                let result = sys::BrotliEncoderSetParameter(state, sys::BrotliEncoderParameter_BROTLI_PARAM_MODE, self.mode as _);
                debug_assert!(result != 0);
            }
        }
    }

    #[cfg(feature = "brotli-rust")]
    #[inline(always)]
    pub(crate) fn apply_rust(&self, state: &mut crate::encoder::brotli::Instance) {
        let result = state.set_parameter(brotli::enc::encode::BrotliEncoderParameter::BROTLI_PARAM_QUALITY, self.quality.0 as _);
        debug_assert!(result);

        if !self.mode.is_default() {
            let result = state.set_parameter(brotli::enc::encode::BrotliEncoderParameter::BROTLI_PARAM_MODE, self.mode as _);
            debug_assert!(result);
        }
    }
}

impl Default for BrotliOptions {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
