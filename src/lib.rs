#![doc = include_str!("../README.md")]
#![no_std]
#![warn(missing_docs)]
#![allow(clippy::style)]

pub mod decoder;
#[cfg(any(
    feature = "zlib",
    feature = "zlib-static",
    feature = "zlib-ng",
    feature = "brotli-c",
    feature = "zstd"
))]
pub(crate) mod utils;
pub use decoder::{Decode, DecodeError, DecodeStatus, Decoder, DecoderExt, Detection};
pub mod encoder;
pub use encoder::{Encode, EncodeOp, EncodeStatus, Encoder, EncoderExt};
mod buffer;
pub mod mem;
pub use buffer::Buffer;
