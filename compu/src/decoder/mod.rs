//! Decoder module

#[cfg(feature = "brotli-c")]
pub mod brotli;
#[cfg(feature = "brotli-c")]
pub use brotli::BrotliDecoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///Result of decoding
pub enum DecoderResult {
    ///Cannot finish due to lack of input data
    NeedInput,
    ///Need to flush data somewhere before continuing
    NeedOutput,
    ///Error happened while decoding.
    Error,
    ///Successfully finished decoding.
    Finished,
    ///Decoder specific error.
    Other(i32)
}

///Describes decompression interface
pub trait Decoder: Sized {
    ///Decoder options
    type Options: Default;

    ///Creates new instance using provided options.
    fn new(opts: Self::Options) -> Self;

    ///Pushes data to into decompression stream, while writing it out in `output`
    ///
    ///Returns tuple that contains: remaining input to process, remaining output buffer size and
    ///result of decode.
    ///
    ///Once `DecoderResult::Finished` or `DecoderResult::Error` is returned, further processing
    ///should be finished.
    fn decode(&mut self, input: &[u8], output: &mut [u8]) -> (usize, usize, DecoderResult);

    ///Retrieves currently buffered output, that hasn't been written yet.
    ///
    ///Some decoders like `brotli` will assume it as consumed after accessing it.
    fn output<'a>(&'a mut self) -> Option<&'a [u8]>;

    ///Returns whether encoder has finished.
    fn is_finished(&self) -> bool;

    ///Creates new instance using default `Options`
    #[inline(always)]
    fn default() -> Self {
        Self::new(Self::Options::default())
    }
}
