# コンプ(compu)

[![Rust](https://github.com/DoumanAsh/compu/actions/workflows/rust.yml/badge.svg)](https://github.com/DoumanAsh/compu/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/compu.svg)](https://crates.io/crates/compu)
[![Documentation](https://docs.rs/compu/badge.svg)](https://docs.rs/crate/compu/)
[![dependency status](https://deps.rs/crate/compu/2.0.0/status.svg)](https://deps.rs/crate/compu/2.0.0)

Rust Compression library with generic interface

## Features

All features are off by default.

This crate requires `alloc` to be available with system allocator set.

### Codecs

 Feature | Description | Encoder | Decoder |
---------|------------ | ------- | ------- |
 `brotli-c` | Enables `brotli` interface using C library | decoder::BrotliC | encoder::BrotliC
 `brotli-rust` | Enables `brotli` interface using Rust library | decoder::BrotliRust | encoder::BrotliRust
 `zlib` | Enables `zlib` interface using libz library | decoder::ZlibC | encoder::ZlibC
 `zlib-static` | Enables `zlib` interface using libz library with 'static' feature | decoder::ZlibC | encoder::ZlibC
 `zlib-ng` | Enables `zlib` interface using libz-ng library | decoder::ZlibNg | encoder::ZlibNg
 `zlib-rust` | Enables `zlib` interface using zlib-rs library | decoder::ZlibRust | encoder::ZlibRust
 `zstd` | Enables `zstd` interface using zstd C library | decoder::ZstdC | encoder::ZstdC

### Misc

- `bytes` - Enables `bytes` support via [decoder::DecoderExt] and [encoder::EncoderExt]

## Usage

### Decode

Minimal example of using Decoder.

If you unsure about compression used, you can try [detect](https://docs.rs/compu/latest/compu/decoder/enum.Detection.html#method.detect) it

```rust,no_run
use compu::{Decoder, DecodeStatus, DecodeError};

fn example(decoder: &mut impl Decoder, input: &[u8]) -> Result<Vec<u8>, DecodeError> {
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

```rust,no_run
use compu::{Encoder, EncodeStatus, EncodeOp};

fn example(encoder: &mut impl Encoder, input: &[u8]) -> Vec<u8> {
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
