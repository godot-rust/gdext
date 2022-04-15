mod api_parser;
mod central_generator;
mod class_generator;
mod godot_exe;
mod util;

pub use api_parser::load_extension_api;
pub use central_generator::generate_central_file;
pub use class_generator::generate_class_files;

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

//#[cfg(feature = "formatted")]
pub fn rustfmt_if_needed(out_files: Vec<PathBuf>) {
    print!("Format {} generated files...", out_files.len());

    let mut process = std::process::Command::new("rustup");
    process
        .arg("run")
        .arg("stable")
        .arg("rustfmt")
        .arg("--edition=2021");

    for file in out_files {
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
