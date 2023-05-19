//!Rust Compression library with generic interface
//!
//!## API
//!
//!API is mostly thin layer that provides uniform behavior for all algorithms the best it can.
//!
//!Please read documentation to see how to use:
//!
//!- [Decoder](decoder/struct.Decoder.html)
//!- [Encoder](encoder/struct.Encoder.html)
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
//!
//!## Usage
//!
//!### Decode
//!
//!Minimal example of using Decoder.
//!
//!```rust,no_run
//!use compu::{Decoder, DecodeStatus, DecodeError};
//!
//!fn example(decoder: &mut Decoder, input: &[u8]) -> Result<Vec<u8>, DecodeError> {
//!     let mut output = Vec::with_capacity(1024);
//!     loop {
//!         let result = decoder.decode_vec(input, &mut output).status?;
//!         match result {
//!             DecodeStatus::NeedInput => panic!("Not enough input, incomplete data?"),
//!             //If you need more output, please allocate spare capacity.
//!             //API never allocates, only you allocate
//!             DecodeStatus::NeedOutput => output.reserve(1024),
//!             DecodeStatus::Finished => {
//!                 //Make sure to reset state, if you want to re-use decoder.
//!                 decoder.reset();
//!                 break Ok(output)
//!             }
//!         }
//!     }
//!}
//!```
//!
//!### Encode
//!
//!```rust,no_run
//!use compu::{Encoder, EncodeStatus, EncodeOp};
//!
//!fn example(encoder: &mut Encoder, input: &[u8]) -> Vec<u8> {
//!     let mut output = Vec::with_capacity(1024);
//!     loop {
//!         let result = encoder.encode_vec(input, &mut output, EncodeOp::Finish).status;
//!         match result {
//!             //This status is returned by any other `EncodeOp` except `Finish
//!             EncodeStatus::Continue => panic!("I wanted to finish but continue!?"),
//!             //If you need more output, please allocate spare capacity.
//!             //API never allocates, only you allocate
//!             EncodeStatus::NeedOutput => output.reserve(1024),
//!             //If you have enough space, `EncodeOp::Finish` will result in this operation
//!             EncodeStatus::Finished => {
//!                 //Make sure to reset state, if you want to re-use it.
//!                 encoder.reset();
//!                 break output;
//!             }
//!             //Generally can indicate internal error likely due to OOM condition.
//!             //Note that all wrappers ensure that Rust's global allocator is used,
//!             //so take care if you use custom one
//!             //Generally should not happen, so it is ok to just panic
//!             //but be a good boy and return error properly if it happens, even if it is unlikely
//!             EncodeStatus::Error => {
//!                 panic!("unlikely")
//!             }
//!         }
//!     }
//!}
//!```

#![no_std]
#![warn(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::style))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::derivable_impls))]

#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng", feature = "brotli-c", feature = "zstd"))]
pub(crate) mod utils;
pub mod decoder;
pub use decoder::{Decoder, Decode, DecodeStatus, DecodeError, Detection};
pub mod encoder;
pub use encoder::{Encoder, Encode, EncodeOp, EncodeStatus};
pub mod mem;
