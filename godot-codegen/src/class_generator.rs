/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates a file for each Godot class

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::{ident, safe_ident, strlit, to_module_name, to_rust_type};
use crate::{special_cases, util, Context, GeneratedClass, GeneratedModule, RustTy};

pub(crate) fn generate_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let mut modules = vec![];
    for class in api.classes.iter() {
        #[cfg(not(feature = "codegen-full"))]
        if !crate::SELECTED_CLASSES.contains(&class.name.as_str()) {
            continue;
        }

        if special_cases::is_class_deleted(class.name.as_str()) {
            continue;
        }

        let generated_class = make_class(class, ctx);
        let file_contents = generated_class.tokens.to_string();

        let module_name = to_module_name(&class.name);
        let out_path = gen_path.join(format!("{module_name}.rs"));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");
        out_files.push(out_path);

        let class_ident = ident(&class.name);
        let module_ident = ident(&module_name);
        modules.push(GeneratedModule {
            class_ident,
            module_ident,
            inherits_macro_ident: generated_class.inherits_macro_ident,
            is_pub: generated_class.has_pub_module,
        });
    }

    let mod_contents = make_module_file(modules).to_string();
    let out_path = gen_path.join("mod.rs");
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

fn make_constructor(class: &Class, ctx: &Context, class_name_str: &Literal) -> TokenStream {
    if ctx.is_singleton(&class.name) {
        // Note: we cannot return &'static mut Self, as this would be very easy to mutably alias.
        // &'static Self would be possible, but we would lose the whole mutability information (even if that
        // is best-effort and not strict Rust mutability, it makes the API much more usable).
        // As long as the user has multiple Gd smart pointers to the same singletons, only the internal raw pointers.
        // See also Deref/DerefMut impl for Gd.
        quote! {
            pub fn singleton() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#class_name_str);
                    let __object_ptr = sys::interface_fn!(global_get_singleton)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    } else if !class.is_instantiable {
        // Abstract base classes or non-singleton classes without constructor
        TokenStream::new()
    } else if class.is_refcounted {
        // RefCounted, Resource, etc
        quote! {
            pub fn new() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#class_name_str);
                    let __object_ptr = sys::interface_fn!(classdb_construct_object)(__class_name.string_sys());
                    //let instance = Self { object_ptr };
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    } else {
        // Manually managed classes: Object, Node etc
        quote! {
            #[must_use]
            pub fn new_alloc() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#class_name_str);
                    let __object_ptr = sys::interface_fn!(classdb_construct_object)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    }
}

fn make_class(class: &Class, ctx: &mut Context) -> GeneratedClass {
    //let sys = TokenStream::from_str("::godot_ffi");
    let base = match class.inherits.as_ref() {
        Some(base) => {
            let base = ident(base);
            quote! { crate::engine::#base }
        }
        None => quote! { () },
    };

    let name = ident(&class.name);
    let name_str = strlit(&class.name);

    let constructor = make_constructor(class, ctx, &name_str);

    let methods = make_methods(&class.methods, &class.name, ctx);
    let enums = make_enums(&class.enums, &class.name, ctx);
    let inherits_macro = format_ident!("inherits_transitive_{}", &class.name);
    let all_bases = ctx.inheritance_tree().map_all_bases(&class.name, ident);

    let memory = if &class.name == "Object" {
        ident("DynamicRefCount")
    } else if class.is_refcounted {
        ident("StaticRefCount")
    } else {
        ident("ManualMemory")
    };

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        use godot_ffi as sys;
        use crate::engine::*;
        use crate::builtin::*;
        use crate::obj::{AsArg, Gd};

        pub(super) mod re_export {
            use super::*;

            #[derive(Debug)]
            #[repr(transparent)]
            pub struct #name {
                object_ptr: sys::GDExtensionObjectPtr,
            }
            impl #name {
                #constructor
                #methods
            }
            impl crate::obj::GodotClass for #name {
                type Base = #base;
                type Declarer = crate::obj::dom::EngineDomain;
                type Mem = crate::obj::mem::#memory;

                const CLASS_NAME: &'static str = #name_str;
            }
            impl crate::obj::EngineClass for #name {
                 fn as_object_ptr(&self) -> sys::GDExtensionObjectPtr {
                     self.object_ptr
                 }
                 fn as_type_ptr(&self) -> sys::GDExtensionTypePtr {
                    std::ptr::addr_of!(self.object_ptr) as sys::GDExtensionTypePtr
                 }
            }
            #(
                impl crate::obj::Inherits<crate::engine::#all_bases> for #name {}
            )*
            impl std::ops::Deref for #name {
                type Target = #base;

                fn deref(&self) -> &Self::Target {
                    // SAFETY: same assumptions as `impl Deref for Gd<T>`, see there for comments
                    unsafe { std::mem::transmute::<&Self, &Self::Target>(self) }
                }
            }
            impl std::ops::DerefMut for #name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: see above
                    unsafe { std::mem::transmute::<&mut Self, &mut Self::Target>(self) }
                }
            }

            #[macro_export]
            #[allow(non_snake_case)]
            macro_rules! #inherits_macro {
                ($Class:ident) => {
                    impl ::godot::obj::Inherits<::godot::engine::#name> for $Class {}
                    #(
                        impl ::godot::obj::Inherits<::godot::engine::#all_bases> for $Class {}
                    )*
                }
            }
        }

        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedClass {
        tokens,
        inherits_macro_ident: inherits_macro,
        has_pub_module: !enums.is_empty(),
    }
}

fn make_module_file(classes_and_modules: Vec<GeneratedModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedModule {
            module_ident,
            class_ident,
            is_pub,
            ..
        } = m;

        let vis = is_pub.then_some(quote! { pub });

        quote! {
            #vis mod #module_ident;
            pub use #module_ident::re_export::#class_ident;
        }
    });

    let macros = classes_and_modules.iter().map(|m| {
        let GeneratedModule {
            inherits_macro_ident,
            ..
        } = m;

        // We cannot re-export the following, because macro is in the crate root
        // pub use #module_ident::re_export::#inherits_macro_ident;
        quote! {
            pub use #inherits_macro_ident;
        }
    });

    quote! {
        #( #decls )*

        #[doc(hidden)]
        pub mod class_macros {
            pub use crate::*;
            #( #macros )*
        }
    }
}

fn make_methods(methods: &Option<Vec<Method>>, class_name: &str, ctx: &mut Context) -> TokenStream {
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

fn make_enums(enums: &Option<Vec<ClassEnum>>, _class_name: &str, _ctx: &Context) -> TokenStream {
    let enums = match enums {
        Some(e) => e,
        None => return TokenStream::new(),
    };

    let definitions = enums.iter().map(|e| util::make_enum_definition(e));

    quote! {
        #( #definitions )*
    }
}

#[cfg(not(feature = "codegen-full"))]
fn is_type_excluded(ty: &str, ctx: &mut Context) -> bool {
    let is_class_excluded = |class: &str| !crate::SELECTED_CLASSES.contains(&class);

    match to_rust_type(ty, ctx) {
        RustTy::BuiltinIdent(_) => false,
        RustTy::BuiltinArray(_) => false,
        RustTy::EngineArray { elem_class, .. } => is_class_excluded(elem_class.as_str()),
        RustTy::EngineEnum {
            surrounding_class, ..
        } => match surrounding_class.as_ref() {
            None => false,
            Some(class) => is_class_excluded(class.as_str()),
        },
        RustTy::EngineClass(_) => is_class_excluded(ty),
    }
}

fn is_method_excluded(method: &Method, #[allow(unused_variables)] ctx: &mut Context) -> bool {
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
    #[cfg(not(feature = "codegen-full"))]
    if method
        .return_value
        .as_ref()
        .map_or(false, |ret| is_type_excluded(ret.type_.as_str(), ctx))
        || method.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
    {
        return true;
    }
    // -- end.

    method.name.starts_with('_')
        || method
            .return_value
            .as_ref()
            .map_or(false, |ret| ret.type_.contains('*'))
        || method
            .arguments
            .as_ref()
            .map_or(false, |args| args.iter().any(|arg| arg.type_.contains('*')))
}

#[cfg(feature = "codegen-full")]
fn is_function_excluded(_function: &UtilityFunction, _ctx: &mut Context) -> bool {
    false
}

#[cfg(not(feature = "codegen-full"))]
fn is_function_excluded(function: &UtilityFunction, ctx: &mut Context) -> bool {
    function
        .return_type
        .as_ref()
        .map_or(false, |ret| is_type_excluded(ret.as_str(), ctx))
        || function.arguments.as_ref().map_or(false, |args| {
            args.iter()
                .any(|arg| is_type_excluded(arg.type_.as_str(), ctx))
        })
}

fn make_method_definition(method: &Method, class_name: &str, ctx: &mut Context) -> TokenStream {
    if is_method_excluded(method, ctx) || special_cases::is_deleted(class_name, &method.name) {
        return TokenStream::new();
    }

    let is_varcall = method.is_vararg;
    let (params, arg_exprs) = make_params(&method.arguments, is_varcall, ctx);

    let method_name_str = special_cases::maybe_renamed(class_name, &method.name);
    /*if method.map_args(|args| args.is_empty()) {
        // Getters (i.e. 0 arguments) will be stripped of their `get_` prefix, to conform to Rust convention
        if let Some(remainder) = method_name.strip_prefix("get_") {
            // Do not apply for get_16 etc
            // TODO also not for get_u16 etc, in StreamPeer
            if !remainder.chars().nth(0).unwrap().is_ascii_digit() {
                method_name = remainder;
            }
        }
    }*/
    let method_name = safe_ident(method_name_str);
    let hash = method.hash;

    // TODO &mut safety
    let receiver = if method.is_const {
        quote!(&self)
    } else {
        quote!(&mut self)
    };

    let (return_decl, call) = make_method_return(&method.return_value, is_varcall, ctx);

    let vis = if special_cases::is_private(class_name, &method.name) {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    };

    if is_varcall {
        // varcall (using varargs)
        quote! {
            #vis fn #method_name( #receiver #(, #params )*, varargs: &[Variant]) #return_decl {
                unsafe {
                    let __class_name = StringName::from(#class_name);
                    let __method_name = StringName::from(#method_name_str);
                    let __method_bind = sys::interface_fn!(classdb_get_method_bind)(
                        __class_name.string_sys(),
                        __method_name.string_sys(),
                        #hash
                    );
                    let __call_fn = sys::interface_fn!(object_method_bind_call);

                    let __explicit_args = [
                        #( #arg_exprs ),*
                    ];
                    let mut __args = Vec::new();
                    __args.extend(__explicit_args.iter().map(Variant::var_sys_const));
                    __args.extend(varargs.iter().map(Variant::var_sys_const));

                    let __args_ptr = __args.as_ptr();

                    #call
                }
            }
        }
    } else {
        // ptrcall
        quote! {
            #vis fn #method_name( #receiver, #( #params ),* ) #return_decl {
                unsafe {
                    let __class_name = StringName::from(#class_name);
                    let __method_name = StringName::from(#method_name_str);
                    let __method_bind = sys::interface_fn!(classdb_get_method_bind)(
                        __class_name.string_sys(),
                        __method_name.string_sys(),
                        #hash
                    );
                    let __call_fn = sys::interface_fn!(object_method_bind_ptrcall);

                    let __args = [
                        #( #arg_exprs ),*
                    ];
                    let __args_ptr = __args.as_ptr();

                    #call
                }
            }
        }
    }
}

pub(crate) fn make_function_definition(
    function: &UtilityFunction,
    ctx: &mut Context,
) -> TokenStream {
    if is_function_excluded(function, ctx) {
        return TokenStream::new();
    }

    let is_vararg = function.is_vararg;
    let (params, arg_exprs) = make_params(&function.arguments, is_vararg, ctx);

    let function_name_str = &function.name;
    let function_name = safe_ident(function_name_str);
    let hash = function.hash;

    let (return_decl, call) = make_utility_return(&function.return_type, is_vararg, ctx);

    if is_vararg {
        quote! {
            pub fn #function_name( #( #params , )* varargs: &[Variant]) #return_decl {
                unsafe {
                    let __function_name = StringName::from(#function_name_str);
                    let __call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(__function_name.string_sys(), #hash);
                    let __call_fn = __call_fn.unwrap_unchecked();

                    let __explicit_args = [
                        #( #arg_exprs ),*
                    ];
                    let mut __args = Vec::new();
                    {
                        use godot_ffi::GodotFfi;
                        __args.extend(__explicit_args.iter().map(Variant::sys_const));
                        __args.extend(varargs.iter().map(Variant::sys_const));
                    }

                    let __args_ptr = __args.as_ptr();

                    #call
                }
            }
        }
    } else {
        quote! {
            pub fn #function_name( #( #params ),* ) #return_decl {
                unsafe {
                    let __function_name = StringName::from(#function_name_str);
                    let __call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(__function_name.string_sys(), #hash);
                    let __call_fn = __call_fn.unwrap_unchecked();

                    let __args = [
                        #( #arg_exprs ),*
                    ];
                    let __args_ptr = __args.as_ptr();

                    #call
                }
            }
        }
    }
}

fn make_params(
    method_args: &Option<Vec<MethodArg>>,
    is_varcall: bool,
    ctx: &mut Context,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let empty = vec![];
    let method_args = method_args.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut arg_exprs = vec![];
    for arg in method_args.iter() {
        let param_name = safe_ident(&arg.name);
        let param_ty = to_rust_type(&arg.type_, ctx);

        params.push(quote! { #param_name: #param_ty });
        if is_varcall {
            arg_exprs.push(quote! {
                <#param_ty as ToVariant>::to_variant(&#param_name)
            });
        } else if let RustTy::EngineClass(path) = param_ty {
            arg_exprs.push(quote! {
                <#path as AsArg>::as_arg_ptr(&#param_name)
            });
        } else {
            arg_exprs.push(quote! {
                <#param_ty as sys::GodotFfi>::sys_const(&#param_name)
            });
        }
    }
    (params, arg_exprs)
}

fn make_method_return(
    return_value: &Option<MethodReturn>,
    is_varcall: bool,
    ctx: &mut Context,
) -> (TokenStream, TokenStream) {
    let return_decl: TokenStream;
    let return_ty: Option<RustTy>;
    match return_value {
        Some(ret) => {
            let ty = to_rust_type(&ret.type_, ctx);
            return_decl = ty.return_decl();
            return_ty = Some(ty);
        }
        None => {
            return_decl = TokenStream::new();
            return_ty = None;
        }
    };

    let call = match (is_varcall, return_ty) {
        (true, Some(return_ty)) => {
            // If the return type is not Variant, then convert to concrete target type
            let return_expr = match return_ty {
                RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { variant },
                _ => quote! { variant.to() },
            };

            // TODO use Result instead of panic on error
            quote! {
                let variant = Variant::from_var_sys_init(|return_ptr| {
                    let mut __err = sys::default_call_error();
                    __call_fn(__method_bind, self.object_ptr, __args_ptr, __args.len() as i64, return_ptr, std::ptr::addr_of_mut!(__err));
                    assert_eq!(__err.error, sys::GDEXTENSION_CALL_OK);
                });
                #return_expr
            }
        }
        (true, None) => {
            // TODO use Result instead of panic on error
            quote! {
                let mut __err = sys::default_call_error();
                __call_fn(__method_bind, self.object_ptr, __args_ptr, __args.len() as i64, std::ptr::null_mut(), std::ptr::addr_of_mut!(__err));
                assert_eq!(__err.error, sys::GDEXTENSION_CALL_OK);
            }
        }
        (false, Some(RustTy::EngineClass(return_ty))) => {
            quote! {
                <#return_ty>::from_sys_init_opt(|return_ptr| {
                    __call_fn(__method_bind, self.object_ptr, __args_ptr, return_ptr);
                })
            }
        }
        (false, Some(return_ty)) => {
            quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init(|return_ptr| {
                    __call_fn(__method_bind, self.object_ptr, __args_ptr, return_ptr);
                })
            }
        }
        (false, None) => {
            quote! {
                __call_fn(__method_bind, self.object_ptr, __args_ptr, std::ptr::null_mut());
            }
        }
    };

    (return_decl, call)
}

fn make_utility_return(
    return_value: &Option<String>,
    is_vararg: bool,
    ctx: &mut Context,
) -> (TokenStream, TokenStream) {
    let return_decl;
    let return_ty;

    if let Some(ret) = return_value {
        let ty = to_rust_type(ret, ctx);
        return_decl = ty.return_decl();
        return_ty = Some(ty);
    } else {
        return_decl = TokenStream::new();
        return_ty = None;
    }

    let call = match (is_vararg, return_ty) {
        (true, Some(return_ty)) => {
            // If the return type is not Variant, then convert to concrete target type
            let return_expr = match return_ty {
                RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { variant },
                _ => quote! { variant.to() },
            };

            quote! {
                use godot_ffi::GodotFfi;
                let variant = Variant::from_sys_init(|return_ptr| {
                    __call_fn(return_ptr, __args_ptr, __args.len() as i32);
                });
                #return_expr
            }
        }
        (true, None) => {
            quote! {
                __call_fn(std::ptr::null_mut(), __args_ptr, __args.len() as i32);
            }
        }
        (false, Some(RustTy::EngineClass(return_ty))) => {
            quote! {
                <#return_ty>::from_sys_init_opt(|return_ptr| {
                    __call_fn(return_ptr, __args_ptr, __args.len() as i32);
                })
            }
        }
        (false, Some(return_ty)) => {
            quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init(|return_ptr| {
                    __call_fn(return_ptr, __args_ptr, __args.len() as i32);
                })
            }
        }
        (false, None) => {
            quote! {
                __call_fn(std::ptr::null_mut(), __args_ptr, __args.len() as i32);
            }
        }
    };

    (return_decl, call)
}
