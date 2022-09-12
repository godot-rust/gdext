use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::io::Write;
use std::path::Path;

macro_rules! push {
    ($inputs:ident; $ident:ident : $ty:ty = $val:expr; $gd_ty:ident) => {
        $inputs.push(Input {
            ident: stringify!($ident),
            rust_ty: quote! { $ty },
            rust_val: quote! { $val },
            gdscript_ty: stringify!($gd_ty),
        });
    };
}

fn main() {
    let mut inputs = vec![];
    push!(inputs; int: i32 = 42; int);
    push!(inputs; bool: bool = true; bool);

    let methods = generate_rust_methods(&inputs);

    let rust_tokens = quote::quote! {
        #[derive(gdext_macros::GodotClass)]
        #[godot(init)]
        struct RustFfi {}

        #[gdext_macros::godot_api]
        impl RustFfi {
            #(#methods)*
        }
    };

    let rust_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));
    let godot_input_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/input"));
    let godot_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/gen"));

    let rust_file = rust_output_dir.join("rust_ffi.rs");
    let gdscript_template = godot_input_dir.join("FfiTestsTemplate.gd");
    let gdscript_file = godot_output_dir.join("FfiTests.gd");

    std::fs::create_dir_all(rust_output_dir).expect("create Rust parent dir");
    std::fs::create_dir_all(godot_output_dir).expect("create GDScript parent dir");
    std::fs::write(rust_file, rust_tokens.to_string()).expect("write to Rust file");
    write_gdscript_code(&inputs, &gdscript_template, &gdscript_file)
        .expect("write to GDScript file");
}

struct Input {
    ident: &'static str,
    rust_ty: TokenStream,
    rust_val: TokenStream,
    gdscript_ty: &'static str,
}

fn generate_rust_methods(inputs: &Vec<Input>) -> Vec<TokenStream> {
    inputs
        .iter()
        .map(|input| {
            let Input {
                ident,
                rust_ty,
                rust_val,
                ..
            } = input;

            let return_method = format_ident!("return_{}", ident);
            let accept_method = format_ident!("accept_{}", ident);
            let mirror_method = format_ident!("mirror_{}", ident);

            quote! {
                #[godot]
                fn #return_method(&self) -> #rust_ty {
                    #rust_val
                }

                #[godot]
                fn #accept_method(&self, i: #rust_ty) -> bool {
                    i == #rust_val
                }

                #[godot]
                fn #mirror_method(&self, i: #rust_ty) -> #rust_ty {
                    i
                }
            }
        })
        .collect()
}

fn write_gdscript_code(
    inputs: &Vec<Input>,
    in_template_path: &Path,
    out_file_path: &Path,
) -> std::io::Result<()> {
    let template = std::fs::read_to_string(in_template_path)?;
    let mut file = std::fs::File::create(out_file_path)?;

    for input in inputs {
        let Input {
            ident,
            gdscript_ty,
            rust_val,
            ..
        } = input;

        let replaced = template
            .replace("IDENT", ident)
            .replace("TYPE", gdscript_ty)
            .replace("VAL", &rust_val.to_string());

        file.write_all(replaced.as_bytes())?;
        file.write_all("\n\n".as_bytes())?;
    }

    Ok(())
}
