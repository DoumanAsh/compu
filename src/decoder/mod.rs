//! Decoder
use core::{mem, ptr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
///Decoding error
pub struct DecodeError(i32);

impl DecodeError {
    ///Creates error which means no error.
    ///
    ///Specifically its code is 0
    pub const fn no_error() -> Self {
        Self(0)
    }

    #[inline(always)]
    ///Returns raw integer
    pub const fn as_raw(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///Result of decoding
pub enum DecodeStatus {
    ///Cannot finish due to lack of input data
    NeedInput,
    ///Need to flush data somewhere before continuing
    NeedOutput,
    ///Successfully finished decoding.
    Finished,
}

///Decode output
pub struct Decode {
    ///Number of bytes left unprocessed in `input`
    pub input_remain: usize,
    ///Number of bytes left unprocessed in `output`
    pub output_remain: usize,
    ///Result of decoding
    pub status: Result<DecodeStatus, DecodeError>,
}

///Decoder interface
pub struct Interface {
    decode_fn: unsafe fn(ptr::NonNull<u8>, *const u8, usize, *mut u8, usize) -> Decode,
    //returns new/updated instance, MUST be replaced
    reset_fn: fn (ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>>,
    drop_fn: fn (ptr::NonNull<u8>),
    describe_error_fn: fn(i32) -> Option<&'static str>,
}

///Decoder
///
///Use [Interface] to instantiate decoder.
///
///Under hood, in order to avoid generics, implemented as vtable with series of function pointers.
///
///
///## Example
///
///Brief example for chunked decoding.
///```rust
///use compu::{Decoder, DecodeStatus, Encoder, EncodeOp, EncodeStatus};
///
///fn decompress(decoder: &mut Decoder, input: core::slice::Chunks<'_, u8>, output: &mut Vec<u8>) {
///   for chunk in input {
///     let spare_capacity = output.spare_capacity_mut();
///     let output_len = spare_capacity.len();
///     let result = decoder.decode_uninit(chunk, spare_capacity);
///
///     assert_eq!(result.input_remain, 0);
///     let status = result.status.expect("success");
///     unsafe {
///         output.set_len(output.len() + output_len - result.output_remain);
///     }
///     if status == DecodeStatus::Finished {
///         break;
///     }
///   }
///
///   //Make sure to reset state, if you want to re-use decoder.
///   decoder.reset();
///}
///
///fn prepare_compressed(encoder: &mut Encoder, data: &[u8], compressed: &mut Vec<u8>) {
///    let spare_capacity = compressed.spare_capacity_mut();
///    let spare_capacity_len = spare_capacity.len();
///    let result = encoder.encode_uninit(DATA, spare_capacity, EncodeOp::Finish);
///    assert_eq!(result.status, EncodeStatus::Finished);
///    unsafe {
///        compressed.set_len(spare_capacity_len - result.output_remain);
///    }
///}
///
///const DATA: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
///
///let mut output = Vec::with_capacity(100);
///
///let mut compressed = Vec::with_capacity(100);
///let mut encoder = compu::encoder::Interface::brotli_c(Default::default()).expect("to create brotli encoder");
///prepare_compressed(&mut encoder, DATA, &mut compressed);
///let mut decoder = compu::decoder::Interface::brotli_c().expect("to create brotli decoder");
///decompress(&mut decoder, compressed.chunks(4), &mut output);
///assert_eq!(output, DATA);
///
///output.truncate(0);
///compressed.truncate(0);
///
///let mut compressed = Vec::with_capacity(100);
///let mut encoder = compu::encoder::Interface::zstd(Default::default()).expect("to create zstd encoder");
///prepare_compressed(&mut encoder, DATA, &mut compressed);
///let mut decoder = compu::decoder::Interface::zstd(Default::default()).expect("to create zstd decoder");
///decompress(&mut decoder, compressed.chunks(4), &mut output);
///assert_eq!(output, DATA);
///
///output.truncate(0);
///compressed.truncate(0);
///
///let mut compressed = Vec::with_capacity(100);
///let mut encoder = compu::encoder::Interface::zlib_ng(Default::default()).expect("to create zlib-ng encoder");
///prepare_compressed(&mut encoder, DATA, &mut compressed);
///let mut decoder = compu::decoder::Interface::zlib_ng(Default::default()).expect("to create zlib-ng decoder");
///decompress(&mut decoder, compressed.chunks(4), &mut output);
///assert_eq!(output, DATA);
///
///output.truncate(0);
///compressed.truncate(0);
///```
pub struct Decoder {
    instance: ptr::NonNull<u8>,
    interface: &'static Interface
}

impl Decoder {
    #[inline(always)]
    ///Raw decoding function, with no checks.
    ///
    ///Intended to be used as building block of higher level interfaces
    ///
    ///Arguments
    ///
    ///- `input` - Pointer to start of input to process. MUST NOT be null.
    ///- `input_len` - Size of data to process in `input`
    ///- `ouput` - Pointer to start of buffer where to write result. MUST NOT be null
    ///- `output_len` - Size of buffer pointed by `output`
    pub unsafe fn raw_decode(&mut self, input: *const u8, input_len: usize, output: *mut u8, output_len: usize) -> Decode {
        (self.interface.decode_fn)(self.instance, input, input_len, output, output_len)
    }

    #[inline(always)]
    ///Decodes `input` into uninit `output`.
    ///
    ///`Decode` will contain number of bytes written into `output`. This number always indicates
    ///number of bytes written hence which can be assumed initialized.
    pub fn decode_uninit(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<u8>]) -> Decode {
        let input_len = input.len();
        let output_len = output.len();
        unsafe {
            self.raw_decode(input.as_ptr(), input_len, output.as_mut_ptr() as _, output_len)
        }
    }

    #[inline(always)]
    ///Decodes `input` into `output`.
    pub fn decode(&mut self, input: &[u8], output: &mut [u8]) -> Decode {
        let input_len = input.len();
        let output_len = output.len();
        unsafe {
            self.raw_decode(input.as_ptr(), input_len, output.as_mut_ptr() as _, output_len)
        }
    }

    #[inline(always)]
    ///Resets `Decoder` state to initial.
    ///
    ///Returns `true` if successfully reset, otherwise `false`
    pub fn reset(&mut self) -> bool {
        match (self.interface.reset_fn)(self.instance) {
            Some(ptr) => {
                self.instance = ptr;
                true
            },
            None => false,
        }
    }

    #[inline(always)]
    ///Returns descriptive text for error.
    pub fn describe_error(&self, error: DecodeError) -> Option<&'static str> {
        (self.interface.describe_error_fn)(error.as_raw())
    }
}

impl Drop for Decoder {
    #[inline]
    fn drop(&mut self) {
        (self.interface.drop_fn)(self.instance);
    }
}

//ZLIB macro has to be defined before declaring modules
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
macro_rules! internal_zlib_impl_decode {
    ($state:ident, $input:ident, $input_len:ident, $output:ident, $output_len:ident) => {{
        use $crate::decoder::DecodeStatus;

        let state = unsafe {
            &mut *($state.as_ptr() as *mut State)
        };
        state.inner.avail_out = $output_len as _;
        state.inner.next_out = $output;

        state.inner.avail_in = $input_len as _;
        state.inner.next_in = $input as *mut _;

        let result = unsafe {
            sys::inflate(&mut state.inner, 0)
        };

        $crate::decoder::Decode {
            input_remain: state.inner.avail_in as usize,
            output_remain: state.inner.avail_out as usize,
            status: match result {
                sys::Z_OK => match state.inner.avail_in {
                    0 => Ok(DecodeStatus::NeedInput),
                    _ => Ok(DecodeStatus::NeedOutput),
                },
                sys::Z_STREAM_END => Ok(DecodeStatus::Finished),
                sys::Z_BUF_ERROR => Ok(DecodeStatus::NeedOutput),
                other => Err(crate::decoder::DecodeError(other))
            }
        }

    }}
}

#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
mod zlib_common;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
pub use zlib_common::ZlibMode;
#[cfg(any(feature = "zlib", feature = "zlib-static"))]
mod zlib;
#[cfg(feature = "zlib-ng")]
mod zlib_ng;
#[cfg(feature = "brotli-c")]
mod brotli_c;
#[cfg(feature = "zstd")]
mod zstd;
#[cfg(feature = "zstd")]
pub use zstd::ZstdOptions;
