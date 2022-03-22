use miniserde::{json, Deserialize};
use quote::quote;
use std::path::Path;

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
    pub fn generate_file() {
        let tokens = quote! {
            struct Constants {

            }
        };

        Self::load_extension_api();

        let string = tokens.to_string();

        let _ = std::fs::create_dir("src/gen");
        std::fs::write("src/gen/extensions.rs", string).expect("failed to write extension file");
    }

    fn load_extension_api() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/input/extension_api.json");

        let build_config = "float_32"; // TODO infer this

        let json = std::fs::read_to_string(path).expect(&format!("failed to open file {:?}", path));
        println!("JSON: {}", &json[0..30]);
        let model: ExtensionApi = json::from_str(&json).expect("failed to deserialize JSON");
    }
}
