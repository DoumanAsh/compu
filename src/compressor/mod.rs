//! Compression module

pub mod memory;
pub mod write;

use crate::encoder::{Encoder, EncoderOp};

use std::io;

///Describes interface for item that can be compressed
pub trait Compress {
    ///Performs compression using provided encoder.
    ///
    ///On failure returns `None`
    fn compress<E: Encoder>(&self, encoder: E) -> Option<Vec<u8>>;

    ///Performs compression using provided encoder and writer.
    fn compress_into<E: Encoder, W: io::Write>(&self, encoder: E, out: W) -> io::Result<usize>;
}

impl<T: core::convert::AsRef<[u8]>> Compress for T {
    fn compress<E: Encoder>(&self, encoder: E) -> Option<Vec<u8>> {
        let mut encoder = memory::Compressor::new(encoder);
        match encoder.push(self.as_ref(), EncoderOp::Finish) {
            true => Some(encoder.take()),
            false => None,
        }
    }

    fn compress_into<E: Encoder, W: io::Write>(&self, encoder: E, out: W) -> io::Result<usize> {
        let mut encoder = write::Compressor::new(encoder, out);
        encoder.push(self.as_ref(), EncoderOp::Finish)
    }
}

impl<T: core::convert::AsRef<[u8]>> Compress for [T] {
    fn compress<E: Encoder>(&self, encoder: E) -> Option<Vec<u8>> {
        let last_idx = match self.len() {
            0 => return Some(Vec::with_capacity(0)),
            len => len - 1,
        };

        let mut encoder = memory::Compressor::new(encoder);

        for idx in 0..last_idx {
            let item = unsafe { self.get_unchecked(idx).as_ref() };

            match encoder.push(item, EncoderOp::Process) {
                true => continue,
                false => return None,
            }
        }

        let item = unsafe { self.get_unchecked(last_idx).as_ref() };
        match encoder.push(item, EncoderOp::Finish) {
            true => Some(encoder.take()),
            false => None,
        }
    }

    fn compress_into<E: Encoder, W: io::Write>(&self, encoder: E, out: W) -> io::Result<usize> {
        let last_idx = match self.len() {
            0 => return Ok(0),
            len => len - 1,
        };

        let mut result = 0;
        let mut encoder = write::Compressor::new(encoder, out);

        for idx in 0..last_idx {
            let item = unsafe { self.get_unchecked(idx).as_ref() };

            result += encoder.push(item, EncoderOp::Process)?;
        }

        let item = unsafe { self.get_unchecked(last_idx).as_ref() };
        result += encoder.push(item, EncoderOp::Finish)?;
        Ok(result)
    }
}
