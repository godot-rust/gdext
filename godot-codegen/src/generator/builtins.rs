/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::context::Context;
use crate::generator::functions_common::{FnCode, FnDefinition, FnDefinitions};
use crate::generator::method_tables::MethodTableKey;
use crate::generator::{enums, functions_common};
use crate::models::domain::{
    BuiltinClass, BuiltinMethod, ClassLike, ExtensionApi, FlowDirection, FnDirection, Function,
    ModName, RustTy, TyName,
};
use crate::{conv, util, SubmitFn};

// Shared with native_structures.rs.
pub struct GeneratedBuiltin {
    pub code: TokenStream,
    pub has_sidecar_module: bool,
}

pub struct GeneratedBuiltinModule {
    pub outer_builtin: Ident,
    pub inner_builtin: Ident,
    pub module_name: ModName,
    pub is_pub_sidecar: bool,
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
    for variant in api.builtins.iter() {
        let Some(class) = variant.builtin_class.as_ref() else {
            continue;
        };

        // let godot_class_name = &class.name().godot_ty;
        let module_name = class.mod_name();

        let variant_shout_name = util::ident(variant.godot_shout_name());
        let generated_builtin = make_builtin_class(class, &variant_shout_name, ctx);
        let file_contents = generated_builtin.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(GeneratedBuiltinModule {
            outer_builtin: class.name().rust_ty.clone(),
            inner_builtin: class.inner_name().clone(),
            module_name: module_name.clone(),
            is_pub_sidecar: generated_builtin.has_sidecar_module,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = make_builtin_module_file(modules);

    submit_fn(out_path, mod_contents);
}

pub fn make_builtin_module_file(classes_and_modules: Vec<GeneratedBuiltinModule>) -> TokenStream {
    let decls = classes_and_modules.iter().map(|m| {
        let GeneratedBuiltinModule {
            outer_builtin,
            inner_builtin,
            module_name,
            is_pub_sidecar,
        } = m;

        // Module is public if it has default extenders (Ex* builders). Enums do not contribute, they're all manually redefined.
        let vis = is_pub_sidecar.then_some(quote! { pub });

        let doc = format!("Default extenders for builtin type [`{outer_builtin}`][crate::builtin::{outer_builtin}].");

        quote! {
            #[doc = #doc]
            #vis mod #module_name;
            pub use #module_name::#inner_builtin;
        }
    });

    quote! {
        #( #decls )*
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_builtin_class(
    class: &BuiltinClass,
    variant_shout_name: &Ident,
    ctx: &mut Context,
) -> GeneratedBuiltin {
    let godot_name = &class.name().godot_ty;

    // Meta direction irrelevant, since we're just interested in the builtin "outer" name.
    // Flow mostly irrelevant too, but we'd like to have VariantArray as the outer class, so choose Godot->Rust.
    let flow = FlowDirection::GodotToRust;
    let RustTy::BuiltinIdent {
        ty: outer_class, ..
    } = conv::to_rust_type(godot_name, None, Some(flow), ctx)
    else {
        panic!("Rust type `{godot_name}` categorized wrong")
    };
    let inner_builtin = class.inner_name();

    // Enable `Ex*` builders for builtins to provide consistent API with classes.
    // By default, builders are only placed on outer methods (for methods that are exposed per special_cases.rs), to reduce codegen.
    // However it's possible to enable builders on the inner type, too, which is why both `inner_builders` + `outer_builders` exist.
    #[rustfmt::skip]
    let (
        FnDefinitions { functions: inner_methods, builders: inner_builders },
        FnDefinitions { functions: outer_methods, builders: outer_builders },
    ) = make_builtin_methods(class, variant_shout_name, &class.methods, ctx);

    let imports = util::make_imports();
    let enums = enums::make_enums(&class.enums, &TokenStream::new());
    let special_constructors = make_special_builtin_methods(class.name(), ctx);

    // `mod re_export` needed for builder structs to reference the Inner* type, similar to how classes use `re_export` for the class type.
    let code = quote! {
        #imports

        pub(super) mod re_export {
            use super::*;

            // Do *not* try to limit visibility, because inner types are used in PackedArrayElement::Inner<'a> associated type.
            // Need to redesign that trait otherwise, and split into private/public parts.
            #[doc(hidden)]
            #[repr(transparent)]
            pub struct #inner_builtin<'inner> {
                pub(super) _outer_lifetime: std::marker::PhantomData<&'inner ()>,
                pub(super) sys_ptr: sys::GDExtensionTypePtr,
            }
        }

        impl<'inner> re_export::#inner_builtin<'inner> {
            pub fn from_outer(outer: &#outer_class) -> Self {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: sys::SysPtr::force_mut(outer.sys()),
                }
            }
            #special_constructors
            #inner_methods
        }

        // Re-export Inner* type for convenience.
        pub use re_export::#inner_builtin;

        // Selected APIs appear directly in the outer class.
        impl #outer_class {
            #outer_methods
        }

        #inner_builders
        #outer_builders
        #enums
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    // If any exposed builders are present, generate a sidecar module for the builtin.
    // Do not care about inner Ex* builders, or builtin enums (enums are manually defined).
    let has_sidecar_module = !outer_builders.is_empty();

    GeneratedBuiltin {
        code,
        has_sidecar_module,
    }
}

/// Returns 2 definition packs, one for the `Inner*` methods, and one for those ending up directly in the public-facing (outer) class.
fn make_builtin_methods(
    builtin_class: &BuiltinClass,
    variant_shout_name: &Ident,
    methods: &[BuiltinMethod],
    ctx: &mut Context,
) -> (FnDefinitions, FnDefinitions) {
    // Can't use partition() without allocating new vectors. It can also not be used after map() since condition is lost at that point.

    let inner_defs = methods
        .iter()
        .filter(|&method| !method.is_exposed_in_outer)
        .map(|method| {
            make_builtin_method_definition(builtin_class, variant_shout_name, method, ctx)
        });
    let inner_defs = FnDefinitions::expand(inner_defs);

    let outer_defs = methods
        .iter()
        .filter(|&method| method.is_exposed_in_outer)
        .map(|method| {
            make_builtin_method_definition(builtin_class, variant_shout_name, method, ctx)
        });
    let outer_defs = FnDefinitions::expand(outer_defs);

    (inner_defs, outer_defs)
}

/// Depending on the built-in class, adds custom constructors and methods.
fn make_special_builtin_methods(class_name: &TyName, _ctx: &Context) -> TokenStream {
    if class_name.godot_ty == "Array" {
        quote! {
            pub fn from_outer_typed<T>(outer: &Array<T>) -> Self
                where
                    T: crate::meta::ArrayElement
            {
                Self {
                    _outer_lifetime: std::marker::PhantomData,
                    sys_ptr: sys::SysPtr::force_mut(outer.sys()),
                }
            }
        }
    } else {
        TokenStream::new()
    }
}

/// Get the safety docs of an unsafe method, or `None` if it is safe.
fn method_safety_doc(class_name: &TyName, method: &BuiltinMethod) -> Option<TokenStream> {
    if class_name.godot_ty == "Array" {
        if method.is_generic() {
            return Some(quote! {
               /// # Safety
               /// You must ensure that the returned array fulfils the safety invariants of [`Array`](crate::builtin::Array), this being:
               /// - Any values written to the array must match the runtime type of the array.
               /// - Any values read from the array must be convertible to the type `T`.
               ///
               /// If the safety invariant of `Array` is intact, which it must be for any publicly accessible arrays, then `T` must match
               /// the runtime type of the array. This then implies that both of the conditions above hold. This means that you only need
               /// to keep the above conditions in mind if you are intentionally violating the safety invariant of `Array`.
               ///
               /// In the current implementation, both cases will produce a panic rather than undefined behavior, but this should not be relied upon.
            });
        } else if &method.return_value().type_tokens().to_string() == "VarArray" {
            return Some(quote! {
                /// # Safety
                ///
                /// You must ensure that the returned array fulfils the safety invariants of [`Array`](crate::builtin::Array).
            });
        }
    }

    None
}

fn make_builtin_method_definition(
    builtin_class: &BuiltinClass,
    variant_shout_name: &Ident,
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
        let variant_type = quote! { sys::VariantType::#variant_shout_name };
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

    let ffi_arg_in = if method.is_exposed_in_outer {
        // TODO create dedicated method (in GodotFfi?) and replace similar occurrences everywhere.
        quote! { sys::SysPtr::force_mut(self.sys()) }
    } else {
        quote! { self.sys_ptr }
    };

    let receiver = functions_common::make_receiver(method.qualifier(), ffi_arg_in);
    let object_ptr = &receiver.ffi_arg;

    let maybe_generic_params = method.return_value().generic_params();

    let ptrcall_invocation = quote! {
        let method_bind = sys::builtin_method_table().#fptr_access;


        Signature::<CallParams, CallRet #maybe_generic_params>::out_builtin_ptrcall(
            method_bind,
            #builtin_name_str,
            #method_name_str,
            #object_ptr,
            args
        )
    };

    let varcall_invocation = quote! {
        let method_bind = sys::builtin_method_table().#fptr_access;

        Signature::<CallParams, CallRet>::out_builtin_ptrcall_varargs(
            method_bind,
            #builtin_name_str,
            #method_name_str,
            #object_ptr,
            args,
            varargs
        )
    };

    let safety_doc = method_safety_doc(builtin_class.name(), method);

    functions_common::make_function_definition(
        method,
        &FnCode {
            receiver,
            varcall_invocation,
            ptrcall_invocation,
            is_virtual_required: false,
            is_varcall_fallible: false,
        },
        safety_doc,
        &TokenStream::new(),
    )
}
