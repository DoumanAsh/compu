//! `brotli` interface implementation

use core::{slice, ptr};
use crate::mem::Box;
use crate::mem::brotli_rust::BrotliAllocator;
use super::{Interface, Encoder, Encode, EncodeStatus, EncodeOp};
use super::brotli_common::BrotliOptions;

pub(crate) type Instance = brotli::enc::encode::BrotliEncoderStateStruct<BrotliAllocator>;

static BROTLI_RUST: Interface = Interface::new(
    reset_fn,
    encode_fn,
    drop_fn,
);

impl Interface {
    #[inline]
    ///Creates decoder with `brotli-rust` interface
    ///
    ///Never returns `None` (probably panics on OOM)
    pub fn brotli_rust(options: BrotliOptions) -> Encoder {
        let mut state = Box::new(instance());

        options.apply_rust(&mut state);

        let ptr = ptr::NonNull::from(Box::leak(state));
        BROTLI_RUST.inner_encoder(ptr.cast(), options.inner)
    }
}

impl EncodeOp {
    #[inline(always)]
    const fn into_rust_brotli(self) -> brotli::enc::encode::BrotliEncoderOperation {
        match self {
            Self::Process => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_PROCESS,
            Self::Flush => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_FLUSH,
            Self::Finish => brotli::enc::encode::BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
        }
    }
}

fn instance() -> Instance {
    brotli::enc::encode::BrotliEncoderCreateInstance(Default::default())
}

unsafe fn encode_fn(state: ptr::NonNull<u8>, input: *const u8, mut input_remain: usize, output: *mut u8, mut output_remain: usize, op: EncodeOp) -> Encode {
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

    let mut result = brotli::enc::encode::BrotliEncoderCompressStream(
        state,
        op.into_rust_brotli(),
        &mut input_remain, input, &mut 0,
        &mut output_remain, output, &mut 0,
        &mut None,
        &mut |_a, _b, _c, _d| (),
    );
    Encode {
        input_remain,
        output_remain,
        status: match result {
            0 => EncodeStatus::Error,
            _ => {
                result = brotli::enc::encode::BrotliEncoderHasMoreOutput(state);
                if result == 1 {
                    EncodeStatus::NeedOutput
                } else if op == EncodeOp::Finish {
                    EncodeStatus::Finished
                } else {
                    EncodeStatus::Continue
                }
            }
        }
    }
}

#[inline]
fn reset_fn(state: ptr::NonNull<u8>, opts: [u8; 2]) -> Option<ptr::NonNull<u8>> {
    let options = BrotliOptions::from_raw(opts);
    let mut state = unsafe {
        Box::from_raw(state.as_ptr() as *mut Instance)
    };

    *state = instance();
    options.apply_rust(&mut state);

    let ptr = Box::leak(state);

    Some(ptr::NonNull::from(ptr).cast())
}

#[inline]
fn drop_fn(state: ptr::NonNull<u8>) {
    let _ = unsafe {
        Box::from_raw(state.as_ptr() as *mut Instance)
    };
}
