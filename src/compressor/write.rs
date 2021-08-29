//!Writer compressor.

use crate::encoder::{Encoder, EncoderOp};

use std::io::{self, Write};

///Compressor
///
///It writes compressed data to supplied writer that implements `Write`
///You can finish compression, by calling `push` with `op` equal to `EncoderOp::Finish`
///
///## Usage
///
///```rust,no_run
///use compu::encoder::{Encoder, EncoderOp, BrotliEncoder};
///
///let data = vec![5; 5];
///let mut encoder = compu::compressor::write::Compressor::new(BrotliEncoder::default(), Vec::new());
///let result = encoder.push(&data, EncoderOp::Finish).expect("Successful compression");
///assert!(result > 0);
///assert!(encoder.encoder().is_finished());
///```
pub struct Compressor<E, W> where E: Encoder, W: Write {
    encoder: E,
    writer: W,
}

impl<E: Encoder, W: Write> Compressor<E, W> {
    ///Creates new instance
    pub fn new(encoder: E, writer: W) -> Self {
        Self {
            encoder,
            writer,
        }
    }

    #[inline]
    ///Returns reference to underlying encoder
    pub fn encoder(&self) -> &E {
        &self.encoder
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

    ///Pushes chunk to compressor
    ///
    ///Specify `op` equal to `EncoderOp::Finish` when last chunk is being pushed.
    ///Allowed to be empty chunk.
    ///
    ///Returns number of bytes written.
    ///
    ///Returns `io::Error` if underlying writer fails, note that if io::Error happens
    ///then compressed data will be lost
    ///
    ///Returns `io::ErrorKind::InvalidData` if compression fails.
    pub fn push(&mut self, mut data: &[u8], op: EncoderOp) -> io::Result<usize> {
        let mut written_len = 0;
        loop {
            let remaining_input = match E::HAS_INTERNAL_BUFFER {
                true => {
                    let (remaining_input, _, result) = self.encoder.encode(data, &mut [], op);

                    if result == false {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to compress"));
                    }

                    if let Some(output) = self.encoder.output() {
                        written_len += output.len();
                        self.writer.write_all(output)?;
                    }

                    remaining_input
                },
                false => loop {
                    let mut buffer = [0; 1024];
                    let (remaining_input, remaining_output, result) = self.encoder.encode(data, &mut buffer, op);

                    if result == false {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to compress"));
                    }

                    let consumed_output = buffer.len() - remaining_output;
                    written_len += consumed_output;
                    self.writer.write_all(&buffer[..consumed_output])?;

                    //If remaining_output is zero, we might need to write some extra.
                    //So we should probably fill writer via temp buffer until it is not ready
                    //as in case of zlib we can leave unfinished when remaining_input is 0
                    if remaining_output > 0 {
                        break remaining_input;
                    } else {
                        let consumed_input = data.len() - remaining_input;
                        data = &data[consumed_input..];
                    }
                },
            };

            match remaining_input {
                0 => break,
                _ => {
                    let consumed_input = data.len() - remaining_input;

                    data = &data[consumed_input..];
                }
            }
        }

        Ok(written_len)
    }

    #[inline]
    ///Enables automatic finish for compressor.
    pub fn auto_finish(self) -> AutoFinish<E, W> {
        AutoFinish {
            inner: self,
            last_op: None,
        }
    }

    #[inline]
    ///Consumes self and returns underlying writer.
    pub fn take(self) -> W {
        self.writer
    }
}

impl<E: Encoder, W: Write> Write for Compressor<E, W> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.push(buf, EncoderOp::Process)
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.push(buf, EncoderOp::Process).map(|_| ())
    }
}

///Wrapper over [Compressor](struct.Compressor.html) that automatically finishes
///compression if needed.
///
///Note that, finish will happen only if some operation happened, but not `EncoderOp::Finish`
///
///# Note:
///
///There is no buffering involved, as soon as data is ready, it is written.
///Which means it is not suitable for async IO where `WouldBlock` error can happen
///as it is considered a error case due to lack of any buffer
///
///# Note:
///
///It relies on fact that any operation after `EncoderOp::Finish` will fail.
pub struct AutoFinish<E, W> where E: Encoder, W: Write {
    inner: Compressor<E, W>,
    last_op: Option<EncoderOp>,
}

impl<E: Encoder, W: Write> AutoFinish<E, W> {
    #[inline]
    ///Returns reference to underlying encoder
    pub fn encoder(&self) -> &E {
        &self.inner.encoder
    }

    #[inline]
    ///Returns reference to underlying writer
    pub fn writer(&self) -> &W {
        &self.inner.writer
    }

    #[inline]
    ///Returns mutable reference to underlying writer
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.inner.writer
    }

    #[inline]
    ///Pushes chunk to underlying compressor.
    pub fn push(&mut self, data: &[u8], op: EncoderOp) -> io::Result<usize> {
        self.inner.push(data, op).map(|result| {
            self.last_op = Some(op);
            result
        })
    }
}

impl<E: Encoder, W: Write> Drop for AutoFinish<E, W> {
    fn drop(&mut self) {
        if let Some(op) = self.last_op {
            if op != EncoderOp::Finish {
                let _ = self.inner.push(&[], EncoderOp::Finish);
            }
        }
    }
}

impl<E: Encoder, W: Write> Write for AutoFinish<E, W> {
    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.push(buf, EncoderOp::Process)
    }

    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.writer.flush()
    }

    #[inline(always)]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.push(buf, EncoderOp::Process).map(|_| ())
    }
}
