use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::path::Path;

macro_rules! push {
    ($inputs:ident; $ident:ident : $ty:ty = $val:expr) => {
        $inputs.push((stringify!($ident), quote! { $ty }, quote! { $val }));
    };
}

fn main() {
    let mut inputs = vec![];
    push!(inputs; int: i32 = 42);

    let methods = generate_methods(inputs);

    let tokens = quote::quote! {
        #[derive(gdext_macros::GodotClass)]
        #[godot(init)]
        struct RustFfi {}

        #[gdext_macros::godot_api]
        impl RustFfi {
            #(#methods)*
        }
    };

    let output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));
    let file_path = output_dir.join("rust_ffi.rs");

    //println!("Output path: {}", file_path.display());
    std::fs::create_dir_all(output_dir).expect("create parent dir");
    std::fs::write(file_path, tokens.to_string()).expect("write to rust_ffi.rs");
}

fn generate_methods(inputs: Vec<(&str, TokenStream, TokenStream)>) -> Vec<TokenStream> {
    inputs
        .into_iter()
        .map(|(ident, ty, val)| {
            let return_method = format_ident!("return_{}", ident);
            let accept_method = format_ident!("accept_{}", ident);
            let mirror_method = format_ident!("mirror_{}", ident);

            quote! {
                #[godot]
                fn #return_method(&self) -> #ty {
                    #val
                }

                #[godot]
                fn #accept_method(&self, i: #ty) -> bool {
                    i == #val
                }

                #[godot]
                fn #mirror_method(&self, i: #ty) -> #ty {
                    i
                }
            }
        })
        .collect()
}
