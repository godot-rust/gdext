//! Generates a file for each Godot class

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::path::{Path, PathBuf};

use crate::api_parser::*;

pub fn generate_class_files(
    api: &ExtensionApi,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::create_dir(gen_path);

    // TODO no limit after testing
    for class in api.classes.iter().take(10) {
        let tokens = make_class(class);
        let string = tokens.to_string();

        let out_path = gen_path.join(format!("{}.rs", class.name));
        std::fs::write(&out_path, string).expect("failed to write extension file");

        out_files.push(out_path);
    }
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

fn make_methods(methods: &Option<Vec<Method>>, class_name: &str) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods.iter().map(|method| -> TokenStream {
        let empty = vec![];
        let method_args = method.arguments.as_ref().unwrap_or(&empty);

        let mut params=vec![];
        let mut args =vec![];
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
    });

    quote! {
        #( #definitions )*
    }
}

fn make_call(return_value: &Option<MethodReturn>) -> TokenStream {
    match return_value {
        Some(ret) => {
            let return_ty = to_rust_type(&ret.type_).to_token_stream();

            quote! {
                #return_ty::from_sys_init(|opaque_ptr| {
                    call_fn(method_bind, self.sys, args_ptr, opaque_ptr);
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

fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

fn c_str(s: &str) -> Literal {
    Literal::string(&format!("{}\0", s))
}
