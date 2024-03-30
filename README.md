# コンプ(compu)

[![Actions Status](https://github.com/DoumanAsh/compu/workflows/Rust/badge.svg)](https://github.com/DoumanAsh/compu/actions)
[![Crates.io](https://img.shields.io/crates/v/compu.svg)](https://crates.io/crates/compu)
[![Documentation](https://docs.rs/compu/badge.svg)](https://docs.rs/crate/compu/)

Rust Compression library with generic interface

## Features

All features are off by default.
This crate requires `alloc` to be available with system allocator set.

- `brotli-c` - Enables `brotli` interface using C library.
- `brotli-rust` - Enables `brotli` interface using pure Rust library.
- `zlib-ng` - Enables `zlib-ng` interface.
- `zlib` - Enables `zlib` interface.
- `zlib-static` - Enables `zlib` interface with `static` feature.
- `zstd` - Enables `zstd` interface.

## Usage

### Decode

Minimal example of using Decoder.
Use `Interface` to create instance.

```rust,no_run
use compu::{Decoder, DecodeStatus, DecodeError};

fn example(decoder: &mut Decoder, input: &[u8]) -> Result<Vec<u8>, DecodeError> {
     let mut output = Vec::with_capacity(1024);
     loop {
         let result = decoder.decode_vec(input, &mut output).status?;
         match result {
             DecodeStatus::NeedInput => panic!("Not enough input, incomplete data?"),
             //If you need more output, please allocate spare capacity.
             //API never allocates, only you allocate
             DecodeStatus::NeedOutput => output.reserve(1024),
             DecodeStatus::Finished => {
                 //Make sure to reset state, if you want to re-use decoder.
                 decoder.reset();
                 break Ok(output)
             }
         }
     }
}
```

### Encode

Minimal example of using Encoder.
Use `Interface` to create instance.

```rust
use compu::{Encoder, EncodeStatus, EncodeOp};

fn example(encoder: &mut Encoder, input: &[u8]) -> Vec<u8> {
     let mut output = Vec::with_capacity(1024);
     loop {
         let result = encoder.encode_vec(input, &mut output, EncodeOp::Finish).status;
         match result {
             //This status is returned by any other `EncodeOp` except `Finish
             EncodeStatus::Continue => panic!("I wanted to finish but continue!?"),
             //If you need more output, please allocate spare capacity.
             //API never allocates, only you allocate
             EncodeStatus::NeedOutput => output.reserve(1024),
             //If you have enough space, `EncodeOp::Finish` will result in this operation
             EncodeStatus::Finished => {
                 //Make sure to reset state, if you want to re-use it.
                 encoder.reset();
                 break output;
             }
             //Generally can indicate internal error likely due to OOM condition.
             //Note that all wrappers ensure that Rust's global allocator is used,
             //so take care if you use custom one
             //Generally should not happen, so it is ok to just panic
             //but be a good boy and return error properly if it happens, even if it is unlikely
             EncodeStatus::Error => {
                 panic!("unlikely")
             }
         }
     }
}
```
