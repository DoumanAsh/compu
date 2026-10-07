//! Encoder

extern crate alloc;

use core::{mem, slice};

use alloc::collections::TryReserveError;
use alloc::vec::Vec;

#[derive(Copy, Clone, PartialEq)]
///Encoder operation
pub enum EncodeOp {
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

#[derive(Debug, Copy, Clone, PartialEq)]
///Encode status
pub enum EncodeStatus {
    ///Encoded, carry on.
    Continue,
    ///Encoded at least partially, but needs more space to write.
    ///
    ///This is generally returned by encoders lacking internal buffer.
    NeedOutput,
    ///Result after `EncoderOp::Finish` issued
    Finished,
    ///Failed to encode.
    Error,
}

#[derive(Debug)]
///Encode output
pub struct Encode {
    ///Number of bytes left unprocessed in `input`
    pub input_remain: usize,
    ///Number of bytes left unprocessed in `output`
    pub output_remain: usize,
    ///Status after `encode`
    pub status: EncodeStatus,
}

///Encoder Interface
///
///## Example
///
///Brief example for chunked encoding.
///
///```rust
///use compu::{Encoder, EncodeStatus, EncodeOp};
///
///fn compress(encoder: &mut impl Encoder, input: &[&[u8]], output: &mut Vec<u8>) {
///   for chunk in input {
///     let spare_capacity = output.spare_capacity_mut();
///     let output_len = spare_capacity.len();
///     let result = encoder.encode_uninit(chunk, spare_capacity, EncodeOp::Flush);
///
///     assert_eq!(result.input_remain, 0);
///     assert_ne!(result.status, EncodeStatus::Error);
///     assert_eq!(result.status, EncodeStatus::Continue);
///     unsafe {
///         output.set_len(output.len() + output_len - result.output_remain);
///     }
///   }
///
///   let spare_capacity = output.spare_capacity_mut();
///   let output_len = spare_capacity.len();
///   let result = encoder.encode_uninit(&[], spare_capacity, EncodeOp::Finish);
///   assert_eq!(result.status, EncodeStatus::Finished);
///
///   unsafe {
///       output.set_len(output.len() + output_len - result.output_remain);
///   }
///   //Make sure to reset state, if you want to re-use encoder.
///   encoder.reset();
///}
///
///let mut output = Vec::with_capacity(100);
///let mut encoder = compu::encoder::BrotliC::new(Default::default()).expect("to create brotli encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///
///output.truncate(0);
///let mut encoder = compu::encoder::ZstdC::new(Default::default()).expect("to create zstd encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///
///output.truncate(0);
///let mut encoder = compu::encoder::ZlibNg::new(Default::default()).expect("to create zlib-ng encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///```
pub trait Encoder {
    ///Encodes `input` into uninit `output` as per `op` operation
    ///
    ///`Encode` will contain number of bytes written into `output`. This number always indicates number of bytes written hence which can be assumed initialized.
    fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode;

    #[inline(always)]
    ///Encodes `input` into `output`.
    fn encode(&mut self, input: &[u8], output: &mut [u8], op: EncodeOp) -> Encode {
        let output = unsafe {
            slice::from_raw_parts_mut(output.as_mut_ptr() as _, output.len())
        };
        self.encode_uninit(input, output, op)
    }

    #[inline(always)]
    ///Encodes `input` into spare space in `output`.
    ///
    ///Function require user to alloc spare capacity himself.
    ///
    ///[Encode::output_remain] will be relative to spare capacity length.
    fn encode_vec(&mut self, input: &[u8], output: &mut Vec<u8>, op: EncodeOp) -> Encode {
        let spare_capacity = output.spare_capacity_mut();
        let spare_capacity_len = spare_capacity.len();
        let result = self.encode_uninit(input, spare_capacity, op);

        let new_len = output.len() + spare_capacity_len - result.output_remain;
        unsafe {
            output.set_len(new_len);
        }
        result
    }

    #[inline(always)]
    ///Encodes `input` into `output` Vec, performing allocation when necessary
    ///
    ///This function will continue encoding as long as input requires more input.
    ///
    ///## Allocation
    ///
    ///Strategy depends on input size.
    ///- Less than 1024:
    ///   - Allocates `input.len()`
    ///   - Re-alloc size `input.len() / 3`
    ///- From 1024 to 65536:
    ///   - Allocates `input.len() / 2`
    ///   - Re-alloc size `1024`
    ///- From 65536:
    ///   - Allocates `input.len() / 3`
    ///   - Re-alloc size `8 * 1024`
    ///
    ///Note that the best strategy is always to re-use buffer
    ///
    ///## Result
    ///
    ///- [Encode::output_remain] will be relatieve to spare capacity of the `output`.
    fn encode_vec_full(&mut self, mut input: &[u8], output: &mut Vec<u8>, op: EncodeOp) -> Result<Encode, TryReserveError> {
        const RESERVE_DEFAULT: usize = 1024;
        let input_len = input.len();
        let reserve_size = if input_len < RESERVE_DEFAULT {
            output.try_reserve_exact(input_len)?;
            input_len / 3
        } else if input_len < (RESERVE_DEFAULT * 16) {
            output.try_reserve_exact(input_len / 2)?;
            RESERVE_DEFAULT
        } else {
            output.try_reserve_exact(input.len() / 3)?;
            RESERVE_DEFAULT * 8
        };

        loop {
            let result = self.encode_vec(input, output, op);
            match result.status {
                EncodeStatus::NeedOutput => {
                    input = &input[input.len() - result.input_remain..];
                    output.try_reserve_exact(reserve_size)?;
                    continue;
                }
                EncodeStatus::Continue if op == EncodeOp::Finish => {
                    input = &input[input.len() - result.input_remain..];
                    continue;
                }
                _ => break Ok(result),
            }
        }
    }

    ///Resets `Encoder` state to initial.
    ///
    ///Returns `true` if successfully reset, otherwise `false`
    fn reset(&mut self) -> bool;
}

impl Encoder for alloc::boxed::Box<dyn Encoder> {
    #[inline(always)]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        (**self).encode_uninit(input, output, op)
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        (**self).reset()
    }
}

impl Encoder for &mut dyn Encoder {
    #[inline(always)]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        (**self).encode_uninit(input, output, op)
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        (**self).reset()
    }
}

impl<T: Encoder> Encoder for alloc::boxed::Box<T> {
    #[inline(always)]
    fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        (**self).encode_uninit(input, output, op)
    }

    #[inline(always)]
    fn reset(&mut self) -> bool {
        (**self).reset()
    }
}

///Extensions to [Encoder]
pub trait EncoderExt: Encoder {
    #[cfg(feature = "bytes")]
    #[inline]
    ///Encodes `input` into `output` buffer, iterating through all spare capacity chunks if
    ///necessary
    ///
    ///Requires `bytes` feature
    ///
    ///[Encode::output_remain] will be relative to spare capacity length.
    fn encode_buf(&mut self, mut input: &[u8], output: &mut impl bytes::BufMut, op: EncodeOp) -> Encode {
        let mut result = Encode {
            input_remain: input.len(),
            output_remain: output.remaining_mut(),
            status: EncodeStatus::NeedOutput,
        };

        loop {
            let spare_capacity = output.chunk_mut();
            let spare_capacity_len = spare_capacity.len();

            let (advanced_len, encode) = unsafe {
                let encode = self.encode_uninit(input, spare_capacity.as_uninit_slice_mut(), op);
                debug_assert!(spare_capacity_len > encode.output_remain);
                let advanced_len = spare_capacity_len.saturating_sub(encode.output_remain);
                output.advance_mut(advanced_len);
                (advanced_len, encode)
            };
            input = &input[result.input_remain - encode.input_remain..];
            result.input_remain = encode.input_remain;
            result.output_remain = result.output_remain.saturating_sub(advanced_len);
            result.status = encode.status;

            match result.status {
                EncodeStatus::Error | EncodeStatus::Finished | EncodeStatus::Continue => break result,
                EncodeStatus::NeedOutput => {
                    if result.output_remain == 0 {
                        break result;
                    }
                }
            }
        }
    }
}

impl<T: Encoder> EncoderExt for T {
}

#[cfg(any(feature = "brotli", feature = "brotli-c"))]
mod brotli_common;
#[cfg(any(feature = "brotli", feature = "brotli-c"))]
pub use brotli_common::{BrotliEncoderMode, BrotliOptions};
#[cfg(feature = "brotli")]
mod brotli;
#[cfg(feature = "brotli")]
pub use brotli::BrotliRust;
#[cfg(feature = "brotli-c")]
mod brotli_c;
#[cfg(feature = "brotli-c")]
pub use brotli_c::BrotliC;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng", feature = "zlib-rs"))]
mod zlib_common;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng", feature = "zlib-rs"))]
pub use zlib_common::*;
#[cfg(any(feature = "zlib", feature = "zlib-static"))]
mod zlib;
#[cfg(any(feature = "zlib", feature = "zlib-static"))]
pub use zlib::ZlibC;
#[cfg(feature = "zlib-ng")]
mod zlib_ng;
#[cfg(feature = "zlib-ng")]
pub use zlib_ng::ZlibNg;
#[cfg(feature = "zlib-rust")]
mod zlib_rust;
#[cfg(feature = "zlib-rust")]
pub use zlib_rust::ZlibRust;
#[cfg(feature = "zstd")]
mod zstd;
#[cfg(feature = "zstd")]
pub use zstd::{ZstdOptions, ZstdStrategy, ZstdC};

impl<const N: usize> crate::Buffer<N> {
    ///Decodes `input` using `decoder` returning number of bytes consumed in `input`
    ///
    ///Returns tuple with:
    ///- Number of consumed bytes in `input`
    ///- Decode status:
    ///    - In case of `Finished` or `Error`, you should not continue to invoke decode until you reset decoder
    ///    - In case of `NeedOutput`, you should consume internal buffer.
    pub fn encode(&mut self, encoder: &mut impl Encoder, input: &[u8], op: EncodeOp) -> (usize, EncodeStatus) {
        let spare_capacity = self.spare_capacity_mut();
        let spare_capacity_len = spare_capacity.len();

        let result = encoder.encode_uninit(input, spare_capacity, op);

        self.cursor = self.cursor + spare_capacity_len - result.output_remain;
        (input.len() - result.input_remain, result.status)
    }
}
