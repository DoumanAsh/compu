//!Writer decompressor.

use crate::decoder::{DecoderResult, Decoder};

use std::io::{self, Write};

///Decompressor
///
///It writes decompressed data to supplied writer that implements `Write`.
///
///# Note:
///
///There is no buffering involved, as soon as data is ready, it is written.
///Which means it is not suitable for async IO where `WouldBlock` error can happen
///as it is considered a error case due to lack of any buffer
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
///assert_eq!(result.0, DecoderResult::Finished);
///assert!(decoder.decoder().is_finished());
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

    #[inline]
    ///Returns reference to underlying decoder
    pub fn decoder(&self) -> &D {
        &self.decoder
    }

    #[inline]
    ///Returns reference to underlying writer
    pub fn writer(&self) -> &W {
        &self.writer
    }

    #[inline]
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
    pub fn push(&mut self, mut data: &[u8]) -> io::Result<(DecoderResult, usize)> {
        let mut written_len = 0;
        let result = loop {
            let (remaining_input, result) = match D::HAS_INTERNAL_BUFFER {
                true => {
                    let (remaining_input, _, result) = self.decoder.decode(data, &mut []);

                    if let Some(output) = self.decoder.output() {
                        self.writer.write_all(output)?;
                        written_len += output.len();
                    }

                    (remaining_input, result)
                },
                false => {
                    let mut buffer = [0; 1024];
                    let (remaining_input, remaining_output, result) = self.decoder.decode(data, &mut buffer);

                    let consumed_output = buffer.len() - remaining_output;
                    let output = &buffer[..consumed_output];
                    self.writer.write_all(output)?;
                    written_len += output.len();

                    (remaining_input, result)
                }
            };

            match result {
                DecoderResult::NeedOutput => {
                    let consumed_input = data.len() - remaining_input;
                    data = &data[consumed_input..];
                }
                result => break result
            }
        };

        Ok((result, written_len))
    }

    #[inline]
    ///Consumes self and returns underlying writer.
    pub fn take(self) -> W {
        self.writer
    }
}

impl<D: Decoder, W: Write> Write for Decompressor<D, W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self.push(data)? {
            (DecoderResult::Finished, result) => Ok(result),
            (DecoderResult::NeedInput, result) => Ok(result),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to decompress")),
        }
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.push(buf).and_then(|(result, _)| match result {
            DecoderResult::NeedInput | DecoderResult::Finished => Ok(()),
            DecoderResult::Other(code) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unable to decompress. Error: {}", code))),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to decompress.")),
        })
    }
}
