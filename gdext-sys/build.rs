use gdext_codegen as gen;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let header_path = "../thirdparty/godot-headers/godot/gdnative_interface.h";
    let bindings = bindgen::Builder::default()
        .header(header_path)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("unable to generate gdnative_interface.h bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("gdnative_interface.rs"))
        .expect("could not write gdnative_interface Rust bindings!");

    let gen_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen");
    let gen_path = Path::new(gen_path);

    let (api, build_config) = gen::load_extension_api();
    gen::generate_central_file(&api, build_config, gen_path);
    gen::generate_class_files(&api, build_config, &gen_path.join("classes"));

    println!("cargo:rerun-if-changed={}", header_path);
}
