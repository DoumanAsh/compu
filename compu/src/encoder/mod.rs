//!Encoder module

#[cfg(feature = "brotli-c")]
pub mod brotli;
#[cfg(feature = "brotli-c")]
pub use brotli::BrotliEncoder;
#[cfg(feature = "zlib")]
pub mod zlib;
#[cfg(feature = "zlib")]
pub use zlib::ZlibEncoder;

#[derive(Copy, Clone, PartialEq)]
///Encoder operation
pub enum EncoderOp {
    ///Just compress as usual.
    Process,
    ///Flush as much data as possible
    ///
    ///Potentially may incur overhead
    Flush,
    ///Finish compression.
    ///
    ///After issuing FINISH, no new data should be added.
    Finish,
}

///Describes compression interface
pub trait Encoder: Sized {
    ///Encoder options
    type Options: Default;

    ///Creates new instance using provided options.
    fn new(opts: &Self::Options) -> Self;

    ///Performs encoding of data chunk.
    ///
    ///Returns tuple that contains: remaining input to process, remaining output buffer size and
    ///whether encode is successful.
    ///
    ///Use `op` equal to `EncoderOp::Finish` to specify last chunk
    fn encode(&mut self, input: &[u8], output: &mut [u8], op: EncoderOp) -> (usize, usize,  bool);

    ///Retrieves currently buffered output, that hasn't been written yet.
    ///
    ///Returned bytes MUST be marked as consumed by implementation.
    fn output<'a>(&'a mut self) -> Option<&'a [u8]>;

    ///Returns estimated number of bytes, for compressed input.
    ///
    ///Note that it might not be reliable, depending on encoder.
    fn compress_size_hint(&self, size: usize) -> usize;

    ///Returns whether encoder has finished.
    fn is_finished(&self) -> bool;

    ///Creates new instance using default `Options`
    #[inline(always)]
    fn default() -> Self {
        Self::new(&Self::Options::default())
    }
}
