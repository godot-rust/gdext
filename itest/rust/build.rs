/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::io::Write;
use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

type IoResult = std::io::Result<()>;

struct Input {
    ident: String,
    gdscript_ty: &'static str,
    gdscript_val: &'static str,
    rust_ty: TokenStream,
    rust_val: TokenStream,
    is_property: bool,
    is_exportable: bool,
    initializer: Option<TokenStream>,
    extra: TokenStream,
}

/// Push with GDScript expr in string
macro_rules! pushs {
    (
        $inputs:ident;
        $GDScriptTy:expr,
        $RustTy:ty,
        $gdscript_val:expr,
        $rust_val:expr,
        $property:expr,
        $export:expr,
        $initializer:expr
        $(; $($extra:tt)* )?
    ) => {
        $inputs.push(Input {
            ident: stringify!($RustTy)
                .to_ascii_lowercase()
                .replace("<", "_")
                .replace(">", ""),
            gdscript_ty: stringify!($GDScriptTy),
            gdscript_val: $gdscript_val,
            rust_ty: quote! { $RustTy },
            rust_val: quote! { $rust_val },
            is_property: $property,
            is_exportable: $export,
            initializer: $initializer,
            extra: quote! { $($($extra)*)? },
        });
    };
}

/// Push simple GDScript expression, outside string
macro_rules! push {
    ($inputs:ident; $GDScriptTy:expr, $RustTy:ty, $val:expr) => {
        push!($inputs; $GDScriptTy, $RustTy, $val, $val);
    };

    ($inputs:ident; $GDScriptTy:expr, $RustTy:ty, $gdscript_val:expr, $rust_val:expr) => {
        pushs!($inputs; $GDScriptTy, $RustTy, stringify!($gdscript_val), $rust_val, true, true, None);
    };
}

macro_rules! push_newtype {
    ($inputs:ident; $GDScriptTy:expr, $name:ident($T:ty), $val:expr) => {
        push_newtype!($inputs; $GDScriptTy, $name($T), $val, $name($val));
    };

    ($inputs:ident; $GDScriptTy:expr, $name:ident($T:ty), $gdscript_val:expr, $rust_val:expr) => {
        push_newtype!(@s $inputs; $GDScriptTy, $name($T), stringify!($gdscript_val), $rust_val);
    };

    (@s $inputs:ident; $GDScriptTy:expr, $name:ident($T:ty), $gdscript_val:expr, $rust_val:expr) => {
        pushs!(
            $inputs; $GDScriptTy, $name, $gdscript_val, $rust_val, false, false, None;

            #[derive(Clone, PartialEq, Debug)]
            pub struct $name($T);

            impl godot::meta::GodotConvert for $name {
                type Via = $T;
            }

            impl godot::meta::ToGodot for $name {
                type Pass = godot::meta::ByValue;

                #[allow(clippy::clone_on_copy)]
                fn to_godot(&self) -> Self::Via {
                    self.0.clone()
                }
            }

            impl godot::meta::FromGodot for $name {
                fn try_from_godot(via: Self::Via) -> Result<Self, godot::meta::error::ConvertError> {
                    Ok(Self(via))
                }
            }
        );
    }
}

// Edit this to change involved types
fn collect_inputs() -> Vec<Input> {
    let mut inputs = vec![];

    // Scalar
    push!(inputs; int, i64, -922337203685477580);
    push!(inputs; int, i32, -2147483648);
    push!(inputs; int, u32, 4294967295);
    push!(inputs; int, i16, -32767);
    push!(inputs; int, u16, 65535);
    push!(inputs; int, i8, -128);
    push!(inputs; int, u8, 255);
    push!(inputs; float, f32, 12.5);
    push!(inputs; float, f64, 127.83156478);
    push!(inputs; bool, bool, true);
    push!(inputs; Color, Color, Color(0.7, 0.5, 0.3, 0.2), Color::from_rgba(0.7, 0.5, 0.3, 0.2));
    push!(inputs; String, GString, "hello", GString::from("hello"));
    push!(inputs; StringName, StringName, &"hello", StringName::from("hello"));
    pushs!(inputs; NodePath, NodePath, r#"^"hello""#, NodePath::from("hello"), true, true, None);
    push!(inputs; Vector2, Vector2, Vector2(12.5, -3.5), Vector2::new(12.5, -3.5));
    push!(inputs; Vector3, Vector3, Vector3(117.5, 100.0, -323.25), Vector3::new(117.5, 100.0, -323.25));
    push!(inputs; Vector4, Vector4, Vector4(-18.5, 24.75, -1.25, 777.875), Vector4::new(-18.5, 24.75, -1.25, 777.875));
    push!(inputs; Vector2i, Vector2i, Vector2i(-2147483648, 2147483647), Vector2i::new(-2147483648, 2147483647));
    push!(inputs; Vector3i, Vector3i, Vector3i(-1, -2147483648, 2147483647), Vector3i::new(-1, -2147483648, 2147483647));
    push!(inputs; Vector4i, Vector4i, Vector4i(-1, -2147483648, 2147483647, 1000), Vector4i::new(-1, -2147483648, 2147483647, 100));
    pushs!(inputs; Callable, Callable, "Callable()", Callable::invalid(), true, false, Some(quote! { Callable::invalid() }));
    pushs!(inputs; Signal, Signal, "Signal()", Signal::invalid(), true, false, Some(quote! { Signal::invalid() }));
    push!(inputs; Rect2, Rect2, Rect2(), Rect2::default());
    push!(inputs; Rect2i, Rect2i, Rect2i(), Rect2i::default());
    push!(inputs; Transform2D, Transform2D, Transform2D(), Transform2D::default());
    pushs!(inputs; Plane, Plane, "Plane()", Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0), true, true, Some(quote! { Plane::new(Vector3::new(1.0, 0.0, 0.0), 0.0) }));
    push!(inputs; Quaternion, Quaternion, Quaternion(), Quaternion::default());
    push!(inputs; AABB, Aabb, AABB(), Aabb::default());
    push!(inputs; Basis, Basis, Basis(), Basis::default());
    push!(inputs; Transform3D, Transform3D, Transform3D(), Transform3D::default());
    push!(inputs; Projection, Projection, Projection(), Projection::default());
    pushs!(inputs; RID, Rid, "RID()", Rid::Invalid, true, false, Some(quote! { Rid::Invalid }));
    push!(inputs; Node, Option<Gd<Node>>, null, None);
    push!(inputs; Resource, Option<Gd<Resource>>, null, None);
    push!(inputs; PackedByteArray, PackedByteArray, PackedByteArray(), PackedByteArray::new());
    push!(inputs; PackedInt32Array, PackedInt32Array, PackedInt32Array(), PackedInt32Array::new());
    push!(inputs; PackedInt64Array, PackedInt64Array, PackedInt64Array(), PackedInt64Array::new());
    push!(inputs; PackedFloat32Array, PackedFloat32Array, PackedFloat32Array(), PackedFloat32Array::new());
    push!(inputs; PackedFloat64Array, PackedFloat64Array, PackedFloat64Array(), PackedFloat64Array::new());
    push!(inputs; PackedStringArray, PackedStringArray, PackedStringArray(), PackedStringArray::new());
    push!(inputs; PackedVector2Array, PackedVector2Array, PackedVector2Array(), PackedVector2Array::new());
    push!(inputs; PackedVector3Array, PackedVector3Array, PackedVector3Array(), PackedVector3Array::new());

    // This is being run in a build script at the same time as other build-scripts, so the `rustc-cfg` directives haven't been run for this
    // build-script. This means that `#[cfg(since_api = "4.3")]` wouldn't do anything.
    if godot_bindings::since_api("4.3") {
        push!(inputs; PackedVector4Array, PackedVector4Array, PackedVector4Array(), PackedVector4Array::new());
    }
    push!(inputs; PackedColorArray, PackedColorArray, PackedColorArray(), PackedColorArray::new());

    push_newtype!(inputs; int, NewI64(i64), -922337203685477580);
    push_newtype!(inputs; int, NewI32(i32), -2147483648);
    push_newtype!(inputs; int, NewU32(u32), 4294967295);
    push_newtype!(inputs; int, NewI16(i16), -32767);
    push_newtype!(inputs; int, NewU16(u16), 65535);
    push_newtype!(inputs; int, NewI8(i8), -128);
    push_newtype!(inputs; int, NewU8(u8), 255);
    push_newtype!(inputs; float, NewF32(f32), 12.5);
    push_newtype!(inputs; float, NewF64(f64), 127.83156478);
    push_newtype!(inputs; bool, NewBool(bool), true);
    push_newtype!(inputs; Color, NewColor(Color), Color(0.7, 0.5, 0.3, 0.2), NewColor(Color::from_rgba(0.7, 0.5, 0.3, 0.2)));
    push_newtype!(inputs; String, NewString(GString), "hello", NewString(GString::from("hello")));
    push_newtype!(inputs; StringName, NewStringName(StringName), &"hello", NewStringName(StringName::from("hello")));
    push_newtype!(@s inputs; NodePath, NewNodePath(NodePath), r#"^"hello""#, NewNodePath(NodePath::from("hello")));
    push_newtype!(inputs; Vector2, NewVector2(Vector2), Vector2(12.5, -3.5), NewVector2(Vector2::new(12.5, -3.5)));
    push_newtype!(inputs; Vector3, NewVector3(Vector3), Vector3(117.5, 100.0, -323.25), NewVector3(Vector3::new(117.5, 100.0, -323.25)));
    push_newtype!(inputs; Vector4, NewVector4(Vector4), Vector4(-18.5, 24.75, -1.25, 777.875), NewVector4(Vector4::new(-18.5, 24.75, -1.25, 777.875)));
    push_newtype!(inputs; Vector2i, NewVector2i(Vector2i), Vector2i(-2147483648, 2147483647), NewVector2i(Vector2i::new(-2147483648, 2147483647)));
    push_newtype!(inputs; Vector3i, NewVector3i(Vector3i), Vector3i(-1, -2147483648, 2147483647), NewVector3i(Vector3i::new(-1, -2147483648, 2147483647)));
    push_newtype!(inputs; Vector4i, NewVector4i(Vector4i), Vector4i(-1, -2147483648, 2147483647, 1), NewVector4i(Vector4i::new(-1, -2147483648, 2147483647, 1)));
    push_newtype!(inputs; Callable, NewCallable(Callable), Callable(), NewCallable(Callable::invalid()));

    // Data structures
    // TODO enable below, when GDScript has typed array literals, or find a hack with eval/lambdas
    /*pushs!(inputs; Array[int], Array<i32>,
        "(func() -> Array[int]: [-7, 12, 40])()",
        array![-7, 12, 40]
    );*/

    push!(inputs; Array, VarArray,
        [-7, "godot", false, Vector2i(-77, 88)],
        varray![-7, "godot", false, Vector2i::new(-77, 88)]);

    pushs!(inputs; Dictionary, VarDictionary,
        r#"{"key": 83, -3: Vector2(1, 2), 0.03: true}"#,
        vdict! { "key": 83, (-3): Vector2::new(1.0, 2.0), 0.03: true },
        true, true, None
    );

    // Composite
    pushs!(inputs; int, InstanceId, "-1", InstanceId::from_i64(0xFFFFFFFFFFFFFFF), false, false, None);
    // TODO: should `Variant` implement property?
    pushs!(inputs; Variant, Variant, "123", 123i64.to_variant(), false, false, None);

    // EngineEnum
    pushs!(inputs; int, Error, "0", Error::OK, false, false, None);

    inputs
}

fn main() {
    let inputs = collect_inputs();
    let methods = generate_rust_methods(&inputs);
    let PropertyTests {
        rust: rust_property_tests,
        gdscript: gdscript_property_tests,
    } = generate_property_template(&inputs);

    let extras = inputs.iter().map(|input| &input.extra);

    let rust_tokens = quote::quote! {
        use godot::builtin::*;
        use godot::meta::*;
        use godot::obj::{Gd, InstanceId};
        use godot::global::{Error, godot_error};
        use godot::classes::{Node, Resource};

        #[derive(godot::register::GodotClass)]
        #[class(init)]
        struct GenFfi {}

        #[allow(clippy::bool_comparison)] // i == true
        #[godot::register::godot_api]
        impl GenFfi {
            #(#methods)*
        }

        mod property_tests {
            use godot::prelude::*;

            #rust_property_tests
        }

        pub use property_tests::*;

        #(#extras)*
    };

    // Godot currently still uses local /gen folder. If this changes one day (no good reason right now),
    // IntegrationTest class could get a func get_out_dir() which returns env!("OUT_DIR") and is called from GDScript.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let rust_output_dir = Path::new(&out_dir);
    let godot_input_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/input"));
    let godot_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/gen"));

    let rust_file = rust_output_dir.join("gen_ffi.rs");
    let gdscript_template = godot_input_dir.join("GenFfiTests.template.gd");
    let gdscript_file = godot_output_dir.join("GenFfiTests.gd");
    let gdscript_property_tests_file = godot_output_dir.join("GenPropertyTests.gd");

    std::fs::create_dir_all(rust_output_dir).expect("create Rust parent dir");
    std::fs::create_dir_all(godot_output_dir).expect("create GDScript parent dir");
    std::fs::write(&rust_file, rust_tokens.to_string()).expect("write to Rust file");
    write_gdscript_code(&inputs, &gdscript_template, &gdscript_file)
        .expect("write to GDScript file");
    std::fs::write(gdscript_property_tests_file, gdscript_property_tests)
        .expect("write to GDScript Property Template file");

    println!("cargo:rerun-if-changed={}", gdscript_template.display());

    rustfmt_if_needed(vec![rust_file]);

    godot_bindings::emit_godot_version_cfg();
    godot_bindings::emit_safeguard_levels();

    // The godot crate has a __codegen-full default feature that enables the godot-codegen/codegen-full feature. When compiling the entire
    // workspace itest also gets compiled with full codegen due to feature unification. This causes compiler errors since the
    // itest/codegen-full feature does not automatically get enabled in such a situation.
    //
    // By conditionally emitting the feature config we can auto enable the feature for itest as well.
    if godot_codegen::IS_CODEGEN_FULL {
        println!("cargo::rustc-cfg=feature=\"codegen-full\"");
    }
}

// TODO remove, or remove code duplication with codegen
fn rustfmt_if_needed(out_files: Vec<std::path::PathBuf>) {
    //print!("Format {} generated files...", out_files.len());

    let mut process = std::process::Command::new("rustup");
    process
        .arg("run")
        .arg("stable")
        .arg("rustfmt")
        .arg("--edition=2021");

    for file in out_files {
        //println!("Format {file:?}");
        process.arg(file);
    }

    match process.output() {
        Ok(_) => println!("Done."),
        Err(err) => {
            println!("Failed.");
            println!("Error: {err}");
        }
    }
}

fn generate_rust_methods(inputs: &[Input]) -> Vec<TokenStream> {
    let mut generated_methods = inputs
        .iter()
        .map(|input| {
            let Input {
                ident,
                rust_ty,
                rust_val,
                ..
            } = input;

            let return_method = format_ident!("return_{ident}");
            let accept_method = format_ident!("accept_{ident}");
            let mirror_method = format_ident!("mirror_{ident}");
            let panic_method = format_ident!("panic_{ident}");

            let return_static_method = format_ident!("return_static_{ident}");
            let accept_static_method = format_ident!("accept_static_{ident}");
            let mirror_static_method = format_ident!("mirror_static_{ident}");

            quote! {
                #[func]
                fn #return_method(&self) -> #rust_ty {
                    #rust_val
                }

                #[func]
                fn #accept_method(&self, i: #rust_ty) -> bool {
                    i == #rust_val
                }

                #[func]
                fn #mirror_method(&self, i: #rust_ty) -> #rust_ty {
                    i
                }

                #[func]
                fn #panic_method(&self) -> #rust_ty {
                    panic!("intentional panic in `{}`", stringify!(#panic_method));
                }

                #[func]
                fn #return_static_method() -> #rust_ty {
                    #rust_val
                }

                #[func]
                fn #accept_static_method(i: #rust_ty) -> bool {
                    i == #rust_val
                }

                #[func]
                fn #mirror_static_method(i: #rust_ty) -> #rust_ty {
                    i
                }
            }
        })
        .collect::<Vec<_>>();

    let manual_methods = quote! {
        #[allow(clippy::suspicious_else_formatting)] // `quote!` might output whole file as one big line.
        #[func]
        fn check_last_notrace(last_method_name: String, expected_callconv: String) -> bool {
            let last = godot::private::trace::pop();
            let mut ok = true;

            if last.class != "GenFfi" {
                godot_error!("Expected class GenFfi, got {}", last.class);
                ok = false;
            }

            if last.method != last_method_name {
                godot_error!("Expected method {}, got {}", last_method_name, last.method);
                ok = false;
            }

            if !last.is_inbound {
                godot_error!("Expected inbound call, got outbound");
                ok = false;
            }

            let expect_ptrcall = expected_callconv == "ptrcall";
            if last.is_ptrcall != expect_ptrcall {
                let actual = Self::to_string(last.is_ptrcall);
                godot_error!("Expected {expected_callconv}, got {actual}");
                ok = false;
            }

            ok
        }

        fn to_string(is_ptrcall: bool) -> &'static str {
            if is_ptrcall {
                "ptrcall"
            } else {
                "varcall"
            }
        }
    };

    generated_methods.push(manual_methods);
    generated_methods
}

struct PropertyTests {
    rust: TokenStream,
    gdscript: String,
}

fn generate_property_template(inputs: &[Input]) -> PropertyTests {
    let mut rust = Vec::new();
    let mut gdscript = Vec::new();
    gdscript.push(String::from("extends Node\n"));
    for input in inputs.iter() {
        let Input {
            ident,
            gdscript_ty,
            rust_ty,
            is_property,
            is_exportable,
            initializer,
            ..
        } = input;

        if !is_property {
            continue;
        }

        let var = format_ident!("var_{ident}");
        let var_array = format_ident!("var_array_{ident}");
        let export = format_ident!("export_{ident}");
        let export_array = format_ident!("export_array_{ident}");

        let initializer = initializer
            .as_ref()
            .map(|init| quote! { #[init(val = #init)] });

        rust.extend([
            quote! {
                #[var]
                #initializer
                #var: #rust_ty
            },
            quote! { #[var] #var_array: Array<#rust_ty> },
        ]);

        gdscript.extend([
            format!("var {var}: {gdscript_ty}"),
            format!("var {var_array}: Array[{gdscript_ty}]"),
        ]);

        if *is_exportable {
            rust.extend([
                quote! {
                    #[export]
                    #initializer
                    #export: #rust_ty
                },
                quote! { #[export] #export_array: Array<#rust_ty> },
            ]);

            gdscript.extend([
                format!("@export var {export}: {gdscript_ty}"),
                format!("@export var {export_array}: Array[{gdscript_ty}]"),
            ]);
        }
    }

    // Only available in Godot 4.3+.
    let rust_exports_4_3 = if godot_bindings::before_api("4.3") {
        TokenStream::new()
    } else {
        quote! {
            #[export(storage)]
            export_storage: GString,

            #[export(file)]
            export_file_array: Array<GString>,
            #[export(file)]
            export_file_parray: PackedStringArray,

            #[export(file = "*.txt")]
            export_file_wildcard_array: Array<GString>,
            #[export(file = "*.txt")]
            export_file_wildcard_parray: PackedStringArray,

            #[export(global_file)]
            export_global_file_array: Array<GString>,
            #[export(global_file)]
            export_global_file_parray: PackedStringArray,

            #[export(global_file = "*.png")]
            export_global_file_wildcard_array: Array<GString>,
            #[export(global_file = "*.png")]
            export_global_file_wildcard_parray: PackedStringArray,

            #[export(dir)]
            export_dir_array: Array<GString>,
            #[export(dir)]
            export_dir_parray: PackedStringArray,

            #[export(global_dir)]
            export_global_dir_array: Array<GString>,
            #[export(global_dir)]
            export_global_dir_parray: PackedStringArray,
        }
    };

    let rust = quote! {
        #[derive(GodotClass)]
        #[class(base = Node, init)]
        pub struct PropertyTestsRust {
            #(#rust,)*
            #rust_exports_4_3

            // All the @export_file/dir variants, with GString, Array<GString> and PackedStringArray.
            #[export(file)]
            export_file: GString,
            #[export(file = "*.txt")]
            export_file_wildcard: GString,
            #[export(global_file)]
            export_global_file: GString,
            #[export(global_file = "*.png")]
            export_global_file_wildcard: GString,
            #[export(dir)]
            export_dir: GString,
            #[export(global_dir)]
            export_global_dir: GString,

            #[export(multiline)]
            export_multiline: GString,
            #[export(range = (0.0, 20.0))]
            export_range_float_0_20: f64,
            #[export(range = (-10.0, 20.0, 0.2))]
            export_range_float_neg10_20_02: f64,
            // We can only export ranges of floats currently.
            //  #[export(range = (0, 100, 1, "or_greater", "or_less"))] export_range_int_0_100_1_or_greater_or_less: int,
            #[export(exp_easing)]
            export_exp_easing: f64,
            #[export(color_no_alpha)]
            export_color_no_alpha: Color,
            // Not implemented
            //  #[export(node_path = ("Button", "TouchScreenButton"))] export_node_path_button_touch_screen_button: NodePath,
            #[export(flags = (Fire, Water, Earth, Wind))]
            export_flags_fire_water_earth_wind: i64,
            #[export(flags = (Self = 4, Allies = 8, Foes = 16))]
            export_flags_self_4_allies_8_foes_16: i64,
            #[export(flags_2d_physics)]
            export_flags_2d_physics: i64,
            #[export(flags_2d_render)]
            export_flags_2d_render: i64,
            #[export(flags_2d_navigation)]
            export_flags_2d_navigation: i64,
            #[export(flags_3d_physics)]
            export_flags_3d_physics: i64,
            #[export(flags_3d_render)]
            export_flags_3d_render: i64,
            #[export(flags_3d_navigation)]
            export_flags_3d_navigation: i64,
            #[export(enum = (Warrior, Magician, Thief))]
            export_enum_int_warrior_magician_thief: i64,
            #[export(enum = (Slow = 30, Average = 60, VeryFast = 200))]
            export_enum_int_slow_30_average_60_very_fast_200: i64,
            #[export(enum = (Rebecca, Mary, Leah))]
            export_enum_string_rebecca_mary_leah: GString,
        }
    };

    // `extends`, basic `var` and `@export var` declarations
    let basic_exports = gdscript.join("\n");

    let advanced_exports = r#"
@export_file var export_file: String
@export_file("*.txt") var export_file_wildcard: String
@export_global_file var export_global_file: String
@export_global_file("*.png") var export_global_file_wildcard: String
@export_dir var export_dir: String
@export_global_dir var export_global_dir: String

@export_multiline var export_multiline: String
@export_range(0, 20) var export_range_float_0_20: float
@export_range(-10, 20, 0.2) var export_range_float_neg10_20_02: float
@export_range(0, 100, 1, "or_greater", "or_less") var export_range_int_0_100_1_or_greater_or_less: int
@export_exp_easing var export_exp_easing: float
@export_color_no_alpha var export_color_no_alpha: Color
@export_node_path("Button", "TouchScreenButton") var export_node_path_button_touch_screen_button: NodePath
@export_flags("Fire", "Water", "Earth", "Wind") var export_flags_fire_water_earth_wind: int
@export_flags("Self:4", "Allies:8", "Foes:16") var export_flags_self_4_allies_8_foes_16: int
@export_flags_2d_physics var export_flags_2d_physics: int
@export_flags_2d_render var export_flags_2d_render: int
@export_flags_2d_navigation var export_flags_2d_navigation: int
@export_flags_3d_physics var export_flags_3d_physics: int
@export_flags_3d_render var export_flags_3d_render: int
@export_flags_3d_navigation var export_flags_3d_navigation: int
@export_enum("Warrior", "Magician", "Thief") var export_enum_int_warrior_magician_thief: int
@export_enum("Slow:30", "Average:60", "VeryFast:200") var export_enum_int_slow_30_average_60_very_fast_200: int
@export_enum("Rebecca", "Mary", "Leah") var export_enum_string_rebecca_mary_leah: String
"#;

    // Only available in Godot 4.3+.
    let advanced_exports_4_3 = r#"
@export_storage var export_storage: String
@export_file var export_file_array: Array[String]
@export_file var export_file_parray: PackedStringArray
@export_file("*.txt") var export_file_wildcard_array: Array[String]
@export_file("*.txt") var export_file_wildcard_parray: PackedStringArray
@export_global_file var export_global_file_array: Array[String]
@export_global_file var export_global_file_parray: PackedStringArray
@export_global_file("*.png") var export_global_file_wildcard_array: Array[String]
@export_global_file("*.png") var export_global_file_wildcard_parray: PackedStringArray
@export_dir var export_dir_array: Array[String]
@export_dir var export_dir_parray: PackedStringArray
@export_global_dir var export_global_dir_array: Array[String]
@export_global_dir var export_global_dir_parray: PackedStringArray
    "#;

    let mut gdscript = format!("{basic_exports}\n{advanced_exports}");
    if godot_bindings::since_api("4.3") {
        gdscript.push('\n');
        gdscript.push_str(advanced_exports_4_3);
    }

    PropertyTests { rust, gdscript }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GDScript templating and generation

fn write_gdscript_code(
    inputs: &Vec<Input>,
    in_template_path: &Path,
    out_file_path: &Path,
) -> IoResult {
    let template = std::fs::read_to_string(in_template_path)?;
    let mut file = std::fs::File::create(out_file_path)?;

    // let (mut last_start, mut prev_end) = (0, 0);
    let mut last = 0;

    let ranges = repo_tweak::find_repeated_ranges(&template, "#(", "#)", &[], false);
    for m in ranges {
        file.write_all(&template.as_bytes()[last..m.before_start])?;

        replace_parts(&template[m.start..m.end], inputs, |replacement| {
            file.write_all(replacement.as_bytes())?;
            Ok(())
        })?;

        last = m.after_end;
    }
    file.write_all(&template.as_bytes()[last..])?;

    Ok(())
}

fn replace_parts(
    repeat_part: &str,
    inputs: &Vec<Input>,
    mut visitor: impl FnMut(&str) -> IoResult,
) -> IoResult {
    for input in inputs {
        let Input {
            ident,
            gdscript_ty,
            gdscript_val,
            ..
        } = input;

        let replaced = repeat_part
            .replace("IDENT", ident)
            .replace("TYPE", gdscript_ty)
            .replace("VAL", gdscript_val.as_ref());

        visitor(&replaced)?;
    }

    Ok(())
}
