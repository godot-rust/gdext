/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::ParseResult;
use crate::class::{
    FuncDefinition, ImplContext, ImplKind, ProcessedFns, make_method_registration,
    process_godot_constants, process_godot_fns,
};
use crate::util::{bail, extract_typename};

pub fn attribute_godot_dyn(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let venial::Item::Impl(mut decl) = input_decl else {
        return bail!(
            input_decl,
            "#[godot_dyn] can only be applied on impl blocks",
        );
    };

    if decl.impl_generic_params.is_some() {
        bail!(
            &decl,
            "#[godot_dyn] does not support lifetimes or generic parameters",
        )?;
    }

    let Some(trait_path) = decl.trait_ty.clone() else {
        return bail!(
            &decl,
            "#[godot_dyn] requires a trait; it cannot be applied to inherent impl blocks",
        );
    };

    let class_path = decl.self_ty.clone();
    let class_name = extract_class_name(&class_path)?;

    // Removes #[func] attributes from the impl block, and gathers the information needed to register those methods in Godot.
    let ProcessedFns {
        func_definitions: funcs,
        signal_definitions: _, // #[signal] is rejected inside #[godot_dyn].
        extra_inherent_fns,
    } = process_godot_fns(&class_name, &mut decl, ImplKind::DynTrait)?;

    let consts = process_godot_constants(&mut decl)?;
    if let Some(constant) = consts.first() {
        return bail!(
            &constant.raw_constant,
            "#[constant] is not supported in #[godot_dyn] impl blocks; declare it in the #[godot_api] impl block",
        );
    }

    // Generated code refers to the class by bare name (e.g. inside FFI callbacks), so a qualified or generic path would not resolve.
    if !funcs.is_empty() && !is_plain_class_path(&class_path) {
        return bail!(
            &class_path,
            "#[godot_dyn] with #[func] methods requires the class to be specified by its plain name (no qualified path, no generic arguments)",
        );
    }

    let mut associated_types = vec![];
    for impl_member in &decl.body_items {
        let venial::ImplMember::AssocType(associated_type) = impl_member else {
            continue;
        };
        let Some(type_expr) = &associated_type.initializer_ty else {
            continue;
        };
        let type_name = &associated_type.name;
        associated_types.push(quote! { #type_name = #type_expr })
    }

    let assoc_type_constraints = if associated_types.is_empty() {
        TokenStream::new()
    } else {
        quote! { < #(#associated_types),* > }
    };

    let prv = quote! { ::godot::private };

    // Early-bound default implementations of #[func(virtual)] methods are not part of the trait, so they need their own inherent impl block.
    let extra_impl = if extra_inherent_fns.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            impl #class_path {
                #(#extra_inherent_fns)*
            }
        }
    };

    let func_registration = make_func_registration(&class_name, &class_path, &trait_path, funcs)?;

    let new_code = quote! {
        #decl
        #extra_impl

        impl ::godot::obj::AsDyn<dyn #trait_path #assoc_type_constraints> for #class_path {
            fn dyn_upcast(&self) -> &(dyn #trait_path #assoc_type_constraints + 'static) {
                self
            }

            fn dyn_upcast_mut(&mut self) -> &mut (dyn #trait_path #assoc_type_constraints + 'static) {
                self
            }
        }

        ::godot::sys::shard_add!(#prv::__GODOT_SHARD_REGISTRY; #prv::ClassShard::new::<#class_path>(
            #prv::ShardItem::DynTraitImpl(#prv::DynTraitImpl::new::<#class_path, dyn #trait_path #assoc_type_constraints>()))
        );

        #func_registration
    };

    Ok(new_code)
}

/// Generates the code registering all `#[func]` methods of this trait impl with Godot.
///
/// The registration functions are appended to the storage declared by the class's primary `#[godot_api]` impl block, i.e. the same mechanism
/// used by secondary impl blocks. Trait methods are registered under their plain Godot name, so GDScript can call them like any other method.
fn make_func_registration(
    class_name: &Ident,
    class_path: &venial::TypeExpr,
    trait_path: &venial::TypeExpr,
    funcs: Vec<FuncDefinition>,
) -> ParseResult<TokenStream> {
    if funcs.is_empty() {
        return Ok(TokenStream::new());
    }

    let prv = quote! { ::godot::private };
    let context = ImplContext::UserTrait(trait_path);

    let docs = crate::docs::make_trait_docs_registration(&funcs, &[], &[], class_name, &prv);

    let method_registrations: Vec<TokenStream> = funcs
        .into_iter()
        .map(|func_def| make_method_registration(class_name, func_def, context))
        .collect::<ParseResult<Vec<TokenStream>>>()?;

    let code = quote! {
        // Better diagnostic than the "no function `__registration_storage`" error, if the class has no primary #[godot_api] impl block.
        const _: () = {
            fn __gdext_requires_godot_api<T: ::godot::obj::cap::ImplementsGodotApi>() {}

            fn __gdext_assert() {
                __gdext_requires_godot_api::<#class_path>();
            }
        };

        ::godot::sys::shard_execute_pre_main!({
            let mut guard = #class_path::__registration_storage().lock().unwrap();

            guard.0.push(|| {
                #( #method_registrations )*
            });
        });

        #docs
    };

    Ok(code)
}

/// Extracts the class name from the `Self` type of the impl block.
fn extract_class_name(class_path: &venial::TypeExpr) -> ParseResult<Ident> {
    match extract_typename(class_path) {
        Some(segment) => Ok(segment.ident),
        None => bail!(class_path, "#[godot_dyn] requires Self type to be a path"),
    }
}

/// Whether the `Self` type is a bare class name, i.e. no qualified path and no generic arguments.
fn is_plain_class_path(class_path: &venial::TypeExpr) -> bool {
    class_path
        .as_path()
        .is_some_and(|path| path.segments.len() == 1 && path.segments[0].generic_args.is_none())
}
