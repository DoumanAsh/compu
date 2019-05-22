//!In-memory compressor.
use crate::encoder::{Encoder, EncoderOp};

use core::slice;
use core::cmp;

///Compressor
///
///It stores compressed data in its internal buffer.
///Once last chunk is pushed, by calling `push` with `op` equal to `EncoderOp::FINISH`
///Result can be retrieved using `output` or `take`.
///
///## Usage
///
///```rust,no_run
///use compu::encoder::{Encoder, EncoderOp, BrotliEncoder};
///
///let data = vec![5; 5];
///let mut encoder = compu::compressor::memory::Compressor::new(BrotliEncoder::default());
///let result = encoder.push(&data, EncoderOp::Finish);
///assert!(result);
///assert!(encoder.encoder().is_finished());
///```
pub struct Compressor<E> {
    encoder: E,
    output: Vec<u8>,
}

impl<E: Encoder> Compressor<E> {
    ///Creates new instance
    pub fn new(encoder: E) -> Self {
        Self {
            encoder,
            output: Vec::with_capacity(0),
        }
    }

    ///Returns reference to underlying encoder
    pub fn encoder(&self) -> &E {
        &self.encoder
    }

    ///Pushes chunk to compressor
    ///
    ///Specify `op` as `EncoderOp::Finish` when last chunk is being pushed
    ///
    ///Returns whether operation is successful.
    pub fn push(&mut self, mut data: &[u8], op: EncoderOp) -> bool {
        let size_hint = self.encoder.compress_size_hint(data.len());
        self.output.reserve(size_hint);

        loop {
            let offset = self.output.len();
            let output_slice = unsafe {
                slice::from_raw_parts_mut(self.output.as_mut_ptr().offset(offset as isize), self.output.capacity() - offset)
            };

            let (remaining_input, remaining_output, result) = self.encoder.encode(data, output_slice, op);
            let consumed_output = output_slice.len() - remaining_output;
            unsafe {
                self.output.set_len(offset + consumed_output);
            }

            if result == false {
                return false;
            }

            match remaining_input {
                0 => return true,
                remaining_input => {
                    let consumed_input = data.len() - remaining_input;
                    data = &data[consumed_input..];
                    self.output.reserve(cmp::min(size_hint, 1024));
                }
            }
        }
    }

    ///Returns slice of currently compressed data
    pub fn output<'a>(&'a self) -> &'a [u8] {
        &self.output
    }

    ///Returns slice of currently decompressed data and marks it as consumed
    ///
    ///After calling this function, the underlying buffer basically sets length equal to 0
    ///allowing to overwrite already written data with further pushes.
    pub fn consume_output<'a>(&'a mut self) -> &'a [u8] {
        let len = self.output.len();

        unsafe {
            self.output.set_len(0);
            core::slice::from_raw_parts(self.output.as_ptr(), len)
        }
    }

    ///Consumes self and returns underlying data.
    pub fn take(self) -> Vec<u8> {
        self.output
    }
}
