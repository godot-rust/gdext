mod api_parser;
mod central_generator;
mod godot_exe;

pub use api_parser::load_extension_api;
pub use central_generator::generate_central_file;

use std::path::Path;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

//#[cfg(feature = "formatted")]
pub(crate) fn format_file_if_needed(output_rs: &Path) {
    print!(
        "Formatting generated file: {}... ",
        output_rs
            .canonicalize()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
    );

    let output = std::process::Command::new("rustup")
        .arg("run")
        .arg("stable")
        .arg("rustfmt")
        .arg("--edition=2021")
        .arg(output_rs)
        .output();

    match output {
        Ok(_) => println!("Done."),
        Err(err) => {
            println!("Failed.");
            println!("Error: {}", err);
        }
    }
}
