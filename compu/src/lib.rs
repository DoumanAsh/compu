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
//! - In-memory
//!     - [Compressor](compressor/memory/struct.Compressor.html) - Uses `Encoder` to compress data into internal buffer.
//!     - [Decompressor](decompressor/memory/struct.Decompressor.html) - Uses `Decoder` to decompress data into external buffer.
//! - Blocking Write interface
//!     - [Compressor](compressor/write/struct.Compressor.html) - Uses `Encoder` to compress data into supplied writer.
//!     - [Decompressor](decompressor/write/struct.Decompressor.html) - Uses `Decoder` to decompress data into supplied writer.
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
