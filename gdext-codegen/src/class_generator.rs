//! Generates a file for each Godot class

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::path::Path;
use std::str::FromStr;

use crate::api_parser::*;

pub fn generate_class_files(api: &ExtensionApi, _build_config: &str, gen_path: &Path) {
    let _ = std::fs::create_dir(gen_path);

    // TODO no limit after testing
    for class in api.classes.iter().take(10) {
        let tokens = make_class(class);
        let string = tokens.to_string();

        let out_path = gen_path.join(format!("{}.rs", class.name));
        std::fs::write(&out_path, string).expect("failed to write extension file");

        crate::format_file_if_needed(&out_path);
    }
}

fn make_class(class: &Class) -> TokenStream {
    //let sys = TokenStream::from_str("::gdext_sys");
    let name = ident(&class.name);
    let methods = make_methods(&class.methods);

    quote! {
        pub struct #name {
            sys: ::gdext_sys::GDNativeObjectPtr,
        }

        impl #name {
            #methods
        }
    }
}

fn make_methods(methods: &Option<Vec<Method>>) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods.iter().map(|method| -> TokenStream {
        let empty = vec![];
        let args = method.arguments.as_ref().unwrap_or(&empty);

        let args = args.iter().map(|arg| {
            let name = ident(&arg.name);
            let rust_ty = to_rust_type(&arg.type_);
            quote! { #name: #rust_ty }
        });

        let fn_name = ident(&method.name);
        quote! {
            pub fn #fn_name( #(#args),* ) {

            }
        }
    });

    quote! {
        #( #definitions )*
    }
}

fn to_rust_type(ty: &str) -> Ident {
    if let Some(remain) = ty.strip_prefix("enum::") {
        let mut parts = remain.split(".");
        let class_name = parts.next().unwrap();
        let enum_name = parts.next().unwrap();
        assert!(parts.next().is_none());

        format_ident!("{}{}", class_name, enum_name) // TODO better
    } else {
        ident( ty)
    }
}

fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}