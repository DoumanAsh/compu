use compu::decoder::{Decoder, DecoderResult, BrotliDecoder};
use compu::encoder::{Encoder, EncoderOp, BrotliEncoder};
use compu::compressor::Compress;
use compu::decompressor::Decompress;

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

        let compressed2 = data.compress(BrotliEncoder::default()).expect("Compress brotli");
        assert!(compressed == compressed2);

        let mut compressed3 = Vec::new();
        data.compress_into(BrotliEncoder::default(), &mut compressed3).expect("Compress into brotli");
        assert!(compressed == compressed3);

        let data_chunks = data.chunks(5).collect::<Vec<_>>();
        let compressed4 = data_chunks.compress(BrotliEncoder::default()).expect("Compress chunks brotli");
        assert!(compressed4 == compressed);

        let mut compressed5 = Vec::new();
        data_chunks.compress_into(BrotliEncoder::default(), &mut compressed5).expect("Compress into chunks brotli");
        assert!(compressed5 == compressed);

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

        let output2 = data_compressed.decompress(BrotliDecoder::default()).expect("decompress brotli");
        assert!(output2 == data);
        let mut output3 = Vec::new();
        data_compressed.decompress_into(BrotliDecoder::default(), &mut output3).expect("decompress into brotli");
        assert!(output3 == data);

        let data_compressed_chunks = data_compressed.chunks(5).collect::<Vec<_>>();
        let output4 = data_compressed_chunks.decompress(BrotliDecoder::default()).expect("decompress brotli");
        assert!(output4 == data);

        let mut output5 = Vec::new();
        data_compressed_chunks.decompress_into(BrotliDecoder::default(), &mut output5).expect("decompress brotli");
        assert!(output5 == data);
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
        assert_eq!(result, (DecoderResult::Finished, data.len()));
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
