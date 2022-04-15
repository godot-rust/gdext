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

    let mut out_files = vec![];

    let now = std::time::Instant::now();
    let (api, build_config) = gen::load_extension_api();
    let load_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    gen::generate_central_file(&api, build_config, gen_path, &mut out_files);
    let central_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    // Note: deletes entire gen_path directory!
    gen::generate_class_files(
        &api,
        build_config,
        &gen_path.join("classes"),
        &mut out_files,
    );
    let class_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    gen::rustfmt_if_needed(out_files);
    let fmt_time = now.elapsed().as_millis();

    println!("cargo:rerun-if-changed={}", header_path);
    println!("Times [ms]:");
    println!("  load-json:     {load_time}");
    println!("  gen-central:   {central_time}");
    println!("  gen-class:     {class_time}");
    println!("  fmt:           {fmt_time}");
    //panic!("Just to output timing")
}
