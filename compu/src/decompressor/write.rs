//!Writer decompressor.

use crate::decoder::{DecoderResult, Decoder};

use std::io::{self, Write};

///Decompressor
///
///It writes decompressed data to supplied writer that implements `Write`
///
///## Usage
///
///```rust,no_run
///use compu::decoder::{Decoder, DecoderResult, BrotliDecoder};
///
///let data = vec![5; 5];
///let mut decoder = compu::decompressor::write::Decompressor::new(BrotliDecoder::default(), Vec::new());
///
///let result = decoder.push(&data).expect("Successful decompression");
///
///assert_eq!(result, DecoderResult::Finished);
///```
pub struct Decompressor<D, W> {
    decoder: D,
    writer: W,
}

impl<D: Decoder, W: Write> Decompressor<D, W> {
    ///Creates new instance
    pub fn new(decoder: D, writer: W) -> Self {
        Self {
            decoder,
            writer
        }
    }

    ///Returns reference to underlying decoder
    pub fn decoder(&self) -> &D {
        &self.decoder
    }

    ///Returns reference to underlying writer
    pub fn writer(&self) -> &W {
        &self.writer
    }

    ///Returns mutable reference to underlying writer
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }


    ///Pushes data into, and returns Decoder's operation status
    ///
    ///- `DecoderResult::Finished` indicates decompression has finished successfully.
    ///- `DecoderResult::NeedInput` indicates that further data is necessary.
    ///- `DecoderResult::NeedOutput` SHOULD not happen
    ///
    ///Any other variants indicates error.
    ///
    ///Returns `io::Error` if underlying writer fails, note that if io::Error happens
    ///then compressed data will be lost
    pub fn push(&mut self, mut data: &[u8]) -> io::Result<DecoderResult> {
        let result = loop {
            let (remaining_input, _, result) = self.decoder.decode(data, &mut []);

            let consumed_input = data.len() - remaining_input;
            if consumed_input > 0 {
                self.writer.write_all(self.decoder.output().expect("To have decoder output"))?;
            }

            match result {
                DecoderResult::NeedOutput => {
                    data = &data[consumed_input..];
                }
                result => break result
            }
        };

        Ok(result)
    }

    ///Consumes self and returns underlying writer.
    pub fn take(self) -> W {
        self.writer
    }
}
