//! Generates a file for each Godot class

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::path::{Path, PathBuf};
use heck::ToSnakeCase as _;

use crate::api_parser::*;

pub fn generate_class_files(
    api: &ExtensionApi,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir(gen_path).expect("create classes directory");

    // TODO no limit after testing
    let selected = ["Object", "Node", "Node3D", "RefCounted"];
    let mut modules = vec![];
    for class in api.classes.iter() {
        if !selected.contains(&class.name.as_str()) {
            continue;
        }

        let file_contents = make_class(class).to_string();

        let module_name = to_module_name(class);
        let out_path = gen_path.join(format!("{}.rs", module_name));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");

        modules.push(ident(&module_name));
        out_files.push(out_path);
    }

    let mod_contents = make_module_file(modules).to_string();
    let out_path =  gen_path.join("mod.rs");
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

fn to_module_name(class: &Class) -> String {
    class.name.to_snake_case()
}

fn make_class(class: &Class) -> TokenStream {
    //let sys = TokenStream::from_str("::gdext_sys");
    let name = ident(&class.name);
    let methods = make_methods(&class.methods, &class.name);

    quote! {
        pub struct #name {
            sys: ::gdext_sys::GDNativeObjectPtr,
        }

        impl #name {
            #methods
        }
    }
}

fn make_module_file(modules: Vec<Ident>) -> TokenStream {
    quote! {
        #( pub mod #modules; )*
    }
}

fn make_methods(methods: &Option<Vec<Method>>, class_name: &str) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods
        .iter()
        .map(|method| make_method_definition(method, class_name));

    quote! {
        #( #definitions )*
    }
}

fn is_method_excluded(method: &Method) -> bool {
    // Currently excluded:
    //
    // * Private virtual methods designed for override; skip for now
    //   E.g.: AudioEffectInstance::_process(const void*, AudioFrame*, int)
    //   TODO decide what to do with them, overriding in a type-safe way?
    //
    // * Methods accepting pointers are often supplementary
    //   E.g.: TextServer::font_set_data_ptr() -- in addition to TextServer::font_set_data().
    //   These are anyway not accessible in GDScript since that language has no pointers.
    //   As such support could be added later (if at all), with possibly safe interfaces (e.g. Vec for void*+size pairs)

    method.name.starts_with("_")
        || method
            .return_value
            .as_ref()
            .map_or(false, |ret| ret.type_.contains("*"))
        || method
            .arguments
            .as_ref()
            .map_or(false, |args| args.iter().any(|arg| arg.type_.contains("*")))
}

fn make_method_definition(method: &Method, class_name: &str) -> TokenStream {
    if is_method_excluded(method) {
        return TokenStream::new();
    }

    let empty = vec![];
    let method_args = method.arguments.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut args = vec![];
    for arg in method_args.iter() {
        let param_name = ident(&arg.name);
        let param_ty = to_rust_type(&arg.type_);
        params.push(quote! { #param_name: #param_ty });
        args.push(param_name);
    }

    let method_name = ident(&method.name);
    let c_method_name = c_str(&method.name);
    let c_class_name = c_str(class_name);
    let hash = method.hash;
    let call = make_call(&method.return_value);

    quote! {
        pub fn #method_name(&self, #(#params),* ) {
            let result = unsafe {
                let method_bind = interface_fn!(classdb_get_method_bind)(#c_class_name, #c_method_name, #hash);

                let call_fn = interface_fn!(object_method_bind_ptrcall);

                let mut args = [
                    #(
                        #args.sys()
                    ),*
                ];
                let args_ptr = args.as_mut_ptr();

                #call
            };

            result
        }
    }
}

fn make_call(return_value: &Option<MethodReturn>) -> TokenStream {
    match return_value {
        Some(ret) => {
            let return_ty = to_rust_type(&ret.type_).to_token_stream();

            quote! {
                <#return_ty as ::gdext_sys::PtrCall>::ptrcall_read_init(|ret_ptr| {
                    call_fn(method_bind, self.sys, args_ptr, ret_ptr);
                })
            }
        }
        None => {
            quote! {
                call_fn(method_bind, self.sys, args_ptr, std::ptr::null_mut());
            }
        }
    }
}

fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

fn c_str(s: &str) -> Literal {
    Literal::string(&format!("{}\0", s))
}

fn to_rust_type(ty: &str) -> Ident {
    //println!("to_rust_ty: {ty}");

    if let Some(remain) = ty.strip_prefix("enum::") {
        let mut parts = remain.split(".");

        let first = parts.next().unwrap();
        let result = match parts.next() {
            Some(second) => {
                // enum::Animation.LoopMode
                format_ident!("{}{}", first, second) // TODO better
            }
            None => {
                // enum::Error
                format_ident!("{}", first)
            }
        };

        assert!(parts.next().is_none(), "Unrecognized enum type '{}'", ty);
        result
    } else {
        ident(ty)
    }
}
