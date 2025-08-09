/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::models::domain::{BuiltinVariant, Constructor, ExtensionApi, Operator};

pub fn make_variant_fns(
    api: &ExtensionApi,
    builtin: &BuiltinVariant,
) -> (TokenStream, TokenStream) {
    let (special_decls, special_inits);
    if let Some(builtin_class) = builtin.associated_builtin_class() {
        let (construct_decls, construct_inits) =
            make_construct_fns(api, builtin, &builtin_class.constructors);

        let (destroy_decls, destroy_inits) =
            make_destroy_fns(builtin, builtin_class.has_destructor);

        let (op_eq_decls, op_eq_inits) =
            make_operator_fns(builtin, &builtin_class.operators, "==", "EQUAL");

        let (op_lt_decls, op_lt_inits) =
            make_operator_fns(builtin, &builtin_class.operators, "<", "LESS");

        special_decls = quote! {
            #op_eq_decls
            #op_lt_decls
            #construct_decls
            #destroy_decls
        };
        special_inits = quote! {
            #op_eq_inits
            #op_lt_inits
            #construct_inits
            #destroy_inits
        };
    } else {
        special_decls = TokenStream::new();
        special_inits = TokenStream::new();
    };

    let snake_case = builtin.snake_name();
    let to_variant = format_ident!("{}_to_variant", snake_case);
    let from_variant = format_ident!("{}_from_variant", snake_case);

    let to_variant_str = to_variant.to_string();
    let from_variant_str = from_variant.to_string();

    let variant_type = builtin.sys_variant_type();
    let variant_type = quote! { crate::#variant_type };

    // Field declaration.
    // The target types are uninitialized-ptrs, because Godot performs placement new on those:
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_internal.h#L1535-L1535

    let decl = quote! {
        pub #to_variant: unsafe extern "C" fn(GDExtensionUninitializedVariantPtr, GDExtensionTypePtr),
        pub #from_variant: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, GDExtensionVariantPtr),
        #special_decls
    };

    // Field initialization in new().
    let init = quote! {
        #to_variant: {
            let fptr = unsafe { get_to_variant_fn(#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #to_variant_str)
        },
        #from_variant: {
            let fptr = unsafe { get_from_variant_fn(#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #from_variant_str)
        },
        #special_inits
    };

    (decl, init)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

fn make_construct_fns(
    api: &ExtensionApi,
    builtin: &BuiltinVariant,
    constructors: &[Constructor],
) -> (TokenStream, TokenStream) {
    if constructors.is_empty() {
        return (TokenStream::new(), TokenStream::new());
    };

    // Constructor vec layout:
    //   [0]: default constructor
    //   [1]: copy constructor
    //   [2]: (optional) typically the most common conversion constructor (e.g. StringName -> String)
    //  rest: (optional) other conversion constructors and multi-arg constructors (e.g. Vector3(x, y, z))

    // Sanity checks -- ensure format is as expected.
    for (i, c) in constructors.iter().enumerate() {
        assert_eq!(i, c.index);
    }

    assert!(
        constructors[0].raw_parameters.is_empty(),
        "default constructor at index 0 must have no parameters"
    );

    let args = &constructors[1].raw_parameters;
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].name, "from");
    assert_eq!(args[0].type_, builtin.godot_original_name());

    let builtin_snake_name = builtin.snake_name();
    let variant_type = builtin.sys_variant_type();

    let construct_default = format_ident!("{builtin_snake_name}_construct_default");
    let construct_copy = format_ident!("{builtin_snake_name}_construct_copy");
    let construct_default_str = construct_default.to_string();
    let construct_copy_str = construct_copy.to_string();

    let (construct_extra_decls, construct_extra_inits) =
        make_extra_constructors(api, builtin, constructors);

    // Target types are uninitialized pointers, because Godot uses placement-new for raw pointer constructions. Callstack:
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/extension/gdextension_interface.cpp#L511
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.cpp#L299
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.cpp#L36
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.h#L267
    // https://github.com/godotengine/godot/blob/b40b35fb39f0d0768d7ec2976135adffdce1b96d/core/variant/variant_construct.h#L50
    let decls = quote! {
        pub #construct_default: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
        pub #construct_copy: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
        #(
            #construct_extra_decls
        )*
    };

    let inits = quote! {
        #construct_default: {
            let fptr = unsafe { get_construct_fn(crate::#variant_type, 0i32) };
            crate::validate_builtin_lifecycle(fptr, #construct_default_str)
        },
        #construct_copy: {
            let fptr = unsafe { get_construct_fn(crate::#variant_type, 1i32) };
            crate::validate_builtin_lifecycle(fptr, #construct_copy_str)
        },
        #(
            #construct_extra_inits
        )*
    };

    (decls, inits)
}

/// Lists special cases for useful constructors
fn make_extra_constructors(
    api: &ExtensionApi,
    builtin: &BuiltinVariant,
    constructors: &[Constructor],
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut extra_decls = Vec::with_capacity(constructors.len() - 2);
    let mut extra_inits = Vec::with_capacity(constructors.len() - 2);
    let variant_type = builtin.sys_variant_type();

    for (i, ctor) in constructors.iter().enumerate().skip(2) {
        let args = &ctor.raw_parameters;
        assert!(
            !args.is_empty(),
            "custom constructors must have at least 1 parameter"
        );

        let type_name = builtin.snake_name();
        let construct_custom = if args.len() == 1 && args[0].name == "from" {
            // Conversion constructor is named according to the source type:
            // String(NodePath from) => string_from_node_path

            let arg_type = api.builtin_by_original_name(&args[0].type_).snake_name();
            format_ident!("{type_name}_from_{arg_type}")
        } else {
            // Type-specific constructor is named according to the argument names:
            // Vector3(float x, float y, float z) => vector3_from_x_y_z
            let mut arg_names = args
                .iter()
                .fold(String::new(), |acc, arg| acc + &arg.name + "_");
            arg_names.pop(); // remove trailing '_'
            format_ident!("{type_name}_from_{arg_names}")
        };

        let construct_custom_str = construct_custom.to_string();
        extra_decls.push(quote! {
                pub #construct_custom: unsafe extern "C" fn(GDExtensionUninitializedTypePtr, *const GDExtensionConstTypePtr),
            });

        let i = i as i32;
        extra_inits.push(quote! {
            #construct_custom: {
                let fptr = unsafe { get_construct_fn(crate::#variant_type, #i) };
                crate::validate_builtin_lifecycle(fptr, #construct_custom_str)
            },
        });
    }

    (extra_decls, extra_inits)
}

fn make_destroy_fns(builtin: &BuiltinVariant, has_destructor: bool) -> (TokenStream, TokenStream) {
    if !has_destructor {
        return (TokenStream::new(), TokenStream::new());
    }

    let destroy = format_ident!("{}_destroy", builtin.snake_name());
    let destroy_str = destroy.to_string();
    let variant_type = builtin.sys_variant_type();

    let decls = quote! {
        pub #destroy: unsafe extern "C" fn(GDExtensionTypePtr),
    };

    let inits = quote! {
        #destroy: {
            let fptr = unsafe { get_destroy_fn(crate::#variant_type) };
            crate::validate_builtin_lifecycle(fptr, #destroy_str)
        },
    };

    (decls, inits)
}

fn make_operator_fns(
    builtin: &BuiltinVariant,
    operators: &[Operator],
    json_symbol: &str,
    sys_name: &str,
) -> (TokenStream, TokenStream) {
    // If there are no operators for that builtin type, or none of the operator matches symbol, then don't generate function.
    if operators.is_empty() || !operators.iter().any(|op| op.symbol == json_symbol) {
        return (TokenStream::new(), TokenStream::new());
    }

    let operator = format_ident!(
        "{}_operator_{}",
        builtin.snake_name(),
        sys_name.to_ascii_lowercase()
    );
    let operator_str = operator.to_string();

    let variant_type = builtin.sys_variant_type();
    let variant_type = quote! { crate::#variant_type };
    let sys_ident = format_ident!("GDEXTENSION_VARIANT_OP_{}", sys_name);

    // Field declaration.
    let decl = quote! {
        pub #operator: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    };

    // Field initialization in new().
    let init = quote! {
        #operator: {
            let fptr = unsafe { get_operator_fn(crate::#sys_ident, #variant_type, #variant_type) };
            crate::validate_builtin_lifecycle(fptr, #operator_str)
        },
    };

    (decl, init)
}
