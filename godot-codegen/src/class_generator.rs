/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates a file for each Godot class

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::{c_str, ident, safe_ident, strlit, to_module_name, to_rust_type};
use crate::{
    special_cases, Context, GeneratedClass, GeneratedModule, KNOWN_TYPES, SELECTED_CLASSES,
};

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
        if !SELECTED_CLASSES.contains(&class.name.as_str())
            || special_cases::is_class_deleted(&class.name.as_str())
        {
            continue;
        }

        let generated_class = make_class(class, &ctx);
        let file_contents = generated_class.tokens.to_string();

        let module_name = to_module_name(&class.name);
        let out_path = gen_path.join(format!("{}.rs", module_name));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");

        let class_ident = ident(&class.name);
        let module_ident = ident(&module_name);
        modules.push(GeneratedModule {
            class_ident,
            module_ident,
            inherits_macro_ident: generated_class.inherits_macro_ident,
            is_pub: generated_class.has_pub_module,
        });
        out_files.push(out_path);
    }

    let mod_contents = make_module_file(modules).to_string();
    let out_path = gen_path.join("mod.rs");
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

fn make_constructor(class: &Class, ctx: &Context, class_name_cstr: TokenStream) -> TokenStream {
    if ctx.is_singleton(&class.name) {
        // Note: we cannot return &'static mut Self, as this would be very easy to mutably alias.
        // &'static Self would be possible, but we would lose the whole mutability information (even if that
        // is best-effort and not strict Rust mutability, it makes the API much more usable).
        // As long as the user has multiple Gd smart pointers to the same singletons, only the internal raw pointers.
        // See also Deref/DerefMut impl for Gd.
        quote! {
            pub fn singleton() -> Gd<Self> {
                unsafe {
                    let object_ptr = sys::interface_fn!(global_get_singleton)(#class_name_cstr);
                    Gd::from_obj_sys(object_ptr)
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
                    let object_ptr = sys::interface_fn!(classdb_construct_object)(#class_name_cstr);
                    //let instance = Self { object_ptr };
                    Gd::from_obj_sys(object_ptr)
                }
            }
        }
    } else {
        // Manually managed classes: Object, Node etc
        quote! {
            #[must_use]
            pub fn new_alloc() -> Gd<Self> {
                unsafe {
                    let object_ptr = sys::interface_fn!(classdb_construct_object)(#class_name_cstr);
                    Gd::from_obj_sys(object_ptr)
                }
            }
        }
    }
}

fn make_class(class: &Class, ctx: &Context) -> GeneratedClass {
    //let sys = TokenStream::from_str("::godot_ffi");
    let base = match class.inherits.as_ref() {
        Some(base) => {
            let base = ident(base);
            quote! { crate::api::#base }
        }
        None => quote! { () },
    };

    let name = ident(&class.name);
    let name_str = strlit(&class.name);
    let name_cstr = c_str(&class.name);

    let constructor = make_constructor(class, ctx, name_cstr);

    let methods = make_methods(&class.methods, &class.name, ctx);
    let enums = make_enums(&class.enums, &class.name, ctx);
    let inherits_macro = format_ident!("inherits_transitive_{}", &class.name);
    let all_bases = ctx.inheritance_tree.map_all_bases(&class.name, ident);

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
        use crate::api::*;
        use crate::builtin::*;
        use crate::obj::Gd;
        use crate::traits::AsArg;

        pub(super) mod re_export {
            use super::*;

            #[derive(Debug)]
            #[repr(transparent)]
            pub struct #name {
                object_ptr: sys::GDNativeObjectPtr,
            }
            impl #name {
                #constructor
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
                    impl ::godot::traits::Inherits<::godot::api::#name> for $Class {}
                    #(
                        impl ::godot::traits::Inherits<::godot::api::#all_bases> for $Class {}
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

fn make_enums(enums: &Option<Vec<Enum>>, _class_name: &str, _ctx: &Context) -> TokenStream {
    let enums = match enums {
        Some(e) => e,
        None => return TokenStream::new(),
    };

    let definitions = enums.iter().map(|enum_| make_enum_definition(enum_));

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
    if is_method_excluded(method) || special_cases::is_deleted(class_name, &method.name) {
        return TokenStream::new();
    }

    let is_varcall = method.is_vararg;
    let (params, arg_exprs) = make_params(&method.arguments, is_varcall, ctx);

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
                let result = unsafe {
                    let method_bind = sys::interface_fn!(classdb_get_method_bind)(#c_class_name, #c_method_name, #hash);
                    let call_fn = sys::interface_fn!(object_method_bind_call);

                    let explicit_args = [
                        #( #arg_exprs ),*
                    ];
                    let mut args = Vec::new();
                    args.extend(explicit_args.iter().map(Variant::var_sys));
                    args.extend(varargs.iter().map(Variant::var_sys));

                    let args_ptr = args.as_ptr();

                    #call
                };

                result
            }
        }
    } else {
        // ptrcall
        quote! {
            #vis fn #method_name( #receiver, #( #params ),* ) #return_decl {
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
}

pub(crate) fn make_function_definition(function: &UtilityFunction, ctx: &Context) -> TokenStream {
    // TODO support vararg functions
    if is_function_excluded(function) || function.is_vararg {
        return TokenStream::new();
    }

    let is_vararg = function.is_vararg;
    let (params, arg_exprs) = make_params(&function.arguments, is_vararg, ctx);

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
    is_varcall: bool,
    ctx: &Context,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let empty = vec![];
    let method_args = method_args.as_ref().unwrap_or(&empty);

    let mut params = vec![];
    let mut arg_exprs = vec![];
    for arg in method_args.iter() {
        let param_name = safe_ident(&arg.name);
        let param = to_rust_type(&arg.type_, ctx);
        let param_ty = param.tokens;

        params.push(quote! { #param_name: #param_ty });
        if is_varcall {
            arg_exprs.push(quote! {
                <#param_ty as ToVariant>::to_variant(&#param_name)
            });
        } else if param.is_engine_class {
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
    is_varcall: bool,
    ctx: &Context,
) -> (TokenStream, TokenStream) {
    let return_ty;
    let return_decl;
    match return_value {
        Some(ret) => {
            return_ty = Some(to_rust_type(&ret.type_, ctx).tokens);
            return_decl = quote! { -> #return_ty };
        }
        None => {
            return_ty = None;
            return_decl = TokenStream::new();
        }
    };

    let call = match (is_varcall, return_ty) {
        (true, _ret) => {
            // TODO use Result instead of panic on error
            quote! {
                Variant::from_var_sys_init(|return_ptr| {
                    let mut err = sys::default_call_error();
                    call_fn(method_bind, self.object_ptr, args_ptr, args.len() as i64, return_ptr, std::ptr::addr_of_mut!(err));
                    assert_eq!(err.error, sys::GDNATIVE_CALL_OK);
                })
            }
        }
        (false, Some(return_ty)) => {
            quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init(|return_ptr| {
                    call_fn(method_bind, self.object_ptr, args_ptr, return_ptr);
                })
            }
        }
        (false, None) => {
            quote! {
                call_fn(method_bind, self.object_ptr, args_ptr, std::ptr::null_mut());
            }
        }
    };

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

fn make_enum_definition(enum_: &Enum) -> TokenStream {
    let enum_name = ident(&enum_.name);

    let enumerators = enum_.values.iter().map(|enumerator| {
        let name = make_enumerator_name(&enumerator.name, &enum_.name);
        let ordinal = &enumerator.value;
        quote! {
            pub const #name: Self = Self { ord: #ordinal };
        }
    });

    // Enumerator ordinal stored as i32, since that's enough to hold all current values.
    // Public interface is i64 though, for forward compatibility.
    quote! {
        #[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
        pub struct #enum_name {
            ord: i32
        }
        impl #enum_name {
            /// Ordinal value of the enumerator, as specified in Godot.
            /// This is not necessarily unique.
            pub const fn ord(self) -> i64 {
                self.ord as i64
            }

            #(
                #enumerators
            )*
        }

    }
}

fn make_enumerator_name(enumerator_name: &str, _enum_name: &str) -> Ident {
    // TODO strip prefixes of `enum_name` appearing in `enumerator_name`
    // tons of variantions, see test cases in lib.rs

    ident(enumerator_name)
}
