use compu::decoder::{self, Decoder, DecoderResult, ZlibDecoder};
use compu::encoder::{self, Encoder, ZlibEncoder, EncoderOp};
use compu::compressor::Compress;
use compu::decompressor::Decompress;

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];
const DATA_DEFLATED: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.deflate"),
    include_bytes!("data/alice29.txt.compressed.deflate")
];
const DATA_GZIP: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.gz"),
    include_bytes!("data/alice29.txt.compressed.gz")
];

#[test]
fn zlib_should_decompress_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];
        let data_compressed = DATA_DEFLATED[idx];
        let data_gzip = DATA_GZIP[idx];
        let mut decoder = ZlibDecoder::default();

        let mut output = vec![0; data.len()];

        let (remaining_input, remaining_output, result) = decoder.decode(data_compressed, output.as_mut());

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(remaining_input, 0);
        assert_eq!(remaining_output, 0);
        assert_eq!(output, data);

        let mut decoder = compu::decompressor::memory::Decompressor::new(ZlibDecoder::default());

        let result = decoder.push(&data_compressed[..data_compressed.len()/2]);
        assert_eq!(result, DecoderResult::NeedInput);

        let result = decoder.push(&data_compressed[data_compressed.len()/2..]);
        assert_eq!(result, DecoderResult::Finished);

        assert_eq!(decoder.output(), data);
        let output = data_compressed.decompress(ZlibDecoder::default()).expect("decompress zlib");
        assert!(output == data);
        let mut output2 = Vec::new();
        data_compressed.decompress_into(ZlibDecoder::default(), &mut output2).expect("decompress zlib");
        assert!(output2 == data);

        let data_compressed_chunks = data_compressed.chunks(5).collect::<Vec<_>>();
        let output3 = data_compressed_chunks.decompress(ZlibDecoder::default()).expect("decompress chunks zlib");
        assert!(output3 == data);
        let mut output4 = Vec::new();
        data_compressed_chunks.decompress_into(ZlibDecoder::default(), &mut output4).expect("decompress zlib");
        assert!(output4 == data);

        let mut written_output = Vec::new();
        let mut decoder = compu::decompressor::write::Decompressor::new(ZlibDecoder::default(), &mut written_output);
        let result = decoder.push(data_gzip).expect("To write decompressed data");
        assert_eq!(result, (DecoderResult::Finished, data.len()));
        assert_eq!(written_output, data);
    }
}

#[test]
fn zlib_should_compress_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];

        //Zlib
        let mut output = Vec::new();
        let mut encoder = compu::compressor::write::Compressor::new(ZlibEncoder::default(), &mut output);

        let written = encoder.push(data, EncoderOp::Finish).expect("To write compressed data");
        assert!(written > 0);
        assert!(encoder.encoder().is_finished());

        let mut decoder = compu::decompressor::memory::Decompressor::new(ZlibDecoder::default());
        let result = decoder.push(&output);
        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output(), data);

        let output2 = data.compress(ZlibEncoder::default()).expect("Compress zlib");
        assert!(output == output2);

        let mut output3 = Vec::new();
        data.compress_into(ZlibEncoder::default(), &mut output3).expect("Compress into zlib");
        assert!(output == output3);

        let data_chunks = data.chunks(5).collect::<Vec<_>>();
        let output4 = data_chunks.compress(ZlibEncoder::default()).expect("Compress chunks zlib");
        assert!(output4 == output);

        let mut output5 = Vec::new();
        data_chunks.compress_into(ZlibEncoder::default(), &mut output5).expect("Compress into chunks zlib");
        assert!(output5 == output);

        let mut decoder = compu::decompressor::memory::Decompressor::new(ZlibDecoder::default());
        let result = decoder.push(&output2);
        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output(), data);

        //Deflate
        let mut output = Vec::new();
        let mut encoder = compu::compressor::write::Compressor::new(ZlibEncoder::new(&encoder::zlib::ZlibOptions::default().mode(encoder::zlib::ZlibMode::Deflate)), &mut output);

        let written = encoder.push(data, EncoderOp::Finish).expect("To write compressed data");
        assert!(written > 0);
        assert!(encoder.encoder().is_finished());

        let mut decoder = compu::decompressor::memory::Decompressor::new(ZlibDecoder::new(&decoder::zlib::ZlibOptions::default().mode(decoder::zlib::ZlibMode::Deflate)));
        let result = decoder.push(&output);
        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output(), data);

        //Gzip
        let mut encoder = ZlibEncoder::new(&encoder::zlib::ZlibOptions::default().mode(encoder::zlib::ZlibMode::Gzip).compression(9));
        let mut output = vec![0; data.len()];

        let (remaining_input, mut remaining_output, result) = encoder.encode(data, output.as_mut(), EncoderOp::Finish);
        assert_eq!(result, true);
        assert_eq!(remaining_input, 0);

        //On very small inputs gzip has quite overhead
        if remaining_output == 0 {
            assert!(!encoder.is_finished());
            let old_len = output.len();
            output.reserve(100);
            unsafe {
                output.set_len(100);
            }
            let (_, new_remaining_output, result) = encoder.encode(&[], &mut output[old_len..], EncoderOp::Finish);

            assert!(result);
            assert!(encoder.is_finished());

            remaining_output = new_remaining_output;
        } else {
            assert!(encoder.is_finished());
        }

        let written_len = output.len() - remaining_output;
        unsafe {
            output.set_len(written_len);
        }

        let mut decoder = compu::decompressor::memory::Decompressor::new(ZlibDecoder::default());
        let result = decoder.push(&output);
        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output(), data);

    }
}
