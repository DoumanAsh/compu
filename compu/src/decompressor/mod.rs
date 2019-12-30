//!High level Decompression API

pub mod memory;
pub mod write;

use crate::decoder::{Decoder, DecoderResult};

use std::io;

///Describes interface for item that can be compressed
pub trait Decompress {
    ///Performs decompression using provided encoder.
    ///
    ///On failure or incomplete input returns `None`
    fn decompress<D: Decoder>(&self, decoder: D) -> Option<Vec<u8>>;

    ///Decompresses and writes output into provided writer.
    fn decompress_into<D: Decoder, W: io::Write>(&self, decoder: D, out: W) -> io::Result<usize>;
}

impl<T: core::convert::AsRef<[u8]>> Decompress for T {
    fn decompress<D: Decoder>(&self, decoder: D) -> Option<Vec<u8>> {
        let mut decoder = memory::Decompressor::new(decoder);
        match decoder.push(self.as_ref()) {
            DecoderResult::Finished => Some(decoder.take()),
            _ => None,
        }
    }

    fn decompress_into<D: Decoder, W: io::Write>(&self, decoder: D, out: W) -> io::Result<usize> {
        let mut decoder = write::Decompressor::new(decoder, out);
        match decoder.push(self.as_ref())? {
            (DecoderResult::Finished, written_len) => Ok(written_len),
            (DecoderResult::NeedInput, _) => Err(io::Error::new(io::ErrorKind::Other, "Decompression needs more input")),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Failed to decompress")),
        }
    }
}

impl<T: core::convert::AsRef<[u8]>> Decompress for [T] {
    fn decompress<D: Decoder>(&self, decoder: D) -> Option<Vec<u8>> {
        let last_idx = match self.len() {
            0 => return Some(Vec::with_capacity(0)),
            len => len - 1,
        };

        let mut decoder = memory::Decompressor::new(decoder);

        for idx in 0..last_idx {
            let item = unsafe { self.get_unchecked(idx).as_ref() };

            match decoder.push(item) {
                DecoderResult::NeedInput => continue,
                _ => return None,
            }
        }

        let item = unsafe { self.get_unchecked(last_idx).as_ref() };
        match decoder.push(item) {
            DecoderResult::Finished => Some(decoder.take()),
            _ => None,
        }
    }

    fn decompress_into<D: Decoder, W: io::Write>(&self, decoder: D, out: W) -> io::Result<usize> {
        let last_idx = match self.len() {
            0 => return Ok(0),
            len => len - 1,
        };

        let mut result = 0;
        let mut decoder = write::Decompressor::new(decoder, out);

        for idx in 0..last_idx {
            let item = unsafe { self.get_unchecked(idx).as_ref() };

            result += match decoder.push(item)? {
                (DecoderResult::NeedInput, written_len) => written_len,
                (DecoderResult::Finished, _) => return Err(io::Error::new(io::ErrorKind::Other, "Too much input")),
                _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to decompress")),
            }
        }

        let item = unsafe { self.get_unchecked(last_idx).as_ref() };
        result += match decoder.push(item)? {
            (DecoderResult::Finished, written_len) => written_len,
            (DecoderResult::NeedInput, _) => return Err(io::Error::new(io::ErrorKind::Other, "Not enough input")),
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to decompress")),
        };

        Ok(result)
    }
}
