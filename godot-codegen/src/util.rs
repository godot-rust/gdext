/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::api_parser::Enum;
use crate::special_cases::is_builtin_scalar;
use crate::{Context, ModName, RustTy, TyName};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

pub fn make_enum_definition(enum_: &Enum) -> TokenStream {
    // TODO enums which have unique ords could be represented as Rust enums
    // This would allow exhaustive matches (or at least auto-completed matches + #[non_exhaustive]). But even without #[non_exhaustive],
    // this might be a forward compatibility hazard, if Godot deprecates enumerators and adds new ones with existing ords.

    let enum_name = ident(&enum_.name);

    let values = &enum_.values;
    let mut enumerators = Vec::with_capacity(values.len());
    // let mut matches = Vec::with_capacity(values.len());
    let mut unique_ords = Vec::with_capacity(values.len());

    for enumerator in values {
        let name = make_enumerator_name(&enumerator.name, &enum_.name);
        let ordinal = Literal::i32_unsuffixed(enumerator.value);

        enumerators.push(quote! {
            pub const #name: Self = Self { ord: #ordinal };
        });
        // matches.push(quote! {
        //     #ordinal => Some(Self::#name),
        // });
        unique_ords.push(enumerator.value);
    }

    // They are not necessarily in order
    unique_ords.sort();
    unique_ords.dedup();

    let bitfield_ops = if enum_.is_bitfield {
        let tokens = quote! {
            // impl #enum_name {
            //     pub const UNSET: Self = Self { ord: 0 };
            // }
            impl std::ops::BitOr for #enum_name {
                type Output = Self;

                fn bitor(self, rhs: Self) -> Self::Output {
                    Self { ord: self.ord | rhs.ord }
                }
            }
        };

        Some(tokens)
    } else {
        None
    };

    // Enumerator ordinal stored as i32, since that's enough to hold all current values and the default repr in C++.
    // Public interface is i64 though, for consistency (and possibly forward compatibility?).
    // TODO maybe generalize GodotFfi over EngineEnum trait
    quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
        pub struct #enum_name {
            ord: i32
        }
        impl #enum_name {
            #(
                #enumerators
            )*
        }
        impl crate::obj::EngineEnum for #enum_name {
            // fn try_from_ord(ord: i32) -> Option<Self> {
            //     match ord {
            //         #(
            //             #matches
            //         )*
            //         _ => None,
            //     }
            // }
            fn try_from_ord(ord: i32) -> Option<Self> {
                match ord {
                    #( ord @ #unique_ords )|* => Some(Self { ord }),
                    _ => None,
                }
            }
            fn ord(self) -> i32 {
                self.ord
            }
        }
        impl sys::GodotFfi for #enum_name {
            sys::ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
        }
        #bitfield_ops
    }
}

fn make_enum_name(enum_name: &str) -> Ident {
    // TODO clean up enum name

    ident(enum_name)
}

fn make_enumerator_name(enumerator_name: &str, _enum_name: &str) -> Ident {
    // TODO strip prefixes of `enum_name` appearing in `enumerator_name`
    // tons of variantions, see test cases in lib.rs

    ident(enumerator_name)
}

pub fn to_snake_case(class_name: &str) -> String {
    use heck::ToSnakeCase;

    // Special cases
    #[allow(clippy::single_match)]
    match class_name {
        "JSONRPC" => return "json_rpc".to_string(),
        _ => {}
    }

    class_name
        .replace("2D", "_2d")
        .replace("3D", "_3d")
        .replace("GDNative", "Gdnative")
        .replace("GDExtension", "Gdextension")
        .to_snake_case()
}

pub fn to_pascal_case(class_name: &str) -> String {
    use heck::ToPascalCase;

    // Special cases
    #[allow(clippy::single_match)]
    match class_name {
        "JSONRPC" => return "JsonRpc".to_string(),
        _ => {}
    }

    class_name
        .to_pascal_case()
        .replace("GdExtension", "GDExtension")
        .replace("GdNative", "GDNative")
}

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

#[rustfmt::skip]
pub fn safe_ident(s: &str) -> Ident {
    // See also: https://doc.rust-lang.org/reference/keywords.html
    match s {
        // Lexer
        | "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false" | "fn" | "for" | "if"
        | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
        | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while"

        // Lexer 2018+
        | "async" | "await" | "dyn"

        // Reserved
        | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"

        // Reserved 2018+
        | "try"
           => format_ident!("{}_", s),

         _ => ident(s)
    }
}

fn to_hardcoded_rust_type(ty: &str) -> Option<&str> {
    let result = match ty {
        "int" => "i64",
        "float" => "f64",
        "String" => "GodotString",
        "Array" => "VariantArray",
        //"enum::Error" => "GodotError",
        "enum::Variant.Type" => "VariantType",
        "enum::Variant.Operator" => "VariantOperator",
        "enum::Vector3.Axis" => "Vector3Axis",
        _ => return None,
    };
    Some(result)
}

/// Maps an _input_ type from the Godot JSON to the corresponding Rust type (wrapping some sort of a token stream).
///
/// Uses an internal cache (via `ctx`), as several types are ubiquitous.
// TODO take TyName as input
pub(crate) fn to_rust_type(ty: &str, ctx: &mut Context<'_>) -> RustTy {
    // Separate find + insert slightly slower, but much easier with lifetimes
    // The insert path will be hit less often and thus doesn't matter
    if let Some(rust_ty) = ctx.find_rust_type(ty) {
        rust_ty.clone()
    } else {
        let rust_ty = to_rust_type_uncached(ty, ctx);
        ctx.insert_rust_type(ty, rust_ty.clone());
        rust_ty
    }
}

fn to_rust_type_uncached(ty: &str, ctx: &mut Context) -> RustTy {
    /// Transforms a Godot class/builtin/enum IDENT (without `::` or other syntax) to a Rust one
    fn rustify_ty(ty: &str) -> Ident {
        if is_builtin_scalar(ty) {
            ident(ty)
        } else {
            TyName::from_godot(ty).rust_ty
        }
    }

    if let Some(hardcoded) = to_hardcoded_rust_type(ty) {
        return RustTy::BuiltinIdent(ident(hardcoded));
    }

    let qualified_enum = ty
        .strip_prefix("enum::")
        .or_else(|| ty.strip_prefix("bitfield::"));

    if let Some(qualified_enum) = qualified_enum {
        return if let Some((class, enum_)) = qualified_enum.split_once('.') {
            // Class-local enum
            let module = ModName::from_godot(class);
            let enum_ty = make_enum_name(enum_);

            RustTy::EngineEnum {
                tokens: quote! { #module::#enum_ty },
                surrounding_class: Some(class.to_string()),
            }
        } else {
            // Global enum
            let enum_ty = make_enum_name(qualified_enum);

            RustTy::EngineEnum {
                tokens: quote! { global::#enum_ty },
                surrounding_class: None,
            }
        };
    } else if let Some(packed_arr_ty) = ty.strip_prefix("Packed") {
        // Don't trigger on PackedScene ;P
        if packed_arr_ty.ends_with("Array") {
            return RustTy::BuiltinIdent(rustify_ty(ty));
        }
    } else if let Some(elem_ty) = ty.strip_prefix("typedarray::") {
        if let Some(_packed_arr_ty) = elem_ty.strip_prefix("Packed") {
            return RustTy::BuiltinIdent(rustify_ty(elem_ty));
        }

        let rust_elem_ty = to_rust_type(elem_ty, ctx);
        return if ctx.is_builtin(elem_ty) {
            RustTy::BuiltinArray(quote! { Array<#rust_elem_ty> })
        } else {
            RustTy::EngineArray {
                tokens: quote! { Array<#rust_elem_ty> },
                elem_class: elem_ty.to_string(),
            }
        };
    }

    // Note: do not check if it's a known engine class, because that will not work in minimal mode (since not all classes are stored)
    if ctx.is_builtin(ty) {
        // Unchanged
        RustTy::BuiltinIdent(rustify_ty(ty))
    } else {
        let ty = rustify_ty(ty);
        RustTy::EngineClass {
            tokens: quote! { Gd<#ty> },
            class: ty.to_string(),
        }
    }
}
