//! Generates a file for each Godot class

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::{c_str, ident, ident_escaped, safe_ident, strlit, to_module_name};
use crate::{Context, RustTy, KNOWN_TYPES, SELECTED_CLASSES};

pub(crate) fn generate_class_files(
    api: &ExtensionApi,
    ctx: &Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    // TODO no limit after testing
    let mut modules = vec![];
    for class in api.classes.iter() {
        if !SELECTED_CLASSES.contains(&class.name.as_str()) {
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
    // TODO separate constructor singleton() for singletons
    let (new, new_attrs) = if class.is_refcounted {
        (ident("new"), TokenStream::new())
    } else {
        (ident("new_alloc"), quote! { #[must_use] })
    };
    let methods = make_methods(&class.methods, &class.name, ctx);

    let name_str = strlit(&class.name);
    let name_cstr = c_str(&class.name);
    let inherits_macro = format_ident!("gdext_inherits_transitive_{}", &class.name);

    let all_bases = ctx.inheritance_tree.map_all_bases(&class.name, ident);

    let memory = if &class.name == "Object" {
        ident("DynamicRefCount")
    } else if class.is_refcounted {
        ident("StaticRefCount")
    } else {
        ident("ManualMemory")
    };

    quote! {
        use gdext_sys as sys;
        use gdext_builtin::*;
        use crate::{Obj, AsArg};

        #[derive(Debug)]
        #[repr(transparent)]
        pub struct #name {
            object_ptr: sys::GDNativeObjectPtr,
        }
        impl #name {
            #new_attrs
            pub fn #new() -> Obj<Self> {
                unsafe {
                    let object_ptr = sys::interface_fn!(classdb_construct_object)(#name_cstr);
                    //let instance = Self { object_ptr };
                    Obj::from_obj_sys(object_ptr)
                }
            }
            #methods
        }
        impl crate::traits::GodotClass for #name {
            type Base = #base;
            type Declarer = crate::traits::dom::EngineDomain;
            type Mem = crate::traits::mem::#memory;

            const CLASS_NAME: &'static str = #name_str;
        }
        impl crate::traits::EngineClass for #name {
             fn as_object_ptr(&self) -> sys::GDNativeObjectPtr {
                 self.object_ptr
             }
             fn as_type_ptr(&self) -> sys::GDNativeTypePtr {
                std::ptr::addr_of!(self.object_ptr) as sys::GDNativeTypePtr
             }
        }
        #(
            impl crate::traits::Inherits<crate::api::#all_bases> for #name {}
        )*

        #[macro_export]
        #[allow(non_snake_case)]
        macro_rules! #inherits_macro {
            ($Class:ident) => {
                impl gdext_class::traits::Inherits<gdext_class::api::#name> for $Class {}
                #(
                    impl gdext_class::traits::Inherits<gdext_class::api::#all_bases> for $Class {}
                )*
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

    // -- FIXME remove when impl complete
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

fn is_function_excluded(function: &UtilityFunction) -> bool {
    function
        .return_type
        .as_ref()
        .map_or(false, |ret| !KNOWN_TYPES.contains(&ret.as_str()))
        || function.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| !KNOWN_TYPES.contains(&arg.type_.as_str()))
        })
}

fn make_method_definition(method: &Method, class_name: &str, ctx: &Context) -> TokenStream {
    if is_method_excluded(method) {
        return TokenStream::new();
    }

    let (params, arg_exprs) = make_params(&method.arguments, ctx);

    let method_name = safe_ident(&method.name);
    let c_method_name = c_str(&method.name);
    let c_class_name = c_str(class_name);
    let hash = method.hash;

    // TODO &mut safety
    let receiver = if method.is_const {
        quote!(&self)
    } else {
        quote!(&mut self)
    };

    let (return_decl, call) = make_method_return(&method.return_value, ctx);

    quote! {
        pub fn #method_name( #receiver, #( #params ),* ) #return_decl {
            let result = unsafe {
                let method_bind = sys::interface_fn!(classdb_get_method_bind)(#c_class_name, #c_method_name, #hash);
                let call_fn = sys::interface_fn!(object_method_bind_ptrcall);

                let args = [
                    #( #arg_exprs ),*
                ];
                let args_ptr = args.as_ptr();

                #call
            };

            result
        }
    }
}

pub(crate) fn make_function_definition(function: &UtilityFunction, ctx: &Context) -> TokenStream {
    if is_function_excluded(function) {
        return TokenStream::new();
    }

    let (params, arg_exprs) = make_params(&function.arguments, ctx);

    let function_name = safe_ident(&function.name);
    let c_function_name = c_str(&function.name);
    let hash = function.hash;

    let (return_decl, call) = make_utility_return(&function.return_type, ctx);

    quote! {
        pub fn #function_name( #( #params ),* ) #return_decl {
            let result = unsafe {
                let call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(#c_function_name, #hash);
                let call_fn = call_fn.unwrap_unchecked();

                let args = [
                    #( #arg_exprs ),*
                ];
                let args_ptr = args.as_ptr();

                #call
            };

            result
        }
    }
}

fn make_params(
    method_args: &Option<Vec<MethodArg>>,
    ctx: &Context,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let empty = vec![];
    let method_args = method_args.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut arg_exprs = vec![];
    for arg in method_args.iter() {
        let param_name = ident_escaped(&arg.name);
        let param = to_rust_type(&arg.type_, ctx);
        let param_ty = param.tokens;

        params.push(quote! { #param_name: #param_ty });
        if param.is_engine_class {
            arg_exprs.push(quote! {
                <#param_ty as AsArg>::as_arg_ptr(&#param_name)
            });
        } else {
            arg_exprs.push(quote! {
                <#param_ty as sys::GodotFfi>::sys(&#param_name)
            });
        }
    }
    (params, arg_exprs)
}

fn make_method_return(
    return_value: &Option<MethodReturn>,
    ctx: &Context,
) -> (TokenStream, TokenStream) {
    let return_decl;
    let call;
    match return_value {
        Some(ret) => {
            let return_ty = to_rust_type(&ret.type_, ctx).tokens;

            return_decl = quote! { -> #return_ty };
            call = quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init(|return_ptr| {
                    call_fn(method_bind, self.object_ptr, args_ptr, return_ptr);
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

fn make_utility_return(return_value: &Option<String>, ctx: &Context) -> (TokenStream, TokenStream) {
    let return_decl;
    let call;
    match return_value {
        Some(ret) => {
            let return_ty = to_rust_type(&ret, ctx).tokens;

            return_decl = quote! { -> #return_ty };
            call = quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init(|return_ptr| {
                    call_fn(return_ptr, args_ptr, args.len() as i32);
                })
            };
        }
        None => {
            return_decl = TokenStream::new();
            call = quote! {
                call_fn(std::ptr::null_mut(), args_ptr, args.len() as i32);
            };
        }
    }

    (return_decl, call)
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

    // Note: GodotFfi must be implemented for each of these types
    // Do not implement for non-canonical types which aren't used in Godot FFI APIs (like i16)
    // TODO double vs float
    let ty = match ty {
        "int" => "i64",
        "float" => "f64",
        "String" => "GodotString",
        other => other,
    };

    return RustTy {
        tokens: ident(ty).to_token_stream(),
        is_engine_class: false,
    };
}
