/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Generates a file for each Godot engine + builtin class

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::central_generator::{collect_builtin_types, BuiltinTypeInfo};
use crate::util::{ident, safe_ident, to_pascal_case, to_rust_type};
use crate::{
    special_cases, util, Context, GeneratedBuiltin, GeneratedBuiltinModule, GeneratedClass,
    GeneratedClassModule, ModName, RustTy, TyName,
};

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
        let class_name = TyName::from_godot(&class.name);
        let module_name = ModName::from_godot(&class.name);

        #[cfg(not(feature = "codegen-full"))]
        if !crate::SELECTED_CLASSES.contains(&class_name.godot_ty.as_str()) {
            continue;
        }

        if special_cases::is_class_deleted(&class_name) {
            continue;
        }

        let generated_class = make_class(class, &class_name, ctx);
        let file_contents = generated_class.tokens.to_string();

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");
        out_files.push(out_path);

        modules.push(GeneratedClassModule {
            class_name,
            module_name,
            inherits_macro_ident: generated_class.inherits_macro_ident,
            is_pub: generated_class.has_pub_module,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_module_file(modules).to_string();
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

pub(crate) fn generate_builtin_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    _build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let builtin_types_map = collect_builtin_types(api);

    let mut modules = vec![];
    for class in api.builtin_classes.iter() {
        let module_name = ModName::from_godot(&class.name);
        let class_name = TyName::from_godot(&class.name);
        let inner_class_name = TyName::from_godot(&format!("Inner{}", class.name));

        if special_cases::is_builtin_type_deleted(&class_name) {
            continue;
        }

        let type_info = builtin_types_map
            .get(&class.name)
            .unwrap_or_else(|| panic!("builtin type not found: {}", class.name));

        let generated_class =
            make_builtin_class(class, &class_name, &inner_class_name, type_info, ctx);
        let file_contents = generated_class.tokens.to_string();

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));
        std::fs::write(&out_path, file_contents).expect("failed to write class file");
        out_files.push(out_path);

        modules.push(GeneratedBuiltinModule {
            class_name: inner_class_name,
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules).to_string();
    std::fs::write(&out_path, mod_contents).expect("failed to write mod.rs file");
    out_files.push(out_path);
}

fn make_constructor(class: &Class, ctx: &Context) -> TokenStream {
    let godot_class_name = &class.name;
    if ctx.is_singleton(godot_class_name) {
        // Note: we cannot return &'static mut Self, as this would be very easy to mutably alias.
        // &'static Self would be possible, but we would lose the whole mutability information (even if that
        // is best-effort and not strict Rust mutability, it makes the API much more usable).
        // As long as the user has multiple Gd smart pointers to the same singletons, only the internal raw pointers.
        // See also Deref/DerefMut impl for Gd.
        quote! {
            pub fn singleton() -> Gd<Self> {
                unsafe {
                    let __class_name = StringName::from(#godot_class_name);
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
                    let __class_name = StringName::from(#godot_class_name);
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
                    let __class_name = StringName::from(#godot_class_name);
                    let __object_ptr = sys::interface_fn!(classdb_construct_object)(__class_name.string_sys());
                    Gd::from_obj_sys(__object_ptr)
                }
            }
        }
    }
}

fn make_class(class: &Class, class_name: &TyName, ctx: &mut Context) -> GeneratedClass {
    // Strings
    let godot_class_str = &class_name.godot_ty;

    // Idents and tokens
    let base = match class.inherits.as_ref() {
        Some(base) => {
            let base = ident(&to_pascal_case(base));
            quote! { crate::engine::#base }
        }
        None => quote! { () },
    };

    let constructor = make_constructor(class, ctx);
    let methods = make_methods(&class.methods, class_name, ctx);
    let enums = make_enums(&class.enums, class_name, ctx);
    let inherits_macro = format_ident!("inherits_transitive_{}", class_name.rust_ty);
    let all_bases = ctx.inheritance_tree().collect_all_bases(class_name);

    let memory = if class_name.rust_ty == "Object" {
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
        use sys::GodotFfi as _;

        pub(super) mod re_export {
            use super::*;

            #[derive(Debug)]
            #[repr(transparent)]
            pub struct #class_name {
                object_ptr: sys::GDExtensionObjectPtr,
            }
            impl #class_name {
                #constructor
                #methods
            }
            impl crate::obj::GodotClass for #class_name {
                type Base = #base;
                type Declarer = crate::obj::dom::EngineDomain;
                type Mem = crate::obj::mem::#memory;

                const CLASS_NAME: &'static str = #godot_class_str;
            }
            impl crate::obj::EngineClass for #class_name {
                 fn as_object_ptr(&self) -> sys::GDExtensionObjectPtr {
                     self.object_ptr
                 }
                 fn as_type_ptr(&self) -> sys::GDExtensionTypePtr {
                    std::ptr::addr_of!(self.object_ptr) as sys::GDExtensionTypePtr
                 }
            }
            #(
                impl crate::obj::Inherits<crate::engine::#all_bases> for #class_name {}
            )*
            impl std::ops::Deref for #class_name {
                type Target = #base;

                fn deref(&self) -> &Self::Target {
                    // SAFETY: same assumptions as `impl Deref for Gd<T>`, see there for comments
                    unsafe { std::mem::transmute::<&Self, &Self::Target>(self) }
                }
            }
            impl std::ops::DerefMut for #class_name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: see above
                    unsafe { std::mem::transmute::<&mut Self, &mut Self::Target>(self) }
                }
            }

            #[macro_export]
            #[allow(non_snake_case)]
            macro_rules! #inherits_macro {
                ($Class:ident) => {
                    impl ::godot::obj::Inherits<::godot::engine::#class_name> for $Class {}
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

fn make_builtin_class(
    class: &BuiltinClass,
    class_name: &TyName,
    inner_class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> GeneratedBuiltin {
    let outer_class = if let RustTy::BuiltinIdent(ident) = to_rust_type(&class.name, ctx) {
        ident
    } else {
        panic!("Rust type `{}` categorized wrong", class.name)
    };
    let inner_class = &inner_class_name.rust_ty;

    let class_enums = class.enums.as_ref().map(|class_enums| {
        class_enums
            .iter()
            .map(BuiltinClassEnum::to_enum)
            .collect::<Vec<Enum>>()
    });

    let methods = make_builtin_methods(&class.methods, class_name, type_info, ctx);
    let enums = make_enums(&class_enums, class_name, ctx);
    let special_constructors = make_special_builtin_methods(class_name, ctx);

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::obj::{AsArg, Gd};
        use crate::sys::GodotFfi as _;
        use crate::engine::Object;

        #[repr(transparent)]
        pub struct #inner_class<'a> {
            _outer_lifetime: std::marker::PhantomData<&'a ()>,
            sys_ptr: sys::GDExtensionTypePtr,
        }
        impl<'a> #inner_class<'a> {
            pub fn from_outer(outer: &#outer_class) -> Self {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: outer.sys(),
                }
            }
            #special_constructors
            #methods
        }

        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedBuiltin { tokens }
}

fn make_module_file(classes_and_modules: Vec<GeneratedClassModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedClassModule {
            module_name,
            class_name,
            is_pub,
            ..
        } = m;

        let vis = is_pub.then_some(quote! { pub });

        quote! {
            #vis mod #module_name;
            pub use #module_name::re_export::#class_name;
        }
    });

    let macros = classes_and_modules.iter().map(|m| {
        let GeneratedClassModule {
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

fn make_builtin_module_file(classes_and_modules: Vec<GeneratedBuiltinModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedBuiltinModule {
            module_name,
            class_name,
            ..
        } = m;

        quote! {
            mod #module_name;
            pub use #module_name::#class_name;
        }
    });

    quote! {
        #( #decls )*
    }
}

fn make_methods(
    methods: &Option<Vec<ClassMethod>>,
    class_name: &TyName,
    ctx: &mut Context,
) -> TokenStream {
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

fn make_builtin_methods(
    methods: &Option<Vec<BuiltinClassMethod>>,
    class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> TokenStream {
    let methods = match methods {
        Some(m) => m,
        None => return TokenStream::new(),
    };

    let definitions = methods
        .iter()
        .map(|method| make_builtin_method_definition(method, class_name, type_info, ctx));

    quote! {
        #( #definitions )*
    }
}

fn make_enums(enums: &Option<Vec<Enum>>, _class_name: &TyName, _ctx: &Context) -> TokenStream {
    let enums = match enums {
        Some(e) => e,
        None => return TokenStream::new(),
    };

    let definitions = enums.iter().map(util::make_enum_definition);

    quote! {
        #( #definitions )*
    }
}

/// Depending on the built-in class, adds custom constructors and methods.
fn make_special_builtin_methods(class_name: &TyName, _ctx: &Context) -> TokenStream {
    if class_name.godot_ty == "Array" {
        quote! {
            pub fn from_outer_typed<T>(outer: &TypedArray<T>) -> Self
                where T: crate::builtin::meta::VariantMetadata
            {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: outer.sys(),
                }
            }
        }
    } else {
        TokenStream::new()
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
        RustTy::EngineClass { .. } => is_class_excluded(ty),
    }
}

fn is_method_excluded(method: &ClassMethod, #[allow(unused_variables)] ctx: &mut Context) -> bool {
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

fn make_method_definition(
    method: &ClassMethod,
    class_name: &TyName,
    ctx: &mut Context,
) -> TokenStream {
    if is_method_excluded(method, ctx) || special_cases::is_deleted(class_name, &method.name) {
        return TokenStream::new();
    }
    /*if method.map_args(|args| args.is_empty()) {
        // Getters (i.e. 0 arguments) will be stripped of their `get_` prefix, to conform to Rust convention
        if let Some(remainder) = method_name.strip_prefix("get_") {
            // TODO Do not apply for FileAccess::get_16, StreamPeer::get_u16, etc
            if !remainder.chars().nth(0).unwrap().is_ascii_digit() {
                method_name = remainder;
            }
        }
    }*/

    let method_name_str = special_cases::maybe_renamed(class_name, &method.name);

    let (receiver, receiver_arg) = make_receiver(
        method.is_static,
        method.is_const,
        quote! { self.object_ptr },
    );

    let hash = method.hash;
    let is_varcall = method.is_vararg;

    let variant_ffi = is_varcall.then(VariantFfi::variant_ptr);
    let function_provider = if is_varcall {
        ident("object_method_bind_call")
    } else {
        ident("object_method_bind_ptrcall")
    };

    let class_name_str = &class_name.godot_ty;
    let init_code = quote! {
        let __class_name = StringName::from(#class_name_str);
        let __method_name = StringName::from(#method_name_str);
        let __method_bind = sys::interface_fn!(classdb_get_method_bind)(
            __class_name.string_sys(),
            __method_name.string_sys(),
            #hash
        );
        let __call_fn = sys::interface_fn!(#function_provider);
    };
    let varcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_arg, __args_ptr, __args.len() as i64, return_ptr, std::ptr::addr_of_mut!(__err));
    };
    let ptrcall_invocation = quote! {
        __call_fn(__method_bind, #receiver_arg, __args_ptr, return_ptr);
    };

    make_function_definition(
        method_name_str,
        special_cases::is_private(class_name, &method.name),
        receiver,
        &method.arguments,
        method.return_value.as_ref(),
        variant_ffi,
        init_code,
        &varcall_invocation,
        &ptrcall_invocation,
        ctx,
    )
}

fn make_builtin_method_definition(
    method: &BuiltinClassMethod,
    class_name: &TyName,
    type_info: &BuiltinTypeInfo,
    ctx: &mut Context,
) -> TokenStream {
    // TODO implement varcalls
    if method.is_vararg {
        return TokenStream::new();
    }

    let method_name_str = &method.name;

    let (receiver, receiver_arg) =
        make_receiver(method.is_static, method.is_const, quote! { self.sys_ptr });

    let return_value = method.return_type.as_deref().map(MethodReturn::from_type);
    let hash = method.hash;
    let is_varcall = method.is_vararg;
    let variant_ffi = is_varcall.then(VariantFfi::type_ptr);

    let variant_type = &type_info.type_names.sys_variant_type;
    let init_code = quote! {
        let __variant_type = sys::#variant_type;
        let __method_name = StringName::from(#method_name_str);
        let __call_fn = sys::interface_fn!(variant_get_ptr_builtin_method)(
            __variant_type,
            __method_name.string_sys(),
            #hash
        );
        let __call_fn = __call_fn.unwrap_unchecked();
    };
    let ptrcall_invocation = quote! {
        __call_fn(#receiver_arg, __args_ptr, return_ptr, __args.len() as i32);
    };

    make_function_definition(
        method_name_str,
        special_cases::is_private(class_name, &method.name),
        receiver,
        &method.arguments,
        return_value.as_ref(),
        variant_ffi,
        init_code,
        &ptrcall_invocation,
        &ptrcall_invocation,
        ctx,
    )
}

pub(crate) fn make_utility_function_definition(
    function: &UtilityFunction,
    ctx: &mut Context,
) -> TokenStream {
    if is_function_excluded(function, ctx) {
        return TokenStream::new();
    }

    let function_name_str = &function.name;
    let return_value = function.return_type.as_deref().map(MethodReturn::from_type);
    let hash = function.hash;
    let variant_ffi = function.is_vararg.then_some(VariantFfi::type_ptr());
    let init_code = quote! {
        let __function_name = StringName::from(#function_name_str);
        let __call_fn = sys::interface_fn!(variant_get_ptr_utility_function)(__function_name.string_sys(), #hash);
        let __call_fn = __call_fn.unwrap_unchecked();
    };
    let invocation = quote! {
        __call_fn(return_ptr, __args_ptr, __args.len() as i32);
    };

    make_function_definition(
        &function.name,
        false,
        TokenStream::new(),
        &function.arguments,
        return_value.as_ref(),
        variant_ffi,
        init_code,
        &invocation,
        &invocation,
        ctx,
    )
}

/// Defines which methods to use to convert between `Variant` and FFI (either variant ptr or type ptr)
struct VariantFfi {
    sys_method: Ident,
    from_sys_init_method: Ident,
}
impl VariantFfi {
    fn variant_ptr() -> Self {
        Self {
            sys_method: ident("var_sys_const"),
            from_sys_init_method: ident("from_var_sys_init"),
        }
    }
    fn type_ptr() -> Self {
        Self {
            sys_method: ident("sys_const"),
            from_sys_init_method: ident("from_sys_init_default"),
        }
    }
}

#[allow(clippy::too_many_arguments)] // adding a struct/trait that's used only here, one time, reduces complexity by precisely 0%
fn make_function_definition(
    function_name: &str,
    is_private: bool,
    receiver: TokenStream,
    method_args: &Option<Vec<MethodArg>>,
    return_value: Option<&MethodReturn>,
    variant_ffi: Option<VariantFfi>,
    init_code: TokenStream,
    varcall_invocation: &TokenStream,
    ptrcall_invocation: &TokenStream,
    ctx: &mut Context,
) -> TokenStream {
    let vis = if is_private {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    };

    let is_varcall = variant_ffi.is_some();
    let fn_name = safe_ident(function_name);
    let (params, arg_exprs) = make_params(method_args, is_varcall, ctx);
    let (return_decl, call_code) = make_return(
        return_value,
        variant_ffi.as_ref(),
        varcall_invocation,
        ptrcall_invocation,
        ctx,
    );

    if let Some(variant_ffi) = variant_ffi.as_ref() {
        // varcall (using varargs)
        let sys_method = &variant_ffi.sys_method;
        quote! {
            #vis fn #fn_name( #receiver #( #params, )* varargs: &[Variant]) #return_decl {
                unsafe {
                    #init_code

                    let __explicit_args = [
                        #( #arg_exprs ),*
                    ];

                    let mut __args = Vec::new();
                    __args.extend(__explicit_args.iter().map(Variant::#sys_method));
                    __args.extend(varargs.iter().map(Variant::#sys_method));

                    let __args_ptr = __args.as_ptr();

                    #call_code
                }
            }
        }
    } else {
        // ptrcall
        quote! {
            #vis fn #fn_name( #receiver #( #params, )* ) #return_decl {
                unsafe {
                    #init_code

                    let __args = [
                        #( #arg_exprs ),*
                    ];

                    let __args_ptr = __args.as_ptr();

                    #call_code
                }
            }
        }
    }
}

fn make_receiver(
    is_static: bool,
    is_const: bool,
    receiver_arg: TokenStream,
) -> (TokenStream, TokenStream) {
    let receiver = if is_static {
        quote! {}
    } else if is_const {
        quote! { &self, }
    } else {
        quote! { &mut self, }
    };

    let receiver_arg = if is_static {
        quote! { std::ptr::null_mut() }
    } else {
        receiver_arg
    };

    (receiver, receiver_arg)
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
        } else if let RustTy::EngineClass { tokens: path, .. } = param_ty {
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

fn make_return(
    return_value: Option<&MethodReturn>,
    variant_ffi: Option<&VariantFfi>,
    varcall_invocation: &TokenStream,
    ptrcall_invocation: &TokenStream,
    ctx: &mut Context,
) -> (TokenStream, TokenStream) {
    let return_decl: TokenStream;
    let return_ty: Option<RustTy>;

    if let Some(ret) = return_value {
        let ty = to_rust_type(&ret.type_, ctx);
        return_decl = ty.return_decl();
        return_ty = Some(ty);
    } else {
        return_decl = TokenStream::new();
        return_ty = None;
    }

    let call = match (variant_ffi, return_ty) {
        (Some(variant_ffi), Some(return_ty)) => {
            // If the return type is not Variant, then convert to concrete target type
            let return_expr = match return_ty {
                RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { variant },
                _ => quote! { variant.to() },
            };
            let from_sys_init_method = &variant_ffi.from_sys_init_method;

            // Note: __err may remain unused if the #call does not handle errors (e.g. utility fn, ptrcall, ...)
            // TODO use Result instead of panic on error
            quote! {
                let variant = Variant::#from_sys_init_method(|return_ptr| {
                    let mut __err = sys::default_call_error();
                    #varcall_invocation
                    sys::panic_on_call_error(&__err);
                });
                #return_expr
            }
        }
        (Some(_), None) => {
            // Note: __err may remain unused if the #call does not handle errors (e.g. utility fn, ptrcall, ...)
            // TODO use Result instead of panic on error
            quote! {
                let mut __err = sys::default_call_error();
                let return_ptr = std::ptr::null_mut();
                #varcall_invocation
                sys::panic_on_call_error(&__err);
            }
        }
        (None, Some(RustTy::EngineClass { tokens, .. })) => {
            let return_ty = tokens;
            quote! {
                <#return_ty>::from_sys_init_opt(|return_ptr| {
                    #ptrcall_invocation
                })
            }
        }
        (None, Some(return_ty)) => {
            quote! {
                <#return_ty as sys::GodotFfi>::from_sys_init_default(|return_ptr| {
                    #ptrcall_invocation
                })
            }
        }
        (None, None) => {
            quote! {
                let return_ptr = std::ptr::null_mut();
                #ptrcall_invocation
            }
        }
    };

    (return_decl, call)
}
