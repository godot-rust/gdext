/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::api_parser::{ClassConstant, Enum};
use crate::special_cases::is_builtin_scalar;
use crate::{Context, GodotTy, ModName, RustTy, TyName};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeStructuresField {
    pub field_type: String,
    pub field_name: String,
}

/// Small utility that turns an optional vector (often encountered as JSON deserialization type) into a slice.
pub fn option_as_slice<T>(option: &Option<Vec<T>>) -> &[T] {
    option.as_ref().map_or(&[], Vec::as_slice)
}

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

    let mut derives = vec!["Copy", "Clone", "Eq", "PartialEq", "Debug", "Hash"];

    if enum_.is_bitfield {
        derives.push("Default");
    }

    let derives = derives.into_iter().map(ident);

    // Enumerator ordinal stored as i32, since that's enough to hold all current values and the default repr in C++.
    // Public interface is i64 though, for consistency (and possibly forward compatibility?).
    // TODO maybe generalize GodotFfi over EngineEnum trait
    quote! {
        #[repr(transparent)]
        #[derive(#( #derives ),*)]
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
        // SAFETY:
        // The enums are transparently represented as an `i32`, so `*mut Self` is sound.
        unsafe impl sys::GodotFfi for #enum_name {
            sys::ffi_methods! { type sys::GDExtensionTypePtr = *mut Self; .. }
        }
        #bitfield_ops
    }
}

pub fn make_constant_definition(constant: &ClassConstant) -> TokenStream {
    let ClassConstant { name, value } = constant;
    let name = ident(name);

    if constant.name.starts_with("NOTIFICATION_") {
        // Already exposed through enums
        quote! {
            pub(crate) const #name: i32 = #value;
        }
    } else {
        quote! {
            pub const #name: i32 = #value;
        }
    }
}

/// Tries to interpret the constant as a notification one, and transforms it to a Rust identifier on success.
pub fn try_to_notification(constant: &ClassConstant) -> Option<Ident> {
    constant
        .name
        .strip_prefix("NOTIFICATION_")
        .map(|s| ident(&shout_to_pascal(s)))
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

pub fn shout_to_pascal(shout_case: &str) -> String {
    // TODO use heck?

    let mut result = String::with_capacity(shout_case.len());
    let mut next_upper = true;

    for ch in shout_case.chars() {
        if next_upper {
            assert_ne!(ch, '_'); // no double underscore
            result.push(ch); // unchanged
            next_upper = false;
        } else if ch == '_' {
            next_upper = true;
        } else {
            result.push(ch.to_ascii_lowercase());
        }
    }

    result
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

fn to_hardcoded_rust_ident(full_ty: &GodotTy) -> Option<&str> {
    let ty = full_ty.ty.as_str();
    let meta = full_ty.meta.as_ref().map(String::as_str);

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

        // Floats
        ("float", Some("double") | None) => "f64",
        ("float", Some("float")) => "f32",

        // Others
        ("bool", None) => "bool",
        ("String", None) => "GodotString",
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
        _ => return None,
    };
    Some(result)
}

/// Maps an input type to a Godot type with the same C representation. This is subtly different than [`to_rust_type`],
/// which maps to an appropriate corresponding Rust type. This function should be used in situations where the C ABI for
/// a type must match the Godot equivalent exactly, such as when dealing with pointers.
pub(crate) fn to_rust_type_abi(ty: &str, ctx: &mut Context) -> RustTy {
    match ty {
        "int" => RustTy::BuiltinIdent(ident("i32")),
        "float" => RustTy::BuiltinIdent(ident("f32")),
        "double" => RustTy::BuiltinIdent(ident("f64")),
        _ => to_rust_type(ty, None, ctx),
    }
}

/// Maps an _input_ type from the Godot JSON to the corresponding Rust type (wrapping some sort of a token stream).
///
/// Uses an internal cache (via `ctx`), as several types are ubiquitous.
// TODO take TyName as input
pub(crate) fn to_rust_type<'a>(ty: &'a str, meta: Option<&String>, ctx: &mut Context) -> RustTy {
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
        if is_builtin_scalar(ty) {
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

    let qualified_enum = ty
        .strip_prefix("enum::")
        .or_else(|| ty.strip_prefix("bitfield::"));

    if let Some(qualified_enum) = qualified_enum {
        return if let Some((class, enum_)) = qualified_enum.split_once('.') {
            // Class-local enum
            let module = ModName::from_godot(class);
            let enum_ty = make_enum_name(enum_);

            RustTy::EngineEnum {
                tokens: quote! { crate::engine::#module::#enum_ty },
                surrounding_class: Some(class.to_string()),
            }
        } else {
            // Global enum
            let enum_ty = make_enum_name(qualified_enum);

            RustTy::EngineEnum {
                tokens: quote! { crate::engine::global::#enum_ty },
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
        RustTy::EngineClass {
            tokens: quote! { Gd<crate::engine::#ty> },
            class: ty.to_string(),
        }
    }
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

            // If Godot provided a default value, ignore it. (TODO We
            // might use these if we synthetically generate constructors
            // in the future)
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

pub(crate) fn to_rust_expr(expr: &str, ty: &RustTy) -> TokenStream {
    to_rust_expr_inner(expr, ty, false)
}

fn to_rust_expr_inner(expr: &str, ty: &RustTy, is_inner: bool) -> TokenStream {
    println!("\n> to_rust_expr_inner({expr}, {is_inner})");

    // Simple literals
    match expr {
        "true" => return quote! { true },
        "false" => return quote! { false },
        "[]" | "{}" if is_inner => return quote! {},
        "[]" => return quote! { Array::new() }, // VariantArray or Array<T>
        "{}" => return quote! { Dictionary::new() },
        "null" => match ty {
            RustTy::BuiltinIdent(ident) if ident == "Variant" => return quote! { Variant::nil() },
            RustTy::EngineClass { .. } => return quote! { unimplemented!("see #156") },
            _ => panic!("null not representable in target type {ty:?}"),
        },
        // empty string appears only for Callable/Rid
        "" if !is_inner => match ty {
            RustTy::BuiltinIdent(ident) if ident == "Rid" => return quote! { Rid::Invalid },
            RustTy::BuiltinIdent(ident) if ident == "Callable" => {
                return quote! { Callable::invalid() }
            }
            _ => panic!("empty string not representable in target type {ty:?}"),
        },
        _ => {}
    }

    // Integers
    if let Ok(num) = expr.parse::<i64>() {
        let lit = Literal::i64_unsuffixed(num);
        return match ty {
            RustTy::BuiltinIdent(ident) if ident == "Variant" => quote! { Variant::from(#lit) },
            RustTy::EngineEnum { .. } => quote! { crate::obj::EngineEnum::from_ord(#lit) },
            // RustTy::BuiltinIdent(ident) if ident == "f64" || ident == "i64" => quote! { #lit },
            // _ => panic!("cannot map int literal {expr} to type {ty:?}")
            // _ => quote! { #lit.try_into().expect("default arg not representable in target type") },
            _ => quote! { #lit as _ },
        };
    }

    // Floats
    if let Ok(num) = expr.parse::<f64>() {
        let lit = Literal::f64_unsuffixed(num);
        // Assume that literals of type `2.4` are never used for integers, so we don't need conversion
        return quote! { #lit };
    }

    // "..." -> String|StringName|NodePath
    if let Some(expr) = expr.strip_prefix('"') {
        let expr = expr.strip_suffix('"').expect("unmatched opening '\"'");
        return if is_inner {
            quote! { #expr }
        } else {
            let RustTy::BuiltinIdent(string_ty) = ty else {
                panic!("cannot map string literal {expr} to type {ty:?}")
            };
            quote! { #string_ty::from(#expr) }
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
        dbg!(godot_ty);
        dbg!(expr);
        let wrapped = expr[pos + 1..].strip_suffix(')').expect("unmatched '('");

        let (rust_ty, ctor) = match godot_ty {
            "NodePath" => ("NodePath", "from"),
            "String" => ("GodotString", "from"),
            "StringName" => ("StringName", "from"),
            "RID" => ("Rid", "new"),
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

// ----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn gdscript_to_rust_expr() {
    // Do not provide types unless absolutely needed (write None) -- even though at the moment, codegen always has them.
    // This potentially allows us to reuse to_rust_expr() in the future in other contexts.

    let ty_int = RustTy::BuiltinIdent(ident("i64"));
    let ty_int = Some(&ty_int);

    let ty_float = RustTy::BuiltinIdent(ident("f64"));
    let ty_float = Some(&ty_float);

    let ty_enum = RustTy::EngineEnum {
        tokens: quote! { SomeEnum },
        surrounding_class: None,
    };
    let ty_enum = Some(&ty_enum);

    let ty_variant = RustTy::BuiltinIdent(ident("Variant"));
    let ty_variant = Some(&ty_variant);

    let ty_object = RustTy::EngineClass {
        tokens: quote! { Gd<MyClass> },
        class: "MyClass".to_string(),
    };
    let ty_object = Some(&ty_object);

    #[rustfmt::skip]
    let table = [
        // int
        ("0",                                              ty_int,             quote! { 0 }),
        ("-1",                                             ty_int,             quote! { -1 }),
        ("2147483647",                                     ty_int,             quote! { 2147483647 }),

        // float (from int/float)
        ("0",                                              ty_float,           quote! { 0 }),
        ("-1",                                             ty_float,           quote! { -1 }),
        ("2147483647",                                     ty_float,           quote! { 2147483647 }),
        ("1.0",                                            None,               quote! { 1.0 }),
        ("1e-05",                                          None,               quote! { 0.00001 }),

        // enum (from int)
        ("7",                                              ty_enum,            quote! { godot::obj::EngineEnum::from_ord(7) }),

        // Variant (from int)
        ("7",                                              ty_variant,         quote! { Variant::from(7) }),

        // Special literals
        ("true",                                           None,               quote! { true }),
        ("false",                                          None,               quote! { false }),
        ("{}",                                             None,               quote! { Dictionary::new() }),
        ("[]",                                             None,               quote! { VariantArray::new() }),

        ("null",                                           ty_variant,         quote! { Variant::nil() }),
        ("null",                                           ty_object,          quote! { None }), // TODO implement #156

        // String-likes
        ("\" \"",                                          None,               quote! { GodotString::from(" ") }),
        ("\"{_}\"",                                        None,               quote! { GodotString::from("{_}") }),
        ("&\"text\"",                                      None,               quote! { StringName::from("text") }),
        ("^\"text\"",                                      None,               quote! { NodePath::from("text") }),

        // Composites
        ("NodePath(\"\")",                                 None,               quote! { NodePath::from("") }),
        ("Color(1, 0, 0.5, 1)",                            None,               quote! { Color::from_rgba(1.into(), 0.into(), 0.5.into(), 1.into()) }),
        ("Vector3(0, 1, 2.5)",                             None,               quote! { Vector3::new(0.into(), 1.into(), 2.5.into()) }),
        ("Rect2(1, 2.2, -3.3, 0)",                         None,               quote! { Rect2::from_components(1.into(), 2.2.into(), -3.3.into(), 0.into()) }),
        ("Rect2i(1, 2.2, -3.3, 0)",                        None,               quote! { Rect2i::from_components(1.into(), 2.2.into(), -3.3.into(), 0.into()) }),
        ("PackedFloat32Array()",                           None,               quote! { PackedFloat32Array::new() }),
        // Due to type inference, it should be enough to just write `Array::new()`
        ("Array[Plane]([])",                               None,               quote! { Array::new() }),
        ("Array[RDPipelineSpecializationConstant]([])",    None,               quote! { Array::new() }),
        ("Array[RID]([])",                                 None,               quote! { Array::new() }),

        // Composites with destructuring
        ("Transform3D(1, 2, 3, 4, -1.1, -1.2, -1.3, -1.4, 0, 0, 0, 0)", None,  quote! {
            Transform3D::__internal_codegen(
                   1.into(),    2.into(),    3.into(),
                   4.into(), -1.1.into(), -1.2.into(),
                -1.3.into(), -1.4.into(),    0.into(),
                   0.into(),    0.into(),    0.into()
            )
        }),

        ("Transform2D(1, 2, -1.1,1.2, 0, 0)",              None,               quote! {
            Transform2D::__internal_codegen(
                   1.into(),   2.into(),
                -1.1.into(), 1.2.into(),
                   0.into(),   0.into()
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

        let actual = to_rust_expr(gdscript, &ty).to_string();
        let expected = rust.to_string();

        println!("{actual} -> {expected}");
        assert_eq!(actual, expected);
    }
}
