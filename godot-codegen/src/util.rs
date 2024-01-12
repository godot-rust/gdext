/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::api_parser::{
    BuiltinClassMethod, Class, ClassConstant, ClassMethod, ConstValue, Enum, UtilityFunction,
};
use crate::{conv, RustTy, TyName};

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use std::fmt;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct NativeStructuresField {
    pub field_type: String,
    pub field_name: String,
}

/// At which stage a class function pointer is loaded.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum ClassCodegenLevel {
    Servers,
    Scene,
    Editor,
}

impl ClassCodegenLevel {
    pub fn with_tables() -> [Self; 3] {
        [Self::Servers, Self::Scene, Self::Editor]
    }

    pub fn table_global_getter(self) -> Ident {
        format_ident!("class_{}_api", self.lower())
    }

    pub fn table_file(self) -> String {
        format!("table_{}_classes.rs", self.lower())
    }

    pub fn table_struct(self) -> Ident {
        format_ident!("Class{}MethodTable", self.upper())
    }

    pub fn lower(self) -> &'static str {
        match self {
            Self::Servers => "servers",
            Self::Scene => "scene",
            Self::Editor => "editor",
        }
    }

    fn upper(self) -> &'static str {
        match self {
            Self::Servers => "Servers",
            Self::Scene => "Scene",
            Self::Editor => "Editor",
        }
    }

    pub fn to_init_level(self) -> TokenStream {
        match self {
            Self::Servers => quote! { crate::init::InitLevel::Servers },
            Self::Scene => quote! { crate::init::InitLevel::Scene },
            Self::Editor => quote! { crate::init::InitLevel::Editor },
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Lookup key for indexed method tables.
// Could potentially save a lot of string allocations with lifetimes.
// See also crate::lazy_keys.
#[derive(Eq, PartialEq, Hash)]
pub(crate) enum MethodTableKey {
    ClassMethod {
        api_level: ClassCodegenLevel,
        class_ty: TyName,
        method_name: String,
    },
    BuiltinMethod {
        builtin_ty: TyName,
        method_name: String,
    },
    /*BuiltinLifecycleMethod {
        builtin_ty: TyName,
        method_name: String,
    },
    UtilityFunction {
        function_name: String,
    },*/
}

impl MethodTableKey {
    /// Maps the method table key to a "category", meaning a distinct method table.
    ///
    /// Categories have independent address spaces for indices, meaning they begin again at 0 for each new category.
    pub fn category(&self) -> String {
        match self {
            MethodTableKey::ClassMethod { api_level, .. } => format!("class.{}", api_level.lower()),
            MethodTableKey::BuiltinMethod { .. } => "builtin".to_string(),
            // MethodTableKey::BuiltinLifecycleMethod { .. } => "builtin.lifecycle".to_string(),
            // MethodTableKey::UtilityFunction { .. } => "utility".to_string(),
        }
    }
}

impl fmt::Debug for MethodTableKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MethodTableKey::ClassMethod {
                api_level: _,
                class_ty: class_name,
                method_name,
            } => write!(f, "ClassMethod({}.{})", class_name.godot_ty, method_name),
            MethodTableKey::BuiltinMethod {
                builtin_ty: variant_type,
                method_name,
            } => write!(
                f,
                "BuiltinMethod({}.{})",
                variant_type.godot_ty, method_name
            ),
            /*MethodTableKey::BuiltinLifecycleMethod {
                builtin_ty: variant_type,
                method_name,
            } => write!(
                f,
                "BuiltinLifecycleMethod({}.{})",
                variant_type.godot_ty, method_name
            ),
            MethodTableKey::UtilityFunction { function_name } => {
                write!(f, "UtilityFunction({})", function_name)
            }*/
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Small utility that turns an optional vector (often encountered as JSON deserialization type) into a slice.
pub(crate) fn option_as_slice<T>(option: &Option<Vec<T>>) -> &[T] {
    option.as_ref().map_or(&[], Vec::as_slice)
}

pub(crate) fn make_imports() -> TokenStream {
    quote! {
        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::builtin::meta::{ClassName, PtrcallReturnUnit, PtrcallReturnT, PtrcallReturnOptionGdT, PtrcallSignatureTuple, VarcallSignatureTuple};
        use crate::engine::native::*;
        use crate::engine::Object;
        use crate::obj::Gd;
        use crate::sys::GodotFfi as _;
    }
}

// Use &ClassMethod instead of &str, to make sure it's the original Godot name and no rename.
pub(crate) fn make_class_method_ptr_name(class_ty: &TyName, method: &ClassMethod) -> Ident {
    format_ident!(
        "{}__{}",
        conv::to_snake_case(&class_ty.godot_ty),
        method.name
    )
}

pub(crate) fn make_builtin_method_ptr_name(
    builtin_ty: &TyName,
    method: &BuiltinClassMethod,
) -> Ident {
    format_ident!(
        "{}__{}",
        conv::to_snake_case(&builtin_ty.godot_ty),
        method.name
    )
}

pub(crate) fn make_utility_function_ptr_name(function: &UtilityFunction) -> Ident {
    safe_ident(&function.name)
}

#[cfg(since_api = "4.2")]
pub fn make_string_name(identifier: &str) -> TokenStream {
    let lit = Literal::byte_string(format!("{identifier}\0").as_bytes());
    quote! {
        StringName::from_latin1_with_nul(#lit)
    }
}

#[cfg(before_api = "4.2")]
pub fn make_string_name(identifier: &str) -> TokenStream {
    quote! {
        StringName::from(#identifier)
    }
}
pub fn make_sname_ptr(identifier: &str) -> TokenStream {
    quote! {
        string_names.fetch(#identifier)
    }
}

pub fn get_api_level(class: &Class) -> ClassCodegenLevel {
    // Work around wrong classification in https://github.com/godotengine/godot/issues/86206.
    fn override_editor(class_name: &str) -> bool {
        cfg!(before_api = "4.3")
            && matches!(
                class_name,
                "ResourceImporterOggVorbis" | "ResourceImporterMP3"
            )
    }

    if class.name.ends_with("Server") {
        ClassCodegenLevel::Servers
    } else if class.api_type == "editor" || override_editor(&class.name) {
        ClassCodegenLevel::Editor
    } else if class.api_type == "core" {
        ClassCodegenLevel::Scene
    } else {
        panic!(
            "class {} has unknown API type {}",
            class.name, class.api_type
        )
    }
}

pub fn make_enum_definition(enum_: &Enum, class_name: Option<&str>) -> TokenStream {
    // TODO enums which have unique ords could be represented as Rust enums
    // This would allow exhaustive matches (or at least auto-completed matches + #[non_exhaustive]). But even without #[non_exhaustive],
    // this might be a forward compatibility hazard, if Godot deprecates enumerators and adds new ones with existing ords.

    let enum_name_str = conv::make_enum_name_str(&enum_.name);
    let enum_name = ident(&enum_name_str);

    // TODO remove once deprecated is removed.
    let deprecated_enum_decl = if enum_name != enum_.name {
        let deprecated_enum_name = ident(&enum_.name);
        let msg = format!("Renamed to `{enum_name}`.");
        quote! {
            #[deprecated = #msg]
            pub type #deprecated_enum_name = #enum_name;
        }
    } else {
        TokenStream::new()
    };

    let godot_enumerators = &enum_.values;
    let mut enumerators = Vec::with_capacity(godot_enumerators.len());
    let mut deprecated_enumerators = Vec::new();

    // This is only used for enum ords (i32), not bitfield flags (u64).
    let mut unique_ords = Vec::with_capacity(godot_enumerators.len());

    let rust_enumerator_names = {
        let original_enumerator_names = enum_.values.iter().map(|e| e.name.as_str()).collect();
        conv::make_enumerator_names(class_name, &enum_name_str, original_enumerator_names)
    };

    for (enumerator, enumerator_name) in godot_enumerators.iter().zip(rust_enumerator_names) {
        let ordinal_lit = if enum_.is_bitfield {
            let bitfield_ord: u64 = enumerator.to_bitfield_ord();
            make_bitfield_flag_ord(bitfield_ord)
        } else {
            let enum_ord: i32 = enumerator.to_enum_ord();
            unique_ords.push(enum_ord);
            make_enumerator_ord(enum_ord)
        };

        let godot_name_str = &enumerator.name;
        let doc_alias = if enumerator_name == godot_name_str {
            TokenStream::new()
        } else {
            // Godot and Rust names differ -> add doc alias for searchability.
            let msg = format!("Renamed to `{}`.", enumerator_name);
            let deprecated_ident = ident(godot_name_str);

            // For now, list previous identifier at the end.
            deprecated_enumerators.push(quote! {
                #[deprecated = #msg]
                pub const #deprecated_ident: Self = Self { ord: #ordinal_lit };
            });

            quote! {
                #[doc(alias = #godot_name_str)]
            }
        };

        enumerators.push(quote! {
            #doc_alias
            pub const #enumerator_name: Self = Self { ord: #ordinal_lit };
        });
    }

    enumerators.extend(deprecated_enumerators);

    let mut derives = vec!["Copy", "Clone", "Eq", "PartialEq", "Hash", "Debug"];

    if enum_.is_bitfield {
        derives.push("Default");
    }

    let derives = derives.into_iter().map(ident);

    let index_enum_impl = if enum_.is_bitfield {
        // Bitfields don't implement IndexEnum.
        TokenStream::new()
    } else {
        // Enums implement IndexEnum only if they are "index-like" (see docs).
        if let Some(enum_max) = try_count_index_enum(enum_) {
            quote! {
                impl crate::obj::IndexEnum for #enum_name {
                    const ENUMERATOR_COUNT: usize = #enum_max;
                }
            }
        } else {
            TokenStream::new()
        }
    };

    let bitfield_ops;
    let self_as_trait;
    let engine_impl;
    let enum_ord_type;

    if enum_.is_bitfield {
        bitfield_ops = quote! {
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
        enum_ord_type = quote! { u64 };
        self_as_trait = quote! { <Self as crate::obj::EngineBitfield> };
        engine_impl = quote! {
            impl crate::obj::EngineBitfield for #enum_name {
                fn try_from_ord(ord: u64) -> Option<Self> {
                    Some(Self { ord })
                }

                fn ord(self) -> u64 {
                    self.ord
                }
            }
        };
    } else {
        // Ordinals are not necessarily in order.
        unique_ords.sort();
        unique_ords.dedup();

        bitfield_ops = TokenStream::new();
        enum_ord_type = quote! { i32 };
        self_as_trait = quote! { <Self as crate::obj::EngineEnum> };
        engine_impl = quote! {
            impl crate::obj::EngineEnum for #enum_name {
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
        };
    };

    // Enumerator ordinal stored as i32, since that's enough to hold all current values and the default repr in C++.
    // Public interface is i64 though, for consistency (and possibly forward compatibility?).
    // Bitfield ordinals are stored as u64. See also: https://github.com/godotengine/godot-cpp/pull/1320
    quote! {
        #deprecated_enum_decl

        #[repr(transparent)]
        #[derive(#( #derives ),*)]
        pub struct #enum_name {
            ord: #enum_ord_type
        }
        impl #enum_name {
            #(
                #enumerators
            )*
        }

        #engine_impl
        #index_enum_impl
        #bitfield_ops

        impl crate::builtin::meta::GodotConvert for #enum_name {
            type Via = #enum_ord_type;
        }

        impl crate::builtin::meta::ToGodot for #enum_name {
            fn to_godot(&self) -> Self::Via {
                #self_as_trait::ord(*self)
            }
        }

        impl crate::builtin::meta::FromGodot for #enum_name {
            fn try_from_godot(via: Self::Via) -> std::result::Result<Self, crate::builtin::meta::ConvertError> {
                #self_as_trait::try_from_ord(via)
                    .ok_or_else(|| crate::builtin::meta::FromGodotError::InvalidEnum.into_error(via))
            }
        }
    }
}

pub fn make_constant_definition(constant: &ClassConstant) -> TokenStream {
    let name = ident(&constant.name);
    let vis = if constant.name.starts_with("NOTIFICATION_") {
        quote! { pub(crate) }
    } else {
        quote! { pub }
    };

    match constant.to_constant() {
        ConstValue::I32(value) => quote! { #vis const #name: i32 = #value; },
        ConstValue::I64(value) => quote! { #vis const #name: i64 = #value; },
    }
}

/// Tries to interpret the constant as a notification one, and transforms it to a Rust identifier on success.
pub fn try_to_notification(constant: &ClassConstant) -> Option<Ident> {
    constant
        .name
        .strip_prefix("NOTIFICATION_")
        .map(|s| ident(&conv::shout_to_pascal(s)))
}

/// If an enum qualifies as "indexable" (can be used as array index), returns the number of possible values.
///
/// See `godot::obj::IndexEnum` for what constitutes "indexable".
fn try_count_index_enum(enum_: &Enum) -> Option<usize> {
    if enum_.is_bitfield || enum_.values.is_empty() {
        return None;
    }

    // Sort by ordinal value. Allocates for every enum in the JSON, but should be OK (most enums are indexable).
    let enumerators = {
        let mut enumerators = enum_.values.clone();
        enumerators.sort_by_key(|v| v.value);
        enumerators
    };

    // Highest ordinal must be the "MAX" one.
    // The presence of "MAX" indicates that Godot devs intended the enum to be used as an index.
    // The condition is not strictly necessary and could theoretically be relaxed; there would need to be concrete use cases though.
    let last = enumerators.last().unwrap(); // safe because of is_empty check above.
    if !last.name.ends_with("_MAX") {
        return None;
    }

    // The rest of the enumerators must be contiguous and without gaps (duplicates are OK).
    let mut last_value = 0;
    for enumerator in enumerators.iter() {
        if last_value != enumerator.value && last_value + 1 != enumerator.value {
            return None;
        }

        last_value = enumerator.value;
    }

    Some(last.value as usize)
}

pub fn ident(s: &str) -> Ident {
    format_ident!("{}", s)
}

pub fn cstr_u8_slice(string: &str) -> Literal {
    Literal::byte_string(format!("{string}\0").as_bytes())
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

/// Converts a potential "meta" type (like u32) to its canonical type (like i64).
///
/// Avoids dragging along the meta type through [`RustTy::BuiltinIdent`].
pub(crate) fn unmap_meta(rust_ty: &RustTy) -> Option<Ident> {
    let RustTy::BuiltinIdent(rust_ty) = rust_ty else {
        return None;
    };

    // Don't use match because it needs allocation (unless == is repeated)
    // Even though i64 and f64 can have a meta of the same type, there's no need to return that here, as there won't be any conversion.

    for ty in ["u64", "u32", "u16", "u8", "i32", "i16", "i8"] {
        if rust_ty == ty {
            return Some(ident("i64"));
        }
    }

    if rust_ty == "f32" {
        return Some(ident("f64"));
    }

    None
}

/// Parse a string of semicolon-separated C-style type declarations. Fail with `None` if any errors occur.
pub fn parse_native_structures_format(input: &str) -> Option<Vec<NativeStructuresField>> {
    input
        .split(';')
        .filter(|var| !var.trim().is_empty())
        .map(|var| {
            let mut parts = var.trim().splitn(2, ' ');
            let mut field_type = parts.next()?.to_owned();
            let mut field_name = parts.next()?.to_owned();

            // If the field is a pointer, put the star on the type, not
            // the name.
            if field_name.starts_with('*') {
                field_name.remove(0);
                field_type.push('*');
            }

            // If Godot provided a default value, ignore it.
            // TODO We might use these if we synthetically generate constructors in the future
            if let Some(index) = field_name.find(" = ") {
                field_name.truncate(index);
            }

            Some(NativeStructuresField {
                field_type,
                field_name,
            })
        })
        .collect()
}

pub(crate) fn make_enumerator_ord(ord: i32) -> Literal {
    Literal::i32_suffixed(ord)
}

/// This method is needed for platform-dependent types like raw `VariantOperator`, which can be `i32` or `u32`.
/// Do not suffix them.
pub(crate) fn make_enumerator_ord_unsuffixed(ord: i32) -> Literal {
    Literal::i32_unsuffixed(ord)
}

pub(crate) fn make_bitfield_flag_ord(ord: u64) -> Literal {
    Literal::u64_suffixed(ord)
}
