fn fetch_brotli_if() {
    use std::process::Command;
    use std::fs;

    println!("Checking brotli source code...");
    let is_to_pull = Command::new("git").arg("status").current_dir(BROTLI_DIR).output().map(|output| !output.status.success()).unwrap_or(true);

    if !is_to_pull {
        println!("Already downloaded.");
        return;
    }

    println!("Downloading brotli source code...");

    let _ = fs::remove_dir_all(BROTLI_DIR);
    fs::create_dir(BROTLI_DIR).expect("To create dir");

    let res = Command::new("git").arg("clone")
                                 .arg("git@github.com:google/brotli.git")
                                 .arg("--branch")
                                 .arg("v1.0")
                                 .arg("--single-branch")
                                 .status()
                                 .expect("To execute sh command");

    if !res.success() {
        panic!("Failed to configure libopus");
    }

}

#[cfg(feature = "build-bindgen")]
extern crate bindgen;

const BROTLI_DIR: &'static str = "brotli";

#[cfg(feature = "build-bindgen")]
fn generate_lib() {
    println!("Generates bindings...");
    let include_path = format!("{}/c/include", BROTLI_DIR);

    #[derive(Debug)]
    struct ParseCallbacks;

    impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
        fn int_macro(&self, name: &str, _value: i64) -> Option<bindgen::callbacks::IntKind> {
            if name.starts_with("BROTLI") {
                Some(bindgen::callbacks::IntKind::Int)
            } else {
                None
            }
        }
    }

    use std::path::PathBuf;

    const PREPEND_LIB: &'static str = "
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
";

    let out = PathBuf::new().join("src").join("lib.rs");

    let bindings = bindgen::Builder::default().header(format!("{}/brotli/encode.h", include_path))
                                              .header(format!("{}/brotli/decode.h", include_path))
                                              .raw_line(PREPEND_LIB)
                                              .clang_arg(format!("-I{}", include_path))
                                              .parse_callbacks(Box::new(ParseCallbacks))
                                              .generate()
                                              .expect("Unable to generate bindings");

    bindings.write_to_file(out).expect("Couldn't write bindings!");

}

#[cfg(not(feature = "build-bindgen"))]
fn generate_lib() {
}

fn build() {
    let abs_include = std::fs::canonicalize("brotli/c/include/").expect("To get absolute path to brotlie include");
    println!("cargo:include={}", abs_include.display());

    cc::Build::new().include("brotli/c/include")
                    .warnings(false)
                    .file("brotli/c/common/dictionary.c")
                    .file("brotli/c/common/transform.c")
                    .file("brotli/c/dec/bit_reader.c")
                    .file("brotli/c/dec/decode.c")
                    .file("brotli/c/dec/huffman.c")
                    .file("brotli/c/dec/state.c")
                    .file("brotli/c/enc/backward_references.c")
                    .file("brotli/c/enc/backward_references_hq.c")
                    .file("brotli/c/enc/bit_cost.c")
                    .file("brotli/c/enc/block_splitter.c")
                    .file("brotli/c/enc/brotli_bit_stream.c")
                    .file("brotli/c/enc/cluster.c")
                    .file("brotli/c/enc/compress_fragment.c")
                    .file("brotli/c/enc/compress_fragment_two_pass.c")
                    .file("brotli/c/enc/dictionary_hash.c")
                    .file("brotli/c/enc/encoder_dict.c")
                    .file("brotli/c/enc/encode.c")
                    .file("brotli/c/enc/entropy_encode.c")
                    .file("brotli/c/enc/histogram.c")
                    .file("brotli/c/enc/literal_cost.c")
                    .file("brotli/c/enc/memory.c")
                    .file("brotli/c/enc/metablock.c")
                    .file("brotli/c/enc/static_dict.c")
                    .file("brotli/c/enc/utf8_util.c")
                    .compile("libbrotli.a");
}


fn main() {
    fetch_brotli_if();
    generate_lib();
    build();
}
