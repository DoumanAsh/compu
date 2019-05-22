//!Writer compressor.

use crate::encoder::{Encoder};

use std::io::{self, Write};

///Compressor
///
///It writes compressed data to supplied writer that implements `Write`
///You can finish compression, by calling `push` with `finish` equal to `true`
///
///## Usage
///
///```rust,no_run
///use compu::encoder::{Encoder, BrotliEncoder};
///
///let data = vec![5; 5];
///let mut encoder = compu::compressor::write::Compressor::new(BrotliEncoder::default(), Vec::new());
///let result = encoder.push(&data, true).expect("Successful compression");
///assert!(result);
///assert!(encoder.encoder().is_finished());
///```
pub struct Compressor<E, W> {
    encoder: E,
    writer: W,
}

impl<E: Encoder, W: Write> Compressor<E, W> {
    ///Creates new instance
    pub fn new(encoder: E, writer: W) -> Self {
        Self {
            encoder,
            writer
        }
    }

    ///Returns reference to underlying encoder
    pub fn encoder(&self) -> &E {
        &self.encoder
    }

    ///Returns reference to underlying writer
    pub fn writer(&self) -> &W {
        &self.writer
    }

    ///Returns mutable reference to underlying writer
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    ///Pushes chunk to compressor
    ///
    ///Specify `finish` when last chunk is being pushed
    ///
    ///Returns whether operation is successful on successful write.
    ///
    ///Returns `io::Error` if underlying writer fails, note that if io::Error happens
    ///then compressed data will be lost
    pub fn push(&mut self, mut data: &[u8], finish: bool) -> io::Result<bool> {
        loop {
            let (remaining_input, _, result) = self.encoder.encode(data, &mut [], finish);
            if result == false {
                return Ok(false);
            }

            let consumed_input = data.len() - remaining_input;

            if consumed_input > 0 {
                self.writer.write_all(self.encoder.output().expect("To have encoder output"))?;
            }

            match remaining_input {
                0 => break,
                _ => {
                    data = &data[consumed_input..];
                }
            }
        }

        Ok(true)
    }

    ///Consumes self and returns underlying writer.
    pub fn take(self) -> W {
        self.writer
    }
}
