# コンプ(compu)

[![Actions Status](https://github.com/DoumanAsh/compu/workflows/Rust/badge.svg)](https://github.com/DoumanAsh/compu/actions)
[![Crates.io](https://img.shields.io/crates/v/compu.svg)](https://crates.io/crates/compu)
[![Documentation](https://docs.rs/compu/badge.svg)](https://docs.rs/crate/compu/)

Rust Compression library with generic interface

## Features

All features are off by default.
This crate requires `alloc` to be available with system allocator set.

- `brotli-c` - Enables `brotli` interface using C library.
- `zlib-ng` - Enables `zlib-ng` interface.
- `zlib` - Enables `zlib` interface.
- `zlib-static` - Enables `zlib` interface with `static` feature.
- `zstd` - Enables `zstd` interface.
