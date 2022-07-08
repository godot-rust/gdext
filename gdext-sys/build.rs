use gdext_codegen as gen;
use std::env;
use std::path::PathBuf;

fn main() {
    let header_path = "../gdext-codegen/input/gdnative_interface.h";
    println!("cargo:rerun-if-changed={}", header_path);

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

    gen::generate();
}

/*fn rerun_if_any_changed(paths: &Vec<PathBuf>){
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}*/
