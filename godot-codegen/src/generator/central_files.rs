/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::conv;
use crate::generator::{enums, gdext_build_struct};
use crate::models::domain::{Enumerator, ExtensionApi};
use crate::util::ident;
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};

pub fn make_sys_central_code(api: &ExtensionApi, ctx: &mut Context) -> TokenStream {
    let VariantEnums {
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_ord,
        variant_op_enumerators_pascal,
        variant_op_enumerators_ord,
        ..
    } = make_variant_enums(api, ctx);

    let build_config_struct = gdext_build_struct::make_gdext_build_struct(&api.godot_version);
    let [opaque_32bit, opaque_64bit] = make_opaque_types(api);

    quote! {
        use crate::{GDExtensionVariantOperator, GDExtensionVariantType};

        #[cfg(target_pointer_width = "32")]
        pub mod types {
            #(#opaque_32bit)*
        }
        #[cfg(target_pointer_width = "64")]
        pub mod types {
            #(#opaque_64bit)*
        }


        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #build_config_struct

        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantType {
            Nil = 0,
            #(
                #variant_ty_enumerators_pascal = #variant_ty_enumerators_ord,
            )*
        }

        impl VariantType {
            #[doc(hidden)]
            pub fn from_sys(enumerator: GDExtensionVariantType) -> Self {
                // Annoying, but only stable alternative is transmute(), which dictates enum size.
                match enumerator {
                    0 => Self::Nil,
                    #(
                        #variant_ty_enumerators_ord => Self::#variant_ty_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant type {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> GDExtensionVariantType {
                self as _
            }
        }

        // ----------------------------------------------------------------------------------------------------------------------------------------------

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantOperator {
            #(
                #variant_op_enumerators_pascal = #variant_op_enumerators_ord,
            )*
        }

        impl VariantOperator {
            #[doc(hidden)]
            pub fn from_sys(enumerator: GDExtensionVariantOperator) -> Self {
                match enumerator {
                    #(
                        #variant_op_enumerators_ord => Self::#variant_op_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant operator {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> GDExtensionVariantOperator {
                self as _
            }
        }
    }
}

pub fn make_core_central_code(api: &ExtensionApi, ctx: &mut Context) -> TokenStream {
    let VariantEnums {
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_rust,
        ..
    } = make_variant_enums(api, ctx);

    let global_enum_defs = make_global_enums(api);

    // TODO impl Clone, Debug, PartialEq, PartialOrd, Hash for VariantDispatch
    // TODO could use try_to().unwrap_unchecked(), since type is already verified. Also directly overload from_variant().
    // But this requires that all the variant types support this.
    quote! {
        use crate::builtin::*;
        use crate::engine::Object;
        use crate::obj::Gd;

        #[allow(dead_code)]
        pub enum VariantDispatch {
            Nil,
            #(
                #variant_ty_enumerators_pascal(#variant_ty_enumerators_rust),
            )*
        }

        impl VariantDispatch {
            pub fn from_variant(variant: &Variant) -> Self {
                match variant.get_type() {
                    VariantType::Nil => Self::Nil,
                    #(
                        VariantType::#variant_ty_enumerators_pascal
                            => Self::#variant_ty_enumerators_pascal(variant.to::<#variant_ty_enumerators_rust>()),
                    )*
                }
            }
        }

        impl std::fmt::Debug for VariantDispatch {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Nil => write!(f, "null"),
                    #(
                        Self::#variant_ty_enumerators_pascal(v) => write!(f, "{v:?}"),
                    )*
                }
            }
        }

        /// Global enums and constants.
        ///
        /// A list of global-scope enumerated constants.
        /// For global built-in functions, check out the [`utilities` module][crate::engine::utilities].
        ///
        /// See also [Godot docs for `@GlobalScope`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#enumerations).
        pub mod global {
            use crate::sys;
            #( #global_enum_defs )*
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

struct VariantEnums {
    variant_ty_enumerators_pascal: Vec<Ident>,
    variant_ty_enumerators_rust: Vec<TokenStream>,
    variant_ty_enumerators_ord: Vec<Literal>,
    variant_op_enumerators_pascal: Vec<Ident>,
    variant_op_enumerators_ord: Vec<Literal>,
}

fn collect_variant_operators(api: &ExtensionApi) -> Vec<&Enumerator> {
    let variant_operator_enum = api
        .global_enums
        .iter()
        .find(|e| &e.name == "VariantOperator") // in JSON: "Variant.Operator"
        .expect("missing enum for VariantOperator in JSON");

    variant_operator_enum.enumerators.iter().collect()
}

fn make_opaque_types(api: &ExtensionApi) -> [Vec<TokenStream>; 2] {
    let mut opaque_types = [Vec::new(), Vec::new()];

    for b in api.builtin_sizes.iter() {
        let index = b.config.is_64bit() as usize;
        let type_def = make_opaque_type(&b.builtin_original_name, b.size);

        opaque_types[index].push(type_def);
    }

    opaque_types
}

fn make_opaque_type(godot_original_name: &str, size: usize) -> TokenStream {
    let name = conv::to_pascal_case(godot_original_name);
    let (first, rest) = name.split_at(1);

    // Capitalize: "int" -> "Int".
    let ident = format_ident!("Opaque{}{}", first.to_ascii_uppercase(), rest);
    quote! {
        pub type #ident = crate::opaque::Opaque<#size>;
    }
}

fn make_variant_enums(api: &ExtensionApi, ctx: &mut Context) -> VariantEnums {
    let variant_operators = collect_variant_operators(api);

    // Generate builtin methods, now with info for all types available.
    // Separate vectors because that makes usage in quote! easier.
    let len = api.builtins.len();

    let mut result = VariantEnums {
        variant_ty_enumerators_pascal: Vec::with_capacity(len),
        variant_ty_enumerators_rust: Vec::with_capacity(len),
        variant_ty_enumerators_ord: Vec::with_capacity(len),
        variant_op_enumerators_pascal: Vec::new(),
        variant_op_enumerators_ord: Vec::new(),
    };

    // Note: NIL is not part of this iteration, it will be added manually.
    for builtin in api.builtins.iter() {
        let original_name = builtin.godot_original_name();
        let rust_ty = conv::to_rust_type(original_name, None, ctx);
        let pascal_case = conv::to_pascal_case(original_name);
        let ord = builtin.unsuffixed_ord_lit();

        result
            .variant_ty_enumerators_pascal
            .push(ident(&pascal_case));
        result
            .variant_ty_enumerators_rust
            .push(rust_ty.to_token_stream());
        result.variant_ty_enumerators_ord.push(ord);
    }

    for op in variant_operators {
        let pascal_name = conv::to_pascal_case(&op.name.to_string());

        let enumerator_name = if pascal_name == "Module" {
            ident("Modulo")
        } else {
            ident(&pascal_name)
        };

        result.variant_op_enumerators_pascal.push(enumerator_name);
        result
            .variant_op_enumerators_ord
            .push(op.value.unsuffixed_lit());
    }

    result
}

fn make_global_enums(api: &ExtensionApi) -> Vec<TokenStream> {
    let mut global_enum_defs = vec![];

    for enum_ in api.global_enums.iter() {
        // Skip those enums which are already manually handled.
        if enum_.name == "VariantType" || enum_.name == "VariantOperator" {
            continue;
        }

        let def = enums::make_enum_definition(enum_);
        global_enum_defs.push(def);
    }

    global_enum_defs
}
