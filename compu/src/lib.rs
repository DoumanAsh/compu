//! Compression/decompression library
//!
//! ## Low-level API
//!
//! Supplies direct wrappers over compression libraries
//!
//! - [Encoder](encoder/trait.Encoder.html) - interface to compression
//! - [Decoder](decoder/trait.Decoder.html) - interface to decompression
//!
//! ## High-level API
//!
//! - [Compressor](compressor/struct.Compressor.html) - Uses `Encoder` to compress data into internal buffer.
//! - [Decompressor](decompressor/struct.Decompressor.html) - Uses `Decoder` to decompress data into external buffer.
//!
//! ## Cargo Features
//!
//! - `brotli-c` - Enables brotli via C library. Default on.
//!

#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]

pub mod encoder;
pub mod decoder;
pub mod compressor;
pub mod decompressor;

pub use compressor::Compressor;
pub use decompressor::Decompressor;
