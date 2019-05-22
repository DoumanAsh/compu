use compu::decoder::{Decoder, DecoderResult, BrotliDecoder};
use compu::encoder::{Encoder, EncoderOp, BrotliEncoder};

use std::io::Write;

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];
const DATA_COMPRESSED: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.br"),
    include_bytes!("data/alice29.txt.compressed.br")
];

#[test]
fn should_auto_finish_compressor() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];

        let mut compressed = Vec::new();

        let mut encoder = compu::compressor::write::Compressor::new(BrotliEncoder::default(), &mut compressed).auto_finish();
        encoder.write(data).expect("Successful write compression");
        assert!(!encoder.encoder().is_finished());

        drop(encoder);

        let mut decompressed = Vec::new();
        let mut decoder = compu::decompressor::write::Decompressor::new(BrotliDecoder::default(), &mut decompressed);
        decoder.write(&compressed).expect("Successful write compression");
        assert!(decoder.decoder().is_finished());

        assert_eq!(decompressed, data);
    }
}

#[test]
fn should_compress_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];

        let mut encoder = BrotliEncoder::default();
        let mut compressed = vec![0; encoder.compress_size_hint(data.len())];

        let (remaining_input, remaining_output, result) = encoder.encode(data, compressed.as_mut(), EncoderOp::Finish);
        //Remove extra
        compressed.truncate(compressed.len() - remaining_output);

        assert_eq!(result, true);
        assert_eq!(remaining_input, 0);

        let mut decoder = BrotliDecoder::default();
        let mut decompressed = vec![0; data.len()];

        let (remaining_input, remaining_output, result) = decoder.decode(&compressed, decompressed.as_mut());

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(remaining_input, 0);
        assert_eq!(remaining_output, 0);
        assert_eq!(decompressed, data);
    }
}

#[test]
fn should_decompress_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];
        let data_compressed = DATA_COMPRESSED[idx];
        let mut decoder = BrotliDecoder::default();

        let mut output = vec![0; data.len()];

        let (remaining_input, remaining_output, result) = decoder.decode(data_compressed, output.as_mut());

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(remaining_input, 0);
        assert_eq!(remaining_output, 0);
        assert_eq!(output, data);
    }
}

#[test]
fn should_compress_data_high() {
    for idx in 0..DATA.len() {
        print!("DATA set {}: ", idx);
        let data = DATA[idx];

        let mut data_compressed = Vec::new();

        let mut encoder = compu::compressor::memory::Compressor::new(BrotliEncoder::default());
        let result = encoder.push(&data[..data.len()/2], EncoderOp::Process);
        assert!(result);
        assert!(!encoder.encoder().is_finished());
        data_compressed.extend_from_slice(encoder.consume_output());
        let result = encoder.push(&data[data.len()/2..], EncoderOp::Finish);
        assert!(result);
        assert!(encoder.encoder().is_finished());
        data_compressed.extend_from_slice(encoder.consume_output());

        let mut encoder = compu::compressor::write::Compressor::new(BrotliEncoder::default(), Vec::new());
        let result = encoder.push(data, EncoderOp::Finish).expect("Successful write compression");
        assert!(result > 0);
        assert!(encoder.encoder().is_finished());
        let data_compressed_write = encoder.take();
        assert_eq!(data_compressed_write, data_compressed);

        let mut decoder = compu::decompressor::memory::Decompressor::new(BrotliDecoder::default());

        let result = decoder.push(&data_compressed);

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output().len(), data.len());
        assert!(decoder.output() == data);
        assert!(decoder.decoder().is_finished());

        let mut decoder = compu::decompressor::write::Decompressor::new(BrotliDecoder::default(), Vec::new());
        let result = decoder.push(&data_compressed).expect("Successful write decompression");
        assert_eq!(result, DecoderResult::Finished);
        assert!(decoder.decoder().is_finished());
        let output = decoder.take();
        assert_eq!(output.len(), data.len());
        assert!(output == data);

        println!("Ok");
    }
}

#[test]
fn should_decompress_data_high() {
    for idx in 0..DATA.len() {
        print!("DATA set {}: ", idx);
        let data = DATA[idx];
        let data_compressed = DATA_COMPRESSED[idx];
        let mut data_decompressed = Vec::with_capacity(data.len());

        let mut decoder = compu::decompressor::memory::Decompressor::new(BrotliDecoder::default());

        let result = decoder.push(&data_compressed[..data_compressed.len()/2]);

        assert_eq!(result, DecoderResult::NeedInput);
        data_decompressed.extend_from_slice(decoder.consume_output());
        assert_eq!(decoder.output().len(), 0);

        let result = decoder.push(&data_compressed[data_compressed.len()/2..]);
        assert_eq!(result, DecoderResult::Finished);
        data_decompressed.extend_from_slice(decoder.consume_output());
        assert_eq!(decoder.output().len(), 0);

        assert_eq!(data_decompressed, data);

        println!("Ok");
    }
}

#[test]
fn insufficient_output_buffer() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];
        let data_compressed = DATA_COMPRESSED[idx];
        let mut decoder = BrotliDecoder::default();

        let mut output = vec![0; DATA.len()/2];

        let (remaining_input, remaining_output, result) = decoder.decode(data_compressed, output.as_mut());
        assert_eq!(result, DecoderResult::NeedOutput);

        let output_offset = output.len() - remaining_output;
        output.resize(data.len(), 0);

        let data_compressed = &data_compressed[data_compressed.len() - remaining_input..];

        let (remaining_input, remaining_output, result) = decoder.decode(data_compressed, &mut output.as_mut_slice()[output_offset..]);
        assert_eq!(remaining_input, 0);
        assert_eq!(remaining_output, 0);
        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(output, data);
    }
}
