use compu::decoder::{Decoder, DecoderResult, BrotliDecoder};
use compu::encoder::{Encoder, BrotliEncoder};

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];
const DATA_COMPRESSED: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.br"),
    include_bytes!("data/alice29.txt.compressed.br")
];

#[test]
fn should_compress_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];

        let mut encoder = BrotliEncoder::default();
        let mut compressed = vec![0; encoder.compress_size_hint(data.len())];

        let (remaining_input, remaining_output, result) = encoder.encode(data, compressed.as_mut(), true);
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

        let mut encoder = compu::Compressor::new(BrotliEncoder::default());
        let result = encoder.push(data, true);
        assert!(result);
        assert!(encoder.inner().is_finished());
        let data_compressed = encoder.output();

        let mut decoder = compu::Decompressor::new(BrotliDecoder::default());

        let result = decoder.push(data_compressed);

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output().len(), data.len());
        assert!(decoder.output() == data);
        assert!(decoder.inner().is_finished());
        println!("Ok");
    }
}

#[test]
fn should_decompress_data_high() {
    for idx in 0..DATA.len() {
        print!("DATA set {}: ", idx);
        let data = DATA[idx];
        let data_compressed = DATA_COMPRESSED[idx];
        let mut decoder = compu::Decompressor::new(BrotliDecoder::default());

        let result = decoder.push(data_compressed);

        assert_eq!(result, DecoderResult::Finished);
        assert_eq!(decoder.output().len(), data.len());
        assert!(decoder.output() == data);
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
