/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Type and expression conversions (Godot -> Rust)

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{quote, ToTokens};
use std::fmt;

use crate::context::Context;
use crate::conv;
use crate::models::domain::{GodotTy, ModName, RustTy, TyName};
use crate::special_cases::is_builtin_type_scalar;
use crate::util::ident;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Godot -> Rust types

fn to_hardcoded_rust_ident(full_ty: &GodotTy) -> Option<&str> {
    let ty = full_ty.ty.as_str();
    let meta = full_ty.meta.as_deref();

    let result = match (ty, meta) {
        // Integers
        ("int", Some("int64") | None) => "i64",
        ("int", Some("int32")) => "i32",
        ("int", Some("int16")) => "i16",
        ("int", Some("int8")) => "i8",
        ("int", Some("uint64")) => "u64",
        ("int", Some("uint32")) => "u32",
        ("int", Some("uint16")) => "u16",
        ("int", Some("uint8")) => "u8",
        ("int", Some(meta)) => panic!("unhandled type int with meta {meta:?}"),

        // Floats (with single precision builds)
        ("float", Some("double") | None) => "f64",
        ("float", Some("float")) => "f32",
        ("float", Some(meta)) => panic!("unhandled type float with meta {meta:?}"),

        // Doubles (with double precision builds)
        ("double", None) => "f64",
        ("double", Some(meta)) => panic!("unhandled type double with meta {meta:?}"),

        // Others
        ("bool", None) => "bool",
        ("String", None) => "GString",
        ("Array", None) => "VariantArray",

        // Types needed for native structures mapping
        ("uint8_t", None) => "u8",
        ("uint16_t", None) => "u16",
        ("uint32_t", None) => "u32",
        ("uint64_t", None) => "u64",
        ("int8_t", None) => "i8",
        ("int16_t", None) => "i16",
        ("int32_t", None) => "i32",
        ("int64_t", None) => "i64",
        ("real_t", None) => "real",
        ("void", None) => "c_void",

        (ty, Some(meta)) => panic!("unhandled type {ty:?} with meta {meta:?}"),

        _ => return None,
    };

    Some(result)
}

fn to_hardcoded_rust_enum(ty: &str) -> Option<&str> {
    let result = match ty {
        //"enum::Error" => "GodotError",
        "enum::Variant.Type" => "VariantType",
        "enum::Variant.Operator" => "VariantOperator",
        "enum::Vector3.Axis" => "Vector3Axis",
        "enum::Vector3i.Axis" => "Vector3Axis",
        _ => return None,
    };
    Some(result)
}

/// Maps an input type to a Godot type with the same C representation. This is subtly different from [`to_rust_type`],
/// which maps to an appropriate corresponding Rust type. This function should be used in situations where the C ABI for
/// a type must match the Godot equivalent exactly, such as when dealing with pointers.
pub(crate) fn to_rust_type_abi(ty: &str, ctx: &mut Context) -> (RustTy, bool) {
    let mut is_obj = false;
    let ty = match ty {
        // In native structures, object pointers are mapped to opaque entities. Instead, an accessor function is provided.
        "Object*" => {
            is_obj = true;
            RustTy::RawPointer {
                inner: Box::new(RustTy::BuiltinIdent(ident("c_void"))),
                is_const: false,
            }
        }
        "int" => RustTy::BuiltinIdent(ident("i32")),
        "float" => RustTy::BuiltinIdent(ident("f32")),
        "double" => RustTy::BuiltinIdent(ident("f64")),
        _ => to_rust_type(ty, None, ctx),
    };

    (ty, is_obj)
}

/// Maps an _input_ type from the Godot JSON to the corresponding Rust type (wrapping some sort of token stream).
///
/// Uses an internal cache (via `ctx`), as several types are ubiquitous.
// TODO take TyName as input
pub(crate) fn to_rust_type<'a>(ty: &'a str, meta: Option<&'a String>, ctx: &mut Context) -> RustTy {
    let full_ty = GodotTy {
        ty: ty.to_string(),
        meta: meta.cloned(),
    };

    // Separate find + insert slightly slower, but much easier with lifetimes
    // The insert path will be hit less often and thus doesn't matter
    if let Some(rust_ty) = ctx.find_rust_type(&full_ty) {
        rust_ty.clone()
    } else {
        let rust_ty = to_rust_type_uncached(&full_ty, ctx);
        ctx.insert_rust_type(full_ty, rust_ty.clone());
        rust_ty
    }
}

fn to_rust_type_uncached(full_ty: &GodotTy, ctx: &mut Context) -> RustTy {
    let ty = full_ty.ty.as_str();

    /// Transforms a Godot class/builtin/enum IDENT (without `::` or other syntax) to a Rust one
    fn rustify_ty(ty: &str) -> Ident {
        if is_builtin_type_scalar(ty) {
            ident(ty)
        } else {
            TyName::from_godot(ty).rust_ty
        }
    }

    if ty.ends_with('*') {
        // Pointer type; strip '*', see if const, and then resolve the inner type.
        let mut ty = ty[0..ty.len() - 1].to_string();

        // 'const' should apply to the innermost pointer, if present.
        let is_const = ty.starts_with("const ") && !ty.ends_with('*');
        if is_const {
            ty = ty.replace("const ", "");
        }

        // .trim() is necessary here, as Godot places a space between a type and the stars when representing a double pointer.
        // Example: "int*" but "int **".
        let inner_type = to_rust_type(ty.trim(), None, ctx);
        return RustTy::RawPointer {
            inner: Box::new(inner_type),
            is_const,
        };
    }

    // Only place where meta is relevant is here.
    if let Some(hardcoded) = to_hardcoded_rust_ident(full_ty) {
        return RustTy::BuiltinIdent(ident(hardcoded));
    }

    if let Some(hardcoded) = to_hardcoded_rust_enum(ty) {
        return RustTy::EngineEnum {
            tokens: ident(hardcoded).to_token_stream(),
            surrounding_class: None, // would need class passed in
        };
    }

    if let Some(bitfield) = ty.strip_prefix("bitfield::") {
        return if let Some((class, enum_)) = bitfield.split_once('.') {
            // Class-local bitfield.
            let module = ModName::from_godot(class);
            let bitfield_ty = conv::make_enum_name(enum_);

            RustTy::EngineBitfield {
                tokens: quote! { crate::classes::#module::#bitfield_ty},
                surrounding_class: Some(class.to_string()),
            }
        } else {
            // Global bitfield.
            let bitfield_ty = conv::make_enum_name(bitfield);

            RustTy::EngineBitfield {
                tokens: quote! { crate::global::#bitfield_ty },
                surrounding_class: None,
            }
        };
    }

    if let Some(qualified_enum) = ty.strip_prefix("enum::") {
        return if let Some((class, enum_)) = qualified_enum.split_once('.') {
            // Class-local enum
            let module = ModName::from_godot(class);
            let enum_ty = conv::make_enum_name(enum_);

            RustTy::EngineEnum {
                tokens: quote! { crate::classes::#module::#enum_ty },
                surrounding_class: Some(class.to_string()),
            }
        } else {
            // Global enum
            let enum_ty = conv::make_enum_name(qualified_enum);

            RustTy::EngineEnum {
                tokens: quote! { crate::global::#enum_ty },
                surrounding_class: None,
            }
        };
    } else if let Some(packed_arr_ty) = ty.strip_prefix("Packed") {
        // Don't trigger on PackedScene ;P
        if packed_arr_ty.ends_with("Array") {
            return RustTy::BuiltinIdent(rustify_ty(ty));
        }
    } else if let Some(elem_ty) = ty.strip_prefix("typedarray::") {
        let rust_elem_ty = to_rust_type(elem_ty, None, ctx);
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
    if ctx.is_builtin(ty) || ctx.is_native_structure(ty) {
        // Unchanged
        RustTy::BuiltinIdent(rustify_ty(ty))
    } else {
        let ty = rustify_ty(ty);
        let qualified_class = quote! { crate::classes::#ty };

        RustTy::EngineClass {
            tokens: quote! { Gd<#qualified_class> },
            object_arg: quote! { ObjectArg<#qualified_class> },
            impl_as_object_arg: quote! { impl AsObjectArg<#qualified_class> },
            inner_class: ty,
        }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Godot -> Rust expressions

pub(crate) fn to_rust_expr(expr: &str, ty: &RustTy) -> TokenStream {
    // println!("\n> to_rust_expr({expr}, {ty:?})");

    to_rust_expr_inner(expr, ty, false)
}

fn to_rust_expr_inner(expr: &str, ty: &RustTy, is_inner: bool) -> TokenStream {
    // println!("> to_rust_expr_inner({expr}, {is_inner})");

    // Simple literals
    match expr {
        "true" => return quote! { true },
        "false" => return quote! { false },
        "[]" | "{}" if is_inner => return quote! {},
        "[]" => return quote! { Array::new() }, // VariantArray or Array<T>
        "{}" => return quote! { Dictionary::new() },
        "null" => {
            return match ty {
                RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { Variant::nil() },
                RustTy::EngineClass { .. } => {
                    quote! { Gd::null_arg() }
                }
                _ => panic!("null not representable in target type {ty:?}"),
            }
        }
        // empty string appears only for Callable/Rid in 4.0; default ctor syntax in 4.1+
        "" | "RID()" | "Callable()" if !is_inner => {
            return match ty {
                RustTy::BuiltinIdent(ident) if ident == "Rid" => quote! { Rid::Invalid },
                RustTy::BuiltinIdent(ident) if ident == "Callable" => {
                    quote! { Callable::invalid() }
                }
                _ => panic!("empty string not representable in target type {ty:?}"),
            }
        }
        _ => {}
    }

    // Integer literals
    if let Ok(num) = expr.parse::<i64>() {
        let lit = Literal::i64_unsuffixed(num);
        return match ty {
            RustTy::EngineBitfield { .. } => quote! { crate::obj::EngineBitfield::from_ord(#lit) },
            RustTy::EngineEnum { .. } => quote! { crate::obj::EngineEnum::from_ord(#lit) },
            RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { Variant::from(#lit) },
            RustTy::BuiltinIdent(ident)
                if ident == "i64" || ident == "f64" || unmap_meta(ty).is_some() =>
            {
                suffixed_lit(num, ident)
            }
            _ if is_inner => quote! { #lit as _ },
            // _ => quote! { #lit as #ty },
            _ => panic!("cannot map integer literal {expr} to type {ty:?}"),
        };
    }

    // Float literals (some floats already handled by integer literals)
    if let Ok(num) = expr.parse::<f64>() {
        return match ty {
            RustTy::BuiltinIdent(ident) if ident == "f64" || unmap_meta(ty).is_some() => {
                suffixed_lit(num, ident)
            }
            _ if is_inner => {
                let lit = Literal::f64_unsuffixed(num);
                quote! { #lit as _ }
            }
            _ => panic!("cannot map float literal {expr} to type {ty:?}"),
        };
    }

    // "..." -> String|StringName|NodePath
    if let Some(expr) = expr.strip_prefix('"') {
        let expr = expr.strip_suffix('"').expect("unmatched opening '\"'");
        return if is_inner {
            quote! { #expr }
        } else {
            match ty {
                RustTy::BuiltinIdent(ident)
                    if ident == "GString" || ident == "StringName" || ident == "NodePath" =>
                {
                    quote! { #ident::from(#expr) }
                }
                _ => quote! { GString::from(#expr) },
                //_ => panic!("cannot map string literal \"{expr}\" to type {ty:?}"),
            }
        };
    }

    // "&..." -> StringName
    if let Some(expr) = expr.strip_prefix("&\"") {
        let expr = expr.strip_suffix('"').expect("unmatched opening '&\"'");
        return quote! { StringName::from(#expr) };
    }

    // "^..." -> NodePath
    if let Some(expr) = expr.strip_prefix("^\"") {
        let expr = expr.strip_suffix('"').expect("unmatched opening '^\"'");
        return quote! { NodePath::from(#expr) };
    }

    // Constructor calls
    if let Some(pos) = expr.find('(') {
        let godot_ty = &expr[..pos];
        let wrapped = expr[pos + 1..].strip_suffix(')').expect("unmatched '('");

        let (rust_ty, ctor) = match godot_ty {
            "NodePath" => ("NodePath", "from"),
            "String" => ("GString", "from"),
            "StringName" => ("StringName", "from"),
            "RID" => ("Rid", "default"),
            "Rect2" => ("Rect2", "from_components"),
            "Rect2i" => ("Rect2i", "from_components"),
            "Vector2" | "Vector2i" | "Vector3" | "Vector3i" => (godot_ty, "new"),
            "Transform2D" => ("Transform2D", "__internal_codegen"),
            "Transform3D" => ("Transform3D", "__internal_codegen"),
            "Color" => {
                if wrapped.chars().filter(|&c| c == ',').count() == 2 {
                    ("Color", "from_rgb")
                } else {
                    ("Color", "from_rgba")
                }
            }
            array if array.starts_with("Packed") && array.ends_with("Array") => {
                assert_eq!(wrapped, "", "only empty packed arrays supported for now");
                (array, "new")
            }
            array if array.starts_with("Array[") => {
                assert_eq!(wrapped, "[]", "only empty typed arrays supported for now");
                ("Array", "new")
            }
            _ => panic!("unsupported type: {godot_ty}"),
        };

        // Split wrapped parts by comma
        let subtokens = wrapped.split(',').map(|part| {
            let part = part.trim(); // ignore whitespace around commas

            // If there is no comma, there will still be one part (the empty string) -- do not substitute
            if part.is_empty() {
                quote! {}
            } else {
                to_rust_expr_inner(part, ty, true)
            }
        });

        let rust_ty = ident(rust_ty);
        let ctor = ident(ctor);
        return quote! {
            #rust_ty::#ctor(#(#subtokens),*)
        };
    }

    panic!(
        "Not yet supported GDScript expression: '{expr}'\n\
        Please report this at https://github.com/godot-rust/gdext/issues/new."
    );
}

fn suffixed_lit(num: impl fmt::Display, suffix: &Ident) -> TokenStream {
    // i32, u16 etc. happen to be also the literal suffixes
    let combined = format!("{num}{suffix}");
    combined
        .parse::<Literal>()
        .unwrap_or_else(|_| panic!("invalid literal {combined}"))
        .to_token_stream()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[test]
fn gdscript_to_rust_expr() {
    // The 'None' type is used to simulate absence of type information. Some tests are commented out, because this functionality is not
    // yet needed. If we ever want to reuse to_rust_expr() in other contexts, we could re-enable them.

    let ty_int = RustTy::BuiltinIdent(ident("i64"));
    let ty_int = Some(&ty_int);

    let ty_int_u16 = RustTy::BuiltinIdent(ident("u16"));
    let ty_int_u16 = Some(&ty_int_u16);

    let ty_float = RustTy::BuiltinIdent(ident("f64"));
    let ty_float = Some(&ty_float);

    let ty_float_f32 = RustTy::BuiltinIdent(ident("f32"));
    let ty_float_f32 = Some(&ty_float_f32);

    let ty_enum = RustTy::EngineEnum {
        tokens: quote! { SomeEnum },
        surrounding_class: None,
    };
    let ty_enum = Some(&ty_enum);

    let ty_bitfield = RustTy::EngineBitfield {
        tokens: quote! { SomeEnum },
        surrounding_class: None,
    };
    let ty_bitfield = Some(&ty_bitfield);

    let ty_variant = RustTy::BuiltinIdent(ident("Variant"));
    let ty_variant = Some(&ty_variant);

    // let ty_object = RustTy::EngineClass {
    //     tokens: quote! { Gd<MyClass> },
    //     class: "MyClass".to_string(),
    // };
    // let ty_object = Some(&ty_object);

    let ty_string = RustTy::BuiltinIdent(ident("GString"));
    let ty_string = Some(&ty_string);

    let ty_stringname = RustTy::BuiltinIdent(ident("StringName"));
    let ty_stringname = Some(&ty_stringname);

    let ty_nodepath = RustTy::BuiltinIdent(ident("NodePath"));
    let ty_nodepath = Some(&ty_nodepath);

    #[rustfmt::skip]
    let table = [
        // int
        ("0",                                              ty_int,             quote! { 0i64 }),
        ("-1",                                             ty_int,             quote! { -1i64 }),
        ("2147483647",                                     ty_int,             quote! { 2147483647i64 }),
        ("-2147483648",                                    ty_int,             quote! { -2147483648i64 }),
        // ("2147483647",                                     None,               quote! { 2147483647 }),
        // ("-2147483648",                                    None,               quote! { -2147483648 }),

        // int, meta=uint16
        ("0",                                              ty_int_u16,         quote! { 0u16 }),
        ("65535",                                          ty_int_u16,         quote! { 65535u16 }),

        // float (from int/float)
        ("0",                                              ty_float,           quote! { 0f64 }),
        ("2147483647",                                     ty_float,           quote! { 2147483647f64 }),
        ("-1.5",                                           ty_float,           quote! { -1.5f64 }),
        ("2e3",                                            ty_float,           quote! { 2000f64 }),
        // ("1.0",                                            None,               quote! { 1.0 }),
        // ("1e-05",                                          None,               quote! { 0.00001 }),

        // float, meta=f32 (from int/float)
        ("0",                                              ty_float_f32,       quote! { 0f32 }),
        ("-2147483648",                                    ty_float_f32,       quote! { -2147483648f32 }),
        ("-2.5",                                           ty_float_f32,       quote! { -2.5f32 }),
        ("3e3",                                            ty_float,           quote! { 3000f64 }),

        // enum (from int)
        ("7",                                              ty_enum,            quote! { crate::obj::EngineEnum::from_ord(7) }),

        // bitfield (from int)
        ("7",                                              ty_bitfield,        quote! { crate::obj::EngineBitfield::from_ord(7) }),

        // Variant (from int)
        ("8",                                              ty_variant,         quote! { Variant::from(8) }),

        // Special literals
        ("true",                                           None,               quote! { true }),
        ("false",                                          None,               quote! { false }),
        ("{}",                                             None,               quote! { Dictionary::new() }),
        ("[]",                                             None,               quote! { Array::new() }),

        ("null",                                           ty_variant,         quote! { Variant::nil() }),
        // TODO implement #156:
        //("null",                                           ty_object,          quote! { None }),

        // String-likes
        ("\" \"",                                          None,               quote! { GString::from(" ") }),
        ("\"{_}\"",                                        None,               quote! { GString::from("{_}") }),
        ("&\"text\"",                                      None,               quote! { StringName::from("text") }),
        ("^\"text\"",                                      None,               quote! { NodePath::from("text") }),

        ("\"text\"",                                       ty_string,          quote! { GString::from("text") }),
        ("\"text\"",                                       ty_stringname,      quote! { StringName::from("text") }),
        ("\"text\"",                                       ty_nodepath,        quote! { NodePath::from("text") }),

        // Composites
        ("NodePath(\"\")",                                 None,               quote! { NodePath::from("") }),
        ("Color(1, 0, 0.5, 1)",                            None,               quote! { Color::from_rgba(1 as _, 0 as _, 0.5 as _, 1 as _) }),
        ("Vector3(0, 1, 2.5)",                             None,               quote! { Vector3::new(0 as _, 1 as _, 2.5 as _) }),
        ("Rect2(1, 2.2, -3.3, 0)",                         None,               quote! { Rect2::from_components(1 as _, 2.2 as _, -3.3 as _, 0 as _) }),
        ("Rect2i(1, 2.2, -3.3, 0)",                        None,               quote! { Rect2i::from_components(1 as _, 2.2 as _, -3.3 as _, 0 as _) }),
        ("PackedFloat32Array()",                           None,               quote! { PackedFloat32Array::new() }),
        // Due to type inference, it should be enough to just write `Array::new()`
        ("Array[Plane]([])",                               None,               quote! { Array::new() }),
        ("Array[RDPipelineSpecializationConstant]([])",    None,               quote! { Array::new() }),
        ("Array[RID]([])",                                 None,               quote! { Array::new() }),

        // Composites with destructuring
        ("Transform3D(1, 2, 3, 4, -1.1, -1.2, -1.3, -1.4, 0, 0, 0, 0)", None,  quote! {
            Transform3D::__internal_codegen(
                   1 as _,    2 as _,    3 as _,
                   4 as _, -1.1 as _, -1.2 as _,
                -1.3 as _, -1.4 as _,    0 as _,
                   0 as _,    0 as _,    0 as _
            )
        }),

        ("Transform2D(1, 2, -1.1,1.2, 0, 0)",              None,               quote! {
            Transform2D::__internal_codegen(
                   1 as _,   2 as _,
                -1.1 as _, 1.2 as _,
                   0 as _,   0 as _
            )
        }),
    ];

    for (gdscript, ty, rust) in table {
        // Use arbitrary type if not specified -> should not be read
        let ty_dontcare = RustTy::EngineArray {
            tokens: TokenStream::new(),
            elem_class: String::new(),
        };
        let ty = ty.unwrap_or(&ty_dontcare);

        let actual = to_rust_expr(gdscript, ty).to_string();
        let expected = rust.to_string();

        // println!("{actual} -> {expected}");
        assert_eq!(actual, expected);
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
