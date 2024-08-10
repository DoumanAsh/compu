//! Decoder
extern crate alloc;

use core::{mem, ptr};

use alloc::collections::TryReserveError;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
///Possible compression archive based on known signatures
pub enum Detection {
    ///ZSTD
    Zstd,
    ///GZIP
    Gzip,
    ///ZLIB
    Zlib,
    ///Indicates that all possible options are exhausted and it is impossible to deduce
    ///compression.
    Unknown,
}

impl Detection {
    ///Attempts to deduce compression format from available bytes.
    ///
    ///Returns `None` if there is not enough `bytes` to perform all possible checks.
    ///In this case you need to append more data to your buffer and provide it again
    pub const fn detect(bytes: &[u8]) -> Option<Detection> {
        //https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#zstandard-frames
        const ZSTD_HEADER: u32 = 0xFD2FB528u32.to_le();
        const GZIP_HEADER: u16 = 0x1f8bu16.to_be();

        macro_rules! detect_gzip {
            ($word:ident) => {
                if $word == GZIP_HEADER {
                    return Some(Detection::Gzip);
                }
            };
        }

        //Signature:
        //
        // # Zlib
        //
        //      FLEVEL: 0       1       2       3
        //CINFO:
        //     0      08 1D   08 5B   08 99   08 D7
        //     1      18 19   18 57   18 95   18 D3
        //     2      28 15   28 53   28 91   28 CF
        //     3      38 11   38 4F   38 8D   38 CB
        //     4      48 0D   48 4B   48 89   48 C7
        //     5      58 09   58 47   58 85   58 C3
        //     6      68 05   68 43   68 81   68 DE
        //     7      78 01   78 5E   78 9C   78 DA
        macro_rules! detect_zlib {
            ($word:ident) => {
                if $word.to_be() % 31 == 0 {
                    match bytes[0] {
                        0x78 => if let 0x01 | 0x5e | 0x9c | 0xda = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x08 => if let 0x1d | 0x5b | 0x99 | 0xd7 = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x18 => if let 0x19 | 0x57 | 0x95 | 0xd3 = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x28 => if let 0x15 | 0x53 | 0x91 | 0xcf = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x38 => if let 0x11 | 0x4f | 0x8d | 0xcb = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x48 => if let 0x0d | 0x4b | 0x89 | 0xc7 = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x58 => if let 0x09 | 0x47 | 0x85 | 0xc3 = bytes[1] {
                            return Some(Detection::Zlib);
                        }
                        0x68 => if let 0x05 | 0x43 | 0x81 | 0xde = bytes[1] {
                            Some(Detection::Zlib);
                        }
                        _ => (),
                    }
                }
            };
        }

        macro_rules! detect_zstd {
            ($dword:ident) => {
                if $dword == ZSTD_HEADER {
                    return Some(Detection::Zstd);
                }
            };
        }

        if bytes.len() < mem::size_of::<u16>() {
            None
        } else if bytes.len() < mem::size_of::<u32>() {
            let word = u16::from_ne_bytes([bytes[0], bytes[1]]);
            detect_gzip!(word);
            detect_zlib!(word);

            None
        } else {
            let word = u16::from_ne_bytes([bytes[0], bytes[1]]);
            detect_gzip!(word);
            detect_zlib!(word);
            let dword = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            detect_zstd!(dword);

            Some(Detection::Unknown)
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    reset_fn: fn(ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>>,
    drop_fn: fn(ptr::NonNull<u8>),
    describe_error_fn: fn(i32) -> Option<&'static str>,
}

impl Interface {
    ///Creates new `Interface` with provided functions to build vtable.
    ///
    ///First argument of every function is state as pointer.
    ///
    ///It is user responsibility to pass correct function pointers
    pub const fn new(decode_fn: unsafe fn(ptr::NonNull<u8>, *const u8, usize, *mut u8, usize) -> Decode, reset_fn: fn(ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>>, drop_fn: fn(ptr::NonNull<u8>), describe_error_fn: fn(i32) -> Option<&'static str>) -> Self {
        Self {
            decode_fn,
            reset_fn,
            drop_fn,
            describe_error_fn,
        }
    }

    #[inline(always)]
    pub(crate) fn inner_decoder(&'static self, instance: ptr::NonNull<u8>) -> Decoder {
        Decoder {
            instance,
            interface: self,
        }
    }

    #[inline(always)]
    ///Creates new decoder
    ///
    ///This function is unsafe as it is up to user to ensure correctness of `Interface
    ///
    ///`instance` - Decoder state, passed as first argument to every function in vtable
    pub unsafe fn decoder(&'static self, state: ptr::NonNull<u8>) -> Decoder {
        self.inner_decoder(state)
    }
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
///     let result = decoder.decode_vec(chunk, output);
///
///     assert_eq!(result.input_remain, 0);
///     let status = result.status.expect("success");
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
///    let result = encoder.encode_vec(DATA, compressed, EncodeOp::Finish);
///    assert_eq!(result.status, EncodeStatus::Finished);
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
    interface: &'static Interface,
}

const _: () = {
    assert!(mem::size_of::<Decoder>() == mem::size_of::<usize>() * 2);
};

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
    ///Decodes `input` into spare space in `output`.
    ///
    ///Function require user to alloc spare capacity himself.
    ///
    ///`Decode::output_remain` will be relatieve to spare capacity length.
    pub fn decode_vec(&mut self, input: &[u8], output: &mut Vec<u8>) -> Decode {
        let spare_capacity = output.spare_capacity_mut();
        let spare_capacity_len = spare_capacity.len();
        let result = self.decode_uninit(input, spare_capacity);

        if result.status.is_ok() {
            let new_len = output.len() + spare_capacity_len - result.output_remain;
            unsafe {
                output.set_len(new_len);
            }
        }
        result
    }

    #[inline(always)]
    ///Decodes `input` into `output` Vec, performing allocation when necessary
    ///
    ///This function will continue decoding as long as input requires more input.
    ///
    ///## Allocation
    ///
    ///Strategy depends on input size.
    ///- Less than 1024:
    ///   - Allocates `input.len()`
    ///   - Re-alloc size `input.len() / 3`
    ///- From 1024 to 65536:
    ///   - Allocates `input.len() + input.len() / 3`
    ///   - Re-alloc size `1024`
    ///- From 65536:
    ///   - Allocates `input.len() * 2`
    ///   - Re-alloc size `8 * 1024`
    ///
    ///Note that the best strategy is always to re-use buffer
    ///
    ///## Result
    ///
    ///- `Decode::output_remain` will be relatieve to spare capacity of the `output`.
    pub fn decode_vec_full(&mut self, mut input: &[u8], output: &mut Vec<u8>) -> Result<Decode, TryReserveError> {
        const RESERVE_DEFAULT: usize = 1024;
        let input_len = input.len();
        let reserve_size = if input_len < RESERVE_DEFAULT {
            output.try_reserve_exact(input_len)?;
            input_len / 3
        } else if input_len < (RESERVE_DEFAULT * 16) {
            output.try_reserve_exact(input_len + input_len / 3)?;
            RESERVE_DEFAULT
        } else {
            output.try_reserve_exact(input.len() * 2)?;
            RESERVE_DEFAULT * 8
        };

        loop {
            let result = self.decode_vec(input, output);
            match result.status {
                Ok(DecodeStatus::NeedOutput) => {
                    input = &input[input.len() - result.input_remain..];
                    output.try_reserve_exact(reserve_size)?;
                    continue;
                }
                _ => break Ok(result),
            }
        }
    }

    #[cfg(feature = "bytes")]
    ///Decodes `input` into `output` buffer, iterating through all spare capacity chunks if
    ///necessary
    ///
    ///Requires `bytes` feature
    ///
    ///`Decode::output_remain` will be relative to spare capacity length.
    pub fn decode_buf(&mut self, mut input: &[u8], output: &mut impl bytes::BufMut) -> Decode {
        let mut result = Decode {
            input_remain: input.len(),
            output_remain: output.remaining_mut(),
            status: Ok(DecodeStatus::NeedOutput),
        };

        loop {
            let spare_capacity = output.chunk_mut();
            let spare_capacity_len = spare_capacity.len();

            let (advanced_len, decode) = unsafe {
                let decode = self.decode_uninit(input, spare_capacity.as_uninit_slice_mut());
                debug_assert!(spare_capacity_len > decode.output_remain);
                let advanced_len = spare_capacity_len.saturating_sub(decode.output_remain);
                output.advance_mut(advanced_len);
                (advanced_len, decode)
            };
            input = &input[result.input_remain - decode.input_remain..];
            result.input_remain = decode.input_remain;
            result.output_remain = result.output_remain.saturating_sub(advanced_len);
            result.status = decode.status;

            match result.status {
                Ok(DecodeStatus::Finished | DecodeStatus::NeedInput) => break result,
                Ok(DecodeStatus::NeedOutput) => {
                    if result.output_remain == 0 {
                        break result;
                    }
                }
                Err(_) => break result,
            }
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
            }
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

        let state = unsafe { &mut *($state.as_ptr() as *mut State) };
        state.inner.avail_out = $output_len as _;
        state.inner.next_out = $output;

        state.inner.avail_in = $input_len as _;
        state.inner.next_in = $input as *mut _;

        let result = unsafe { sys::inflate(&mut state.inner, 0) };

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
                other => Err(crate::decoder::DecodeError(other)),
            },
        }
    }};
}

#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
mod zlib_common;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-ng"))]
pub use zlib_common::ZlibMode;
#[cfg(feature = "brotli-rust")]
mod brotli;
#[cfg(feature = "brotli-c")]
mod brotli_c;
#[cfg(any(feature = "zlib", feature = "zlib-static"))]
mod zlib;
#[cfg(feature = "zlib-ng")]
mod zlib_ng;
#[cfg(feature = "zstd")]
mod zstd;
#[cfg(feature = "zstd")]
pub use zstd::ZstdOptions;

impl<const N: usize> crate::Buffer<N> {
    ///Decodes `input` using `decoder` returning number of bytes consumed in `input`
    ///
    ///On success returns tuple with:
    ///- Number of consumed bytes in `input`
    ///- Decode status:
    ///    - In case of `Finished`, you should not continue to invoke decode until you reset decoder
    ///    - In case of `NeedOutput`, you should consume internal buffer.
    ///
    ///In case of error, internal buffer size will not change
    pub fn decode(&mut self, decoder: &mut Decoder, input: &[u8]) -> Result<(usize, DecodeStatus), DecodeError> {
        let spare_capacity = self.spare_capacity_mut();
        let spare_capacity_len = spare_capacity.len();

        let result = decoder.decode_uninit(input, spare_capacity);

        match result.status {
            Ok(status) => {
                self.cursor = self.cursor + spare_capacity_len - result.output_remain;
                Ok((input.len() - result.input_remain, status))
            }
            Err(error) => Err(error),
        }
    }
}
