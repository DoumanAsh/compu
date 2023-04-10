//!Rust Compression library with generic interface
//!
//!## Features
//!
//!All features are off by default.
//!This crate requires `alloc` to be available with system allocator set.
//!
//!- `brotli-c` - Enables `brotli` interface using C library.
//!- `zlib-ng` - Enables `zlib-ng` interface.
//!- `zlib` - Enables `zlib` interface.
//!- `zlib-static` - Enables `zlib` interface with `static` feature.
//!- `zstd` - Enables `zstd` interface.

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::derivable_impls))]

pub(crate) mod utils;
pub mod decoder;
pub use decoder::Decoder;
pub mod encoder;
pub mod mem;