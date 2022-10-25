/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use bindgen::Builder;
use godot_codegen as gen;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // For custom path on macOS, iOS, Android etc: see gdnative-sys/build.rs

    let header_path = "../godot-codegen/input/gdnative_interface.h";
    println!("cargo:rerun-if-changed={}", header_path);

    let builder = bindgen::Builder::default()
        .header(header_path)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false);

    let bindings = configure_platform_specific(builder)
        .generate()
        .expect("unable to generate gdnative_interface.h bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);
    bindings
        .write_to_file(out_path.join("gdnative_interface.rs"))
        .expect("could not write gdnative_interface Rust bindings!");

    gen::generate_sys_files(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/sys"
        )),
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/godot-gen/core"
        )),
    );
}

//#[cfg(target_os = "macos")]
fn configure_platform_specific(builder: Builder) -> Builder {
    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
    if target_vendor == "apple" {
        eprintln!("Build selected for macOS.");
        let path = env::var("LLVM_PATH ").expect("env var 'LLVM_PATH' not set");

        builder
            .clang_arg("-I")
            .clang_arg(format!("{path}/include"))
            .clang_arg("-L")
            .clang_arg(format!("{path}/lib"))
    } else {
        eprintln!("Build selected for Linux/Windows.");
        builder
    }
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
