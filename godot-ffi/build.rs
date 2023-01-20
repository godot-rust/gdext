/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::env;
use std::path::Path;

fn main() {
    // For custom path on macOS, iOS, Android etc: see gdnative-sys/build.rs
    let gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen/"));

    if gen_path.exists() {
        std::fs::remove_dir_all(gen_path).unwrap_or_else(|e| panic!("failed to delete dir: {e}"));
    }

    run_bindgen(&gen_path.join("gdextension_interface.rs"));

    let stubs_only = cfg!(gdext_test);
    godot_codegen::generate_sys_files(gen_path, stubs_only);
}

fn run_bindgen(out_file: &Path) {
    let header_path = "../godot-codegen/input/gdextension_interface.h";
    println!("cargo:rerun-if-changed={}", header_path);

    let builder = bindgen::Builder::default()
        .header(header_path)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false);

    std::fs::create_dir_all(
        out_file
            .parent()
            .expect("bindgen output file has parent dir"),
    )
    .expect("create bindgen output dir");

    let bindings = configure_platform_specific(builder)
        .generate()
        .expect("failed generate gdextension_interface.h bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_file)
        .expect("failed write gdextension_interface.h bindings to file");
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
    let directory = format!("{}/{}", prefix, suffix);

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
