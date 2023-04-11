//! Encoder

use core::{mem, ptr};

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

///Encoder interface
pub struct Interface {
    //returns new/updated instance, MUST be replaced
    reset_fn: fn (ptr::NonNull<u8>, opts: [u8; 2]) -> Option<ptr::NonNull<u8>>,
    encode_fn: unsafe fn (ptr::NonNull<u8>, *const u8, usize, *mut u8, usize, EncodeOp) -> Encode,
    drop_fn: fn (ptr::NonNull<u8>),
}

///Decoder
///
///Use [Interface] to instantiate decoder
///
///Under hood, in order to avoid generics, implemented as vtable with series of function pointers.
///
///## Example
///
///Brief example for chunked encoding.
///
///```rust
///use compu::{Encoder, EncodeStatus, EncodeOp};
///
///fn compress(encoder: &mut Encoder, input: &[&[u8]], output: &mut Vec<u8>) {
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
///let mut encoder = compu::encoder::Interface::brotli_c(Default::default()).expect("to create brotli encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///
///output.truncate(0);
///let mut encoder = compu::encoder::Interface::zstd(Default::default()).expect("to create zstd encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///
///output.truncate(0);
///let mut encoder = compu::encoder::Interface::zlib_ng(Default::default()).expect("to create zlib-ng encoder");
///compress(&mut encoder, &[&[1, 2, 3, 4], &[5, 6, 7 ,8], &[9, 10]], &mut output);
///assert!(output.len() > 0);
///```
pub struct Encoder {
    instance: ptr::NonNull<u8>,
    interface: &'static Interface,
    opts: [u8; 2]
}

impl Encoder {
    #[inline(always)]
    ///Raw encoding function, with no checks.
    ///
    ///Intended to be used as building block of higher level interfaces
    ///
    ///Arguments
    ///
    ///- `input` - Pointer to start of input to process. MUST NOT be null.
    ///- `input_len` - Size of data to process in `input`
    ///- `ouput` - Pointer to start of buffer where to write result. MUST NOT be null
    ///- `output_len` - Size of buffer pointed by `output`
    ///- `op` - Encoding operation to perform.
    pub unsafe fn raw_encode(&mut self, input: *const u8, input_len: usize, output: *mut u8, output_len: usize, op: EncodeOp) -> Encode {
        (self.interface.encode_fn)(self.instance, input, input_len, output, output_len, op)
    }

    #[inline(always)]
    ///Encodes `input` into uninit `output`.
    ///
    ///`Encode` will contain number of bytes written into `output`. This number always indicates number of bytes written hence which can be assumed initialized.
    pub fn encode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>], op: EncodeOp) -> Encode {
        let input_len = input.len();
        let output_len = output.len();
        unsafe {
            self.raw_encode(input.as_ptr(), input_len, output.as_mut_ptr() as _, output_len, op)
        }
    }

    #[inline(always)]
    ///Encodes `input` into `output`.
    pub fn encode(&mut self, input: &[u8], output: &mut [u8], op: EncodeOp) -> Encode {
        let input_len = input.len();
        let output_len = output.len();
        unsafe {
            self.raw_encode(input.as_ptr(), input_len, output.as_mut_ptr() as _, output_len, op)
        }
    }

    #[inline(always)]
    ///Resets `Encoder` state to initial.
    ///
    ///Returns `true` if successfully reset, otherwise `false`
    pub fn reset(&mut self) -> bool {
        match (self.interface.reset_fn)(self.instance, self.opts) {
            Some(ptr) => {
                self.instance = ptr;
                true
            },
            None => false,
        }
    }
}

impl Drop for Encoder {
    #[inline]
    fn drop(&mut self) {
        (self.interface.drop_fn)(self.instance);
    }
}

//ZLIB macro has to be defined before declaring modules
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
macro_rules! internal_zlib_impl_encode {
    ($state:ident, $input:ident, $input_remain:ident, $output:ident, $output_remain:ident, $op:ident) => {{
        let op = match $op {
            $crate::encoder::EncodeOp::Process => sys::Z_NO_FLUSH,
            $crate::encoder::EncodeOp::Flush => sys::Z_SYNC_FLUSH,
            $crate::encoder::EncodeOp::Finish => sys::Z_FINISH
        };

        let state = unsafe {
            &mut *($state.as_ptr() as *mut State)
        };

        state.inner.avail_out = $output_remain as _;
        state.inner.next_out = $output;

        state.inner.avail_in = $input_remain as _;
        state.inner.next_in = $input as *mut _;

        let result = unsafe {
            sys::deflate(&mut state.inner, op)
        };

        $crate::encoder::Encode {
            input_remain: state.inner.avail_in as usize,
            output_remain: state.inner.avail_out as usize,
            status: match result {
                sys::Z_STREAM_END => $crate::encoder::EncodeStatus::Finished,
                //If it is final chunk, zlib may report OK while it needs more output (specifically in case of GZIP)
                sys::Z_OK => {
                    if op == sys::Z_FINISH {
                        $crate::encoder::EncodeStatus::NeedOutput
                    } else {
                        $crate::encoder::EncodeStatus::Continue
                    }
                },
                sys::Z_BUF_ERROR => $crate::encoder::EncodeStatus::NeedOutput,
                _ => $crate::encoder::EncodeStatus::Error,
            }
        }
    }}
}


#[cfg(feature = "brotli-c")]
mod brotli_c;
#[cfg(feature = "brotli-c")]
pub use brotli_c::{BrotliOptions, BrotliEncoderMode};
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
mod zlib_common;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
pub use zlib_common::*;
#[cfg(any(feature = "zlib", feature = "zlib-static"))]
mod zlib;
#[cfg(feature = "zlib-ng")]
mod zlib_ng;
#[cfg(feature = "zstd")]
mod zstd;
#[cfg(feature = "zstd")]
pub use zstd::{ZstdOptions, ZstdStrategy};
