use miniserde::{json, Deserialize};
use quote::{format_ident, quote};
use std::path::Path;
use proc_macro2::TokenStream;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(Deserialize)]
struct ExtensionApi {
    builtin_class_sizes: Vec<ClassSizes>,
}

#[derive(Deserialize)]
struct ClassSizes {
    build_configuration: String,
    sizes: Vec<ClassSize>,
}

#[derive(Deserialize)]
struct ClassSize {
    name: String,
    size: usize,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

pub struct ApiParser {}

impl ApiParser {
    pub fn generate_file(gen_path: &Path) {
        let consts = Self::load_extension_api();
        let tokens = quote! {
            #[allow(non_upper_case_globals)]
            mod constants {
                #(#consts)*
            }
        };

        let string = tokens.to_string();

        let _ = std::fs::create_dir(gen_path);
        let out_path = gen_path.join("extensions.rs");
        std::fs::write(&out_path, string).expect("failed to write extension file");
        Self::format_file_if_needed(&out_path);
    }

    fn load_extension_api() -> Vec<TokenStream> {
        let build_config = "float_32"; // TODO infer this

        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/input/extension_api.json");
        let json = std::fs::read_to_string(path).expect(&format!("failed to open file {:?}", path));
        let model: ExtensionApi = json::from_str(&json).expect("failed to deserialize JSON");

        let mut result = vec![];

        for class in &model.builtin_class_sizes {
            if &class.build_configuration == build_config {
                for ClassSize { name, size } in &class.sizes {
                    let name = format_ident!("SIZE_{}", name);
                    result.push(quote! {
                        const #name: usize = #size;
                    });
                }
            }
        }

        result
    }

    //#[cfg(feature = "formatted")]
    fn format_file_if_needed(output_rs: &Path) {
        print!(
            "Formatting generated file: {}... ",
            output_rs.canonicalize().unwrap().as_os_str().to_str().unwrap()
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

        panic!("FORMATTERED!");
    }
}
