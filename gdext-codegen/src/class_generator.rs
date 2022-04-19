//! Generates a file for each Godot class

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::to_module_name;

// Workaround for limiting number of types as long as implementation is incomplete
const KNOWN_TYPES: [&str; 11] = [
    // builtin:
    "bool",
    "int",
    "float",
    "String",
    "Vector2",
    "Vector3",
    "Color",
    // classes:
    "Object",
    "Node",
    "Node3D",
    "RefCounted",
];

const SELECTED: [&str; 4] = ["Object", "Node", "Node3D", "RefCounted"];

#[derive(Default)]
struct Context<'a> {
    engine_classes: HashSet<&'a str>,
}

impl<'a> Context<'a> {
    fn is_engine_class(&self, class_name: &str) -> bool {
        self.engine_classes.contains(class_name)
    }
}

pub fn generate_class_files(
    api: &ExtensionApi,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let mut ctx = Context::default();
    for class in api.classes.iter() {
        if !SELECTED.contains(&class.name.as_str()) {
            continue;
        }

        println!("-- add engine class {}", class.name);
        ctx.engine_classes.insert(class.name.as_str());
    }

    // TODO no limit after testing
    let mut modules = vec![];
    for class in api.classes.iter() {
        if !SELECTED.contains(&class.name.as_str()) {
            continue;
        }

        let file_contents = make_class(class, &ctx).to_string();

        let module_name = to_module_name(&class.name);
        let out_path = gen_path.join(format!("{}.rs", module_name));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");

        let class_ident = ident(&class.name);
        let module_ident = ident(&module_name);
        modules.push((class_ident, module_ident));
        out_files.push(out_path);
    }

    let mod_contents = make_module_file(modules).to_string();
    let out_path = gen_path.join("mod.rs");
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

fn make_class(class: &Class, ctx: &Context) -> TokenStream {
    //let sys = TokenStream::from_str("::gdext_sys");
    let base = match class.inherits.as_ref() {
        Some(base) => {
            let base = ident(base);
            quote! { crate::api::#base }
        }
        None => quote! { () },
    };
    let name = ident(&class.name);
    let methods = make_methods(&class.methods, &class.name, ctx);

    let name_str = Literal::string(&class.name);

    quote! {
        use gdext_sys as sys;
        use gdext_builtin::*;
        use crate::{Obj, AsArg};

        #[derive(Debug)]
        //#[repr(C)]
        // TODO: un-pub field
        pub struct #name {
            pub object_ptr: sys::GDNativeObjectPtr,
        }

        impl #name {
            #methods
        }
        impl crate::traits::GodotClass for #name {
            const ENGINE_CLASS: bool = true;
            type Base = #base;
            fn class_name() -> String {
                #name_str.to_string()
            }
        }
        impl crate::traits::EngineClass for #name {
            fn as_object_ptr(&self) -> sys::GDNativeObjectPtr {
                self.object_ptr
            }
            fn as_type_ptr(&self) -> sys::GDNativeTypePtr {
                // TODO:mut
                &self.object_ptr as *const sys::GDNativeObjectPtr as sys::GDNativeTypePtr
            }
        }
    }
    // note: TypePtr -> ObjectPtr conversion OK?
}

fn make_module_file(classes_and_modules: Vec<(Ident, Ident)>) -> TokenStream {
    let decls = classes_and_modules.into_iter().map(|(class, module)| {
        let vis = TokenStream::new(); // TODO pub if other symbols
        quote! {
            #vis mod #module;
            pub use #module::#class;
        }
    });

    quote! {
        #( #decls )*
    }
}

fn make_methods(methods: &Option<Vec<Method>>, class_name: &str, ctx: &Context) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods
        .iter()
        .map(|method| make_method_definition(method, class_name, ctx));

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

    // FIXME remove when impl complete
    if method
        .return_value
        .as_ref()
        .map_or(false, |ret| !KNOWN_TYPES.contains(&ret.type_.as_str()))
        || method.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| !KNOWN_TYPES.contains(&arg.type_.as_str()))
        })
    {
        return true;
    }
    // -- end.

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

fn make_method_definition(method: &Method, class_name: &str, ctx: &Context) -> TokenStream {
    if is_method_excluded(method) {
        return TokenStream::new();
    }

    let empty = vec![];
    let method_args = method.arguments.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut call_exprs = vec![];
    for arg in method_args.iter() {
        let param_name = ident_escaped(&arg.name);
        let param = to_rust_type(&arg.type_, ctx);
        let param_ty = param.tokens;

        params.push(quote! { #param_name: #param_ty });
        if param.is_engine_class {
            call_exprs.push(quote! {
                <#param_ty as AsArg>::as_arg_ptr(&#param_name)
            });
        } else {
            call_exprs.push(quote! {
                <#param_ty as sys::PtrCall>::ptrcall_write_return(#param_name)
            });
        }
    }

    let method_name = ident(&method.name);
    let c_method_name = c_str(&method.name);
    let c_class_name = c_str(class_name);
    let hash = method.hash;

    let (return_decl, call) = make_return(&method.return_value, ctx);

    quote! {
        pub fn #method_name(&self, #(#params),* ) #return_decl {
            let result = unsafe {
                let method_bind = sys::interface_fn!(classdb_get_method_bind)(#c_class_name, #c_method_name, #hash);

                let call_fn = sys::interface_fn!(object_method_bind_ptrcall);

                let mut args = [
                    #(
                        #call_exprs
                    ),*
                ];
                let args_ptr = args.as_mut_ptr();

                #call
            };

            result
        }
    }
}

fn make_return(return_value: &Option<MethodReturn>, ctx: &Context) -> (TokenStream, TokenStream) {
    let return_decl;
    let call;
    match return_value {
        Some(ret) => {
            let return_ty = to_rust_type(&ret.type_, ctx).tokens;

            return_decl = quote! { -> #return_ty };
            call = quote! {
                <#return_ty as sys::PtrCall>::ptrcall_read_init(|ret_ptr| {
                    call_fn(method_bind, self.object_ptr, args_ptr, ret_ptr);
                })
            };
        }
        None => {
            return_decl = TokenStream::new();
            call = quote! {
                call_fn(method_bind, self.object_ptr, args_ptr, std::ptr::null_mut());
            };
        }
    }

    (return_decl, call)
}

fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

fn ident_escaped(s: &str) -> Ident {
    // note: could also use Ident::parse(s) from syn, but currently this crate doesn't depend on it

    let transformed = match s {
        "type" => "type_",
        s => s,
    };

    ident(transformed)
}

fn c_str(s: &str) -> TokenStream {
    let s = Literal::string(&format!("{}\0", s));
    quote! {
        #s.as_ptr() as *const i8
    }
}

fn to_rust_type(ty: &str, ctx: &Context) -> RustTy {
    //println!("to_rust_ty: {ty}");

    if let Some(remain) = ty.strip_prefix("enum::") {
        let mut parts = remain.split(".");

        let first = parts.next().unwrap();
        let ident = match parts.next() {
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
        return RustTy {
            tokens: ident.to_token_stream(),
            is_engine_class: false,
        };
    }

    if ctx.is_engine_class(ty) {
        let ty = ident(ty);
        return RustTy {
            tokens: quote! { Obj<#ty> },
            is_engine_class: true,
        };
    }

    let ty = match ty {
        "int" => "i32",
        "float" => "f32",          // TODO double vs float
        "String" => "GodotString", // TODO double vs float
        other => other,
    };

    return RustTy {
        tokens: ident(ty).to_token_stream(),
        is_engine_class: false,
    };
}

struct RustTy {
    tokens: TokenStream,
    is_engine_class: bool,
}
