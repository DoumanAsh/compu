use core::{ptr, slice};

use super::{Decode, DecodeError, DecodeStatus, Decoder, Interface};
use crate::mem::brotli_rust::BrotliAllocator;
use crate::mem::Box;
pub(crate) type Instance = brotli::BrotliState<BrotliAllocator, BrotliAllocator, BrotliAllocator>;

static BROTLI_RUST: Interface = Interface::new(
    decode_fn,
    reset_fn,
    drop_fn,
    describe_error_fn
);

impl Interface {
    #[inline]
    ///Creates decoder with `brotli-rust` interface
    ///
    ///Panics on OOM issues
    pub fn brotli_rust() -> Decoder {
        let state = Box::new(instance());

        let ptr = ptr::NonNull::from(Box::leak(state));
        BROTLI_RUST.inner_decoder(ptr.cast())
    }
}
#[inline]
fn instance() -> Instance {
    Instance::new(Default::default(), Default::default(), Default::default())
}

#[inline]
unsafe fn decode_fn(state: ptr::NonNull<u8>, input: *const u8, mut input_remain: usize, output: *mut u8, mut output_remain: usize) -> Decode {
    let state = unsafe {
        &mut *(state.as_ptr() as *mut Instance)
    };

    let input = unsafe {
        slice::from_raw_parts(input, input_remain)
    };
    //Potential UB but it is non-issue
    //Complain here
    //https://github.com/dropbox/rust-brotli/issues/177
    let output = unsafe {
        slice::from_raw_parts_mut(output, output_remain)
    };

    let result = brotli::BrotliDecompressStream(&mut input_remain, &mut 0, input, &mut output_remain, &mut 0, output, &mut 0, state);

    Decode {
        input_remain,
        output_remain,
        status: match result {
            brotli::BrotliResult::ResultSuccess => Ok(DecodeStatus::Finished),
            brotli::BrotliResult::NeedsMoreInput => Ok(DecodeStatus::NeedInput),
            brotli::BrotliResult::NeedsMoreOutput => Ok(DecodeStatus::NeedOutput),
            brotli::BrotliResult::ResultFailure => Err(DecodeError(state.error_code as _)),
        },
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>) -> Option<ptr::NonNull<u8>> {
    let mut state = unsafe {
        Box::from_raw(state.as_ptr() as *mut Instance)
    };

    *state = instance();
    let ptr = Box::leak(state);
    Some(ptr::NonNull::from(ptr).cast())
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    let _ = unsafe { Box::from_raw(state.as_ptr() as *mut Instance) };
}

#[inline]
fn describe_error_fn(code: i32) -> Option<&'static str> {
    match code {
        0 => Some("NO_ERROR"),
        //1 => Some("SUCCESS"),
        //2 => Some("NEEDS_MORE_INPUT"),
        //3 => Some("NEEDS_MORE_OUTPUT"),

        /* Errors caused by invalid input */
        -1 => Some("ERROR_FORMAT_EXUBERANT_NIBBLE"),
        -2 => Some("ERROR_FORMAT_RESERVED"),
        -3 => Some("ERROR_FORMAT_EXUBERANT_META_NIBBLE"),
        -4 => Some("ERROR_FORMAT_SIMPLE_HUFFMAN_ALPHABET"),
        -5 => Some("ERROR_FORMAT_SIMPLE_HUFFMAN_SAME"),
        -6 => Some("ERROR_FORMAT_FL_SPACE"),
        -7 => Some("ERROR_FORMAT_HUFFMAN_SPACE"),
        -8 => Some("ERROR_FORMAT_CONTEXT_MAP_REPEAT"),
        -9 => Some("ERROR_FORMAT_BLOCK_LENGTH_1"),
        -10 => Some("ERROR_FORMAT_BLOCK_LENGTH_2"),
        -11 => Some("ERROR_FORMAT_TRANSFORM"),
        -12 => Some("ERROR_FORMAT_DICTIONARY"),
        -13 => Some("ERROR_FORMAT_WINDOW_BITS"),
        -14 => Some("ERROR_FORMAT_PADDING_1"),
        -15 => Some("ERROR_FORMAT_PADDING_2"),
        -16 => Some("ERROR_FORMAT_DISTANCE"),

        /* -17..-18 codes are reserved */
        -19 => Some("ERROR_DICTIONARY_NOT_SET"),
        -20 => Some("ERROR_INVALID_ARGUMENTS"),

        /* Memory allocation problems */
        -21 => Some("ERROR_ALLOC_CONTEXT_MODES"),
        /* Literal => insert and distance trees together */
        -22 => Some("ERROR_ALLOC_TREE_GROUPS"),
        /* -23..-24 codes are reserved for distinct tree groups */
        -25 => Some("ERROR_ALLOC_CONTEXT_MAP"),
        -26 => Some("ERROR_ALLOC_RING_BUFFER_1"),
        -27 => Some("ERROR_ALLOC_RING_BUFFER_2"),
        /* -28..-29 codes are reserved for dynamic ring-buffer allocation */
        -30 => Some("ERROR_ALLOC_BLOCK_TYPE_TREES"),

        /* "Impossible" states */
        -31 => Some("ERROR_UNREACHABLE"),
        _ => None,
    }
}
