//!In-memory decompressor.

use crate::decoder::{DecoderResult, Decoder};

///Decompressor
///
///It stores decompressed data in its internal buffer.
///Once `push` returns `DecoderResult::Finished` or `DecoderResult::Error`, it it no longer expected to accept new input.
///Result can be retrieved using `output` or `take`.
///
///## Usage
///
///```rust,no_run
///use compu::decoder::{Decoder, DecoderResult, BrotliDecoder};
///
///let data = vec![5; 5];
///let mut decoder = compu::decompressor::memory::Decompressor::new(BrotliDecoder::default());
///
///let result = decoder.push(&data);
///
///assert_eq!(result, DecoderResult::Finished);
///```
pub struct Decompressor<D> {
    decoder: D,
    output: Vec<u8>,
}

impl<D: Decoder> Decompressor<D> {
    ///Creates new instance
    pub fn new(decoder: D) -> Self {
        Self {
            decoder,
            output: Vec::with_capacity(1024),
        }
    }

    ///Returns reference to underlying decoder
    pub fn decoder(&self) -> &D {
        &self.decoder
    }

    ///Pushes data into, and returns Decoder's operation status
    ///
    ///- `DecoderResult::Finished` indicates decompression has finished successfully.
    ///- `DecoderResult::NeedInput` indicates that further data is necessary.
    ///- `DecoderResult::NeedOutput` SHOULD not happen
    ///
    ///Any other variants indicates error.
    pub fn push(&mut self, mut data: &[u8]) -> DecoderResult {
        loop {
            let offset = self.output.len();
            let output_slice = unsafe {
                core::slice::from_raw_parts_mut(self.output.as_mut_ptr().offset(offset as isize), self.output.capacity() - offset)
            };

            let (remaining_input, remaining_output, result) = self.decoder.decode(data, output_slice);
            let consumed_output = output_slice.len() - remaining_output;
            unsafe {
                self.output.set_len(offset + consumed_output);
            }

            match result {
                DecoderResult::NeedOutput => {
                    let consumed_input = data.len() - remaining_input;
                    data = &data[consumed_input..];
                    self.output.reserve(1024);
                },
                result => break result
            }
        }
    }

    ///Returns slice of currently decompressed data
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
