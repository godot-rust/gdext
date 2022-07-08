mod api_parser;
mod central_generator;
mod class_generator;
mod godot_exe;
mod godot_version;
mod util;

use api_parser::load_extension_api;
use central_generator::generate_central_file;
use class_generator::generate_class_files;
use std::env;
use std::path::{Path, PathBuf};

// macro_rules! local_path {
//     ($path:lit) => {
//         Path::new(concat!(env!("CARGO_MANIFEST_DIR"), $path))
//     };
// }

pub fn generate() {
    let sys_gen_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../gdext-sys/src/gen"));
    let class_gen_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../gdext-class/src/gen"
    ));

    let mut out_files = vec![];

    let now = std::time::Instant::now();
    let (api, build_config) = load_extension_api();
    let load_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    generate_central_file(&api, build_config, sys_gen_path, &mut out_files);
    let central_time = now.elapsed().as_millis();

    // Class files -- currently output in gdext-class; could maybe be separated cleaner
    let now = std::time::Instant::now();
    // Note: deletes entire generated directory!
    generate_class_files(
        &api,
        build_config,
        &class_gen_path.join("classes"),
        &mut out_files,
    );
    let class_time = now.elapsed().as_millis();

    let now = std::time::Instant::now();
    rustfmt_if_needed(out_files);
    let fmt_time = now.elapsed().as_millis();

    println!("Times [ms]:");
    println!("  load-json:     {load_time}");
    println!("  gen-central:   {central_time}");
    println!("  gen-class:     {class_time}");
    println!("  fmt:           {fmt_time}");
}

//#[cfg(feature = "formatted")]
fn rustfmt_if_needed(out_files: Vec<PathBuf>) {
    //print!("Format {} generated files...", out_files.len());

    let mut process = std::process::Command::new("rustup");
    process
        .arg("run")
        .arg("stable")
        .arg("rustfmt")
        .arg("--edition=2021");

    for file in out_files {
        //println!("Format {file:?}");
        process.arg(file);
    }

    match process.output() {
        Ok(_) => println!("Done."),
        Err(err) => {
            println!("Failed.");
            println!("Error: {}", err);
        }
    }
}
