use compu::decoder::{Decoder, DecoderResult, ZlibDecoder};
use compu::encoder::{Encoder, ZlibEncoder, EncoderOp};

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];
const DATA_DEFLATED: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.deflate"),
    include_bytes!("data/alice29.txt.compressed.deflate")
];

#[test]
fn zlib_should_decompress_deflated_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];
        let data_compressed = DATA_DEFLATED[idx];
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
    }
}

#[test]
fn zlib_should_compress_deflated_data() {
    for idx in 0..DATA.len() {
        println!("DATA set {}:\n", idx);
        let data = DATA[idx];

        let mut encoder = ZlibEncoder::default();
        let mut output = vec![0; data.len()];

        let (remaining_input, remaining_output, result) = encoder.encode(data, output.as_mut(), EncoderOp::Finish);
        assert_eq!(result, true);
        assert_eq!(remaining_input, 0);
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
