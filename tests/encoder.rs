use compu::{encoder, decoder};
use encoder::{Encoder, Interface, EncodeStatus, EncodeOp};
use decoder::{Decoder, DecodeStatus, Detection};

const DATA: [&[u8]; 2] = [
    include_bytes!("data/10x10y"),
    include_bytes!("data/alice29.txt")
];

fn test_case(idx: usize, encoder: &mut Encoder, decoder: &mut Decoder, data: &[u8], expected_detection: Detection) {
    println!("{idx}: DATA.len()={}", data.len());

    let mut compressed = vec![0; data.len()];
    let mut decompressed = vec![0; data.len()];
    let result = encoder.encode(data, compressed.as_mut(), EncodeOp::Finish);
    assert_eq!(result.input_remain, 0);

    if result.status == EncodeStatus::NeedOutput {
        //Some formats have overhead on small data that makes compression pointless (due to header size)
        //so allocate more space and finalize it
        compressed.reserve(100);
        let spare_capacity = compressed.spare_capacity_mut();
        let spare_capacity_len = spare_capacity.len();
        let result = encoder.encode_uninit(&[], spare_capacity, EncodeOp::Finish);
        assert_eq!(result.status, EncodeStatus::Finished);
        unsafe {
            compressed.set_len(compressed.len() + spare_capacity_len - result.output_remain);
        }
    } else {
        compressed.truncate(compressed.len() - result.output_remain);
    }

    assert_eq!(Detection::detect(&compressed), Some(expected_detection));
    let result = decoder.decode(&compressed, decompressed.as_mut());
    assert_eq!(result.status, Ok(DecodeStatus::Finished));
    assert_eq!(data, decompressed);

    encoder.reset();
    decoder.reset();
}

#[cfg(feature = "bytes")]
fn test_case_bytes(idx: usize, encoder: &mut Encoder, decoder: &mut Decoder, data: &[u8], expected_detection: Detection) {
    use bytes::BufMut;
    println!("{idx}: DATA.len()={}", data.len());

    let mut compressed = bytes::BytesMut::from(b"test".as_slice());
    compressed.truncate(0);

    let mut decompressed = bytes::BytesMut::new();
    decompressed.reserve(data.len());
    let result = encoder.encode_buf(data, &mut compressed, EncodeOp::Finish);

    let expected_remain_output = compressed.remaining_mut();
    assert_eq!(result.input_remain, 0);
    assert_eq!(result.output_remain, expected_remain_output);

    if result.status == EncodeStatus::NeedOutput {
        //Some formats have overhead on small data that makes compression pointless (due to header size)
        //so allocate more space and finalize it
        compressed.reserve(100);
        let result = encoder.encode_buf(&[], &mut compressed, EncodeOp::Finish);
        assert_eq!(result.status, EncodeStatus::Finished);
    }

    let compressed = compressed.freeze();

    assert_eq!(Detection::detect(&compressed), Some(expected_detection));
    let result = decoder.decode_buf(&compressed, &mut decompressed);
    assert_eq!(result.status, Ok(DecodeStatus::Finished));
    assert_eq!(data, decompressed);

    encoder.reset();
    decoder.reset();
}

fn test_case_empty_final(idx: usize, encoder: &mut Encoder, decoder: &mut Decoder, data: &[u8]) {
    println!("{idx}: DATA.len()={}", data.len());

    let mut compressed = vec![0; data.len()];
    let mut decompressed = vec![0; data.len() + 100];
    compressed.truncate(0);

    let mut output = compressed.spare_capacity_mut();
    let mut output_len = output.len();
    let result = encoder.encode_uninit(data, output, EncodeOp::Process);
    assert_ne!(result.status, EncodeStatus::Error);
    unsafe {
        compressed.set_len(output_len - result.output_remain);
    }

    output = compressed.spare_capacity_mut();
    output_len = output.len();
    let result = encoder.encode_uninit(&data[data.len() - result.input_remain..], output, EncodeOp::Flush);
    assert_eq!(result.input_remain, 0);
    assert_eq!(result.status, EncodeStatus::Continue);
    unsafe {
        compressed.set_len(compressed.len() + output_len - result.output_remain);
    }

    compressed.reserve(100);
    output = compressed.spare_capacity_mut();
    output_len = output.len();
    let result = encoder.encode_uninit(&[], output, EncodeOp::Finish);
    assert_eq!(result.status, EncodeStatus::Finished);
    unsafe {
        compressed.set_len(compressed.len() + output_len - result.output_remain);
    }

    decompressed.truncate(0);
    for (idx, chunk) in compressed.chunks(compressed.len() / 4).enumerate() {
        println!("compressed(idx={idx}) with len={}", chunk.len());
        let current_len = decompressed.len();
        let output = decompressed.spare_capacity_mut();
        let output_len = output.len();

        let result = decoder.decode_uninit(chunk, output);
        assert_eq!(result.input_remain, 0);
        assert!(result.output_remain > 0);

        unsafe {
            decompressed.set_len(current_len + output_len - result.output_remain)
        }
        let status = result.status.expect("to decode");
        if status == DecodeStatus::Finished {
            break;
        } else {
            assert_eq!(status, DecodeStatus::NeedInput);
        }
    }
    assert_eq!(data, decompressed);

    encoder.reset();
    decoder.reset();
}

#[cfg(feature = "brotli-c")]
#[test]
fn should_encode_and_decode_brotli_c() {
    let mut encoder = Interface::brotli_c(Default::default()).expect("create brotli encoder");
    let mut decoder = decoder::Interface::brotli_c().expect("create brotli decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(feature = "brotli-rust")]
#[test]
fn should_encode_and_decode_brotli_rust() {
    let mut encoder = Interface::brotli_rust(Default::default());
    let mut decoder = decoder::Interface::brotli_rust();
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(feature = "zstd")]
#[test]
fn should_encode_and_decode_zstd() {
    let mut encoder = Interface::zstd(Default::default()).expect("create zstd encoder");
    let mut decoder = decoder::Interface::zstd(Default::default()).expect("create zstd decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zstd);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_and_decode_zlib_gzip() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Gzip).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Gzip);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_and_decode_zlib_ng_gzip() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Gzip).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Gzip);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_and_decode_zlib() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Zlib).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zlib);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_and_decode_zlib_ng() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Zlib).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zlib);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_and_decode_zlib_deflate() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Deflate).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_and_decode_zlib_ng_deflate() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Deflate).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(feature = "brotli-c")]
#[test]
fn should_encode_with_empty_final_and_decode_brotli_c() {
    let mut encoder = Interface::brotli_c(Default::default()).expect("create brotli encoder");
    let mut decoder = decoder::Interface::brotli_c().expect("create brotli decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(feature = "brotli-rust")]
#[test]
fn should_encode_with_empty_final_and_decode_brotli_rust() {
    let mut encoder = Interface::brotli_rust(Default::default());
    let mut decoder = decoder::Interface::brotli_rust();
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(feature = "zstd")]
#[test]
fn should_encode_with_empty_final_and_decode_zstd() {
    let mut encoder = Interface::zstd(Default::default()).expect("create zstd encoder");
    let mut decoder = decoder::Interface::zstd(Default::default()).expect("create zstd decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_with_empty_final_and_decode_zlib_gzip() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Gzip).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_with_empty_final_and_decode_zlib_ng_gzip() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Gzip).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_with_empty_final_and_decode_zlib() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Zlib).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_with_empty_final_and_decode_zlib_ng() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Zlib).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(any(feature = "zlib", feature = "zlib-static"))]
#[test]
fn should_encode_with_empty_final_and_decode_zlib_deflate() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Deflate).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(feature = "zlib-ng")]
#[test]
fn should_encode_with_empty_final_and_decode_zlib_ng_deflate() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Deflate).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_empty_final(idx, &mut encoder, &mut decoder, DATA[idx]);
    }
}

#[cfg(all(feature = "brotli-c", feature = "bytes"))]
#[test]
fn should_encode_and_decode_brotli_c_bytes() {
    let mut encoder = Interface::brotli_c(Default::default()).expect("create brotli encoder");
    let mut decoder = decoder::Interface::brotli_c().expect("create brotli decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(all(feature = "brotli-rust", feature = "bytes"))]
#[test]
fn should_encode_and_decode_brotli_rust_bytes() {
    let mut encoder = Interface::brotli_rust(Default::default());
    let mut decoder = decoder::Interface::brotli_rust();
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(all(feature = "zstd", feature = "bytes"))]
#[test]
fn should_encode_and_decode_zstd_bytes() {
    let mut encoder = Interface::zstd(Default::default()).expect("create zstd encoder");
    let mut decoder = decoder::Interface::zstd(Default::default()).expect("create zstd decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zstd);
    }
}

#[cfg(all(any(feature = "zlib", feature = "zlib-static"), feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_gzip_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Gzip).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Gzip);
    }
}

#[cfg(all(feature = "zlib-ng", feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_ng_gzip_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Gzip);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Gzip).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Gzip);
    }
}

#[cfg(all(any(feature = "zlib", feature = "zlib-static"), feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Zlib).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zlib);
    }
}

#[cfg(all(feature = "zlib-ng", feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_ng_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Zlib);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Zlib).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Zlib);
    }
}

#[cfg(all(any(feature = "zlib", feature = "zlib-static"), feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_deflate_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib(options).expect("create zlib encoder");
    let mut decoder = decoder::Interface::zlib(decoder::ZlibMode::Deflate).expect("create zlib decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}

#[cfg(all(feature = "zlib-ng", feature = "bytes"))]
#[test]
fn should_encode_and_decode_zlib_ng_deflate_bytes() {
    let options = encoder::ZlibOptions::new().mode(encoder::ZlibMode::Deflate);
    let mut encoder = Interface::zlib_ng(options).expect("create zlib-ng encoder");
    let mut decoder = decoder::Interface::zlib_ng(decoder::ZlibMode::Deflate).expect("create zlib-ng decoder");
    for idx in 0..DATA.len() {
        test_case_bytes(idx, &mut encoder, &mut decoder, DATA[idx], Detection::Unknown);
    }
}
