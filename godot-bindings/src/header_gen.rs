/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::env;
use std::path::Path;

pub(crate) fn generate_rust_binding(in_h_path: &Path, out_rs_path: &Path) {
    let c_header_path = in_h_path.display().to_string();
    println!("cargo:rerun-if-changed={}", c_header_path);

    let builder = bindgen::Builder::default()
        .header(c_header_path)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false);

    std::fs::create_dir_all(
        out_rs_path
            .parent()
            .expect("bindgen output file has parent dir"),
    )
    .expect("create bindgen output dir");

    let bindings = configure_platform_specific(builder)
        .generate()
        .unwrap_or_else(|err| {
            panic!(
                "bindgen generate failed\n    c: {}\n   rs: {}\n  err: {}\n",
                in_h_path.display(),
                out_rs_path.display(),
                err
            )
        });

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings.write_to_file(out_rs_path).unwrap_or_else(|err| {
        panic!(
            "bindgen write failed\n    c: {}\n   rs: {}\n  err: {}\n",
            in_h_path.display(),
            out_rs_path.display(),
            err
        )
    });
}

//#[cfg(target_os = "macos")]
fn configure_platform_specific(builder: bindgen::Builder) -> bindgen::Builder {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
    if target_vendor == "apple" {
        eprintln!("Build selected for macOS.");
        let path = env::var("LLVM_PATH").expect("env var 'LLVM_PATH' not set");

        builder
            .clang_arg("-I")
            // .clang_arg(format!("{path}/include"))
            .clang_arg(apple_include_path().expect("apple include path"))
            .clang_arg("-L")
            .clang_arg(format!("{path}/lib"))
    } else {
        eprintln!("Build selected for Linux/Windows.");
        builder
    }
}

fn apple_include_path() -> Result<String, std::io::Error> {
    use std::process::Command;

    let target = std::env::var("TARGET").unwrap();
    let platform = if target.contains("apple-darwin") {
        "macosx"
    } else if target == "x86_64-apple-ios" || target == "aarch64-apple-ios-sim" {
        "iphonesimulator"
    } else if target == "aarch64-apple-ios" {
        "iphoneos"
    } else {
        panic!("not building for macOS or iOS");
    };

    // run `xcrun --sdk iphoneos --show-sdk-path`
    let output = Command::new("xcrun")
        .args(["--sdk", platform, "--show-sdk-path"])
        .output()?
        .stdout;
    let prefix = std::str::from_utf8(&output)
        .expect("invalid output from `xcrun`")
        .trim_end();

    let suffix = "usr/include";
    let directory = format!("{prefix}/{suffix}");

    Ok(directory)
}

// #[cfg(not(target_os = "macos"))]
// fn configure_platform_specific(builder: Builder) -> Builder {
//     println!("Build selected for Linux/Windows.");
//     builder
// }

/*fn rerun_if_any_changed(paths: &Vec<PathBuf>){
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}*/
