use compu::decoder;
use decoder::{Interface, DecodeStatus, DecodeError};

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];
const DATA_BROTLI: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.br"),
    include_bytes!("data/alice29.txt.compressed.br")
];
const DATA_GZIP: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.gz"),
    include_bytes!("data/alice29.txt.compressed.gz")
];
const DATA_ZSTD: [&[u8]; 2] = [
    include_bytes!("data/10x10y.compressed.zstd"),
    include_bytes!("data/alice29.txt.compressed.zstd")
];

fn test_case(idx: usize, decoder: &mut decoder::Decoder, data: &[u8], compressed: &[u8]) {
    println!("{idx}: DATA.len()={} || COMPRESSED.len()={}", data.len(), compressed.len());

    //Full
    let mut output = vec![0; data.len()];
    let result = decoder.decode(compressed, output.as_mut());
    assert_eq!(result.status, Ok(DecodeStatus::Finished));
    assert_eq!(result.input_remain, 0);
    assert_eq!(result.output_remain, 0);
    assert_eq!(data, output);
    decoder.reset();

    //Partial buffer
    let result = decoder.decode(compressed, &mut output[..DATA.len()/2]);
    assert_eq!(result.status, Ok(DecodeStatus::NeedOutput));
    assert_eq!(result.output_remain, 0);

    //decoders like Brotli come with own buffers that store everything inside, while zlib will wait
    //for more output buffer to be available before proceeding.
    let remaining = &compressed[compressed.len() - result.input_remain..];
    let result = decoder.decode(remaining, &mut output[DATA.len()/2..]);
    assert_eq!(result.status, Ok(DecodeStatus::Finished));
    assert_eq!(data, output);
    decoder.reset();

    let error = DecodeError::no_error();
    let error = decoder.describe_error(error).expect("to get generic error");
    println!("error={error}");
}

#[cfg(feature = "brotli-c")]
#[test]
fn should_decode_brotli_c() {
    let mut decoder = Interface::brotli_c().expect("create brotli decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut decoder, DATA[idx], DATA_BROTLI[idx]);
    }
}

#[cfg(feature = "brotli-rust")]
#[test]
fn should_decode_brotli_rust() {
    let mut decoder = Interface::brotli_rust();
    for idx in 0..DATA.len() {
        test_case(idx, &mut decoder, DATA[idx], DATA_BROTLI[idx]);
    }
}

#[cfg(feature = "zstd")]
#[test]
fn should_decode_zstd() {
    let mut decoder = Interface::zstd(Default::default()).expect("create zstd decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut decoder, DATA[idx], DATA_ZSTD[idx]);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_decode_zlib_gzip() {
    let mut decoder = Interface::zlib(decoder::ZlibMode::Gzip).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut decoder, DATA[idx], DATA_GZIP[idx]);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_decode_zlib_ng_gzip() {
    let mut decoder = Interface::zlib(decoder::ZlibMode::Gzip).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut decoder, DATA[idx], DATA_GZIP[idx]);
    }
}
