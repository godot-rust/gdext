/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::generator::functions_common::{FnCode, FnDefinition, FnDefinitions};
use crate::generator::method_tables::MethodTableKey;
use crate::generator::{enums, functions_common};
use crate::models::domain::{
    BuiltinClass, BuiltinMethod, ClassLike, ExtensionApi, FnDirection, Function, ModName, RustTy,
    TyName,
};
use crate::{conv, util, SubmitFn};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::path::Path;

// Shared with native_structures.rs.
pub struct GeneratedBuiltin {
    pub code: TokenStream,
}

pub struct GeneratedBuiltinModule {
    pub symbol_ident: Ident,
    pub module_name: ModName,
}

pub fn generate_builtin_class_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create classes directory");

    let mut modules = vec![];
    for class in api.builtins.iter() {
        let Some(class) = class.builtin_class.as_ref() else {
            continue;
        };

        // let godot_class_name = &class.name().godot_ty;
        let module_name = class.mod_name();

        let generated_class = make_builtin_class(class, ctx);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedBuiltinModule {
            symbol_ident: class.inner_name().clone(),
            module_name: module_name.clone(),
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules);

    submit_fn(out_path, mod_contents);
}

pub fn make_builtin_module_file(classes_and_modules: Vec<GeneratedBuiltinModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedBuiltinModule {
            module_name,
            symbol_ident,
            ..
        } = m;

        quote! {
            mod #module_name;
            pub use #module_name::#symbol_ident;
        }
    });

    quote! {
        #( #decls )*
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_builtin_class(class: &BuiltinClass, ctx: &mut Context) -> GeneratedBuiltin {
    let godot_name = &class.name().godot_ty;

    let RustTy::BuiltinIdent(outer_class) = conv::to_rust_type(godot_name, None, ctx) else {
        panic!("Rust type `{}` categorized wrong", godot_name)
    };
    let inner_class = class.inner_name();

    let FnDefinitions {
        functions: methods,
        builders,
    } = make_builtin_methods(class, &class.methods, ctx);

    let imports = util::make_imports();
    let enums = enums::make_enums(&class.enums);
    let special_constructors = make_special_builtin_methods(class.name(), ctx);

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let code = quote! {
        #imports

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

        #builders
        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    GeneratedBuiltin { code }
}

fn make_builtin_methods(
    builtin_class: &BuiltinClass,
    methods: &[BuiltinMethod],
    ctx: &mut Context,
) -> FnDefinitions {
    let definitions = methods
        .iter()
        .map(|method| make_builtin_method_definition(builtin_class, method, ctx));

    FnDefinitions::expand(definitions)
}

/// Depending on the built-in class, adds custom constructors and methods.
fn make_special_builtin_methods(class_name: &TyName, _ctx: &Context) -> TokenStream {
    if class_name.godot_ty == "Array" {
        quote! {
            pub fn from_outer_typed<T>(outer: &Array<T>) -> Self
                where
                    T: crate::builtin::meta::GodotType
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

fn make_builtin_method_definition(
    builtin_class: &BuiltinClass,
    method: &BuiltinMethod,
    ctx: &mut Context,
) -> FnDefinition {
    let FnDirection::Outbound { hash } = method.direction() else {
        unreachable!("builtin methods are never virtual")
    };

    let builtin_name = builtin_class.name();
    let builtin_name_str = builtin_name.rust_ty.to_string();
    let method_name_str = method.godot_name();

    let fptr_access = if cfg!(feature = "codegen-lazy-fptrs") {
        let variant_type = quote! { sys::VariantType::#builtin_name };
        let variant_type_str = &builtin_name.godot_ty;

        quote! {
            fptr_by_key(sys::lazy_keys::BuiltinMethodKey {
                variant_type: #variant_type,
                variant_type_str: #variant_type_str,
                method_name: #method_name_str,
                hash: #hash,
            })
        }
    } else {
        let table_index = ctx.get_table_index(&MethodTableKey::from_builtin(builtin_class, method));

        quote! { fptr_by_index(#table_index) }
    };

    let receiver = functions_common::make_receiver(method.qualifier(), quote! { self.sys_ptr });
    let object_ptr = &receiver.ffi_arg;

    let ptrcall_invocation = quote! {
        let method_bind = sys::builtin_method_table().#fptr_access;

        <CallSig as PtrcallSignatureTuple>::out_builtin_ptrcall::<RetMarshal>(
            method_bind,
            #builtin_name_str,
            #method_name_str,
            #object_ptr,
            args
        )
    };

    // TODO(#382): wait for https://github.com/godot-rust/gdext/issues/382
    let varcall_invocation = quote! {
        /*<CallSig as VarcallSignatureTuple>::out_class_varcall(
            method_bind,
            #method_name_str,
            #object_ptr,
            args,
            varargs
        )*/
    };

    functions_common::make_function_definition(
        method,
        &FnCode {
            receiver,
            varcall_invocation,
            ptrcall_invocation,
        },
    )
}
