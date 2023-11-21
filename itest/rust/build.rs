/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::io::Write;
use std::path::Path;

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

            #[derive(Debug, Clone, PartialEq)]
            pub struct $name($T);

            impl godot::builtin::meta::GodotConvert for $name {
                type Via = $T;
            }

            impl godot::builtin::meta::ToGodot for $name {
                #[allow(clippy::clone_on_copy)]
                fn to_godot(&self) -> Self::Via {
                    self.0.clone()
                }
            }

            impl godot::builtin::meta::FromGodot for $name {
                fn try_from_godot(via: Self::Via) -> Option<Self> {
                    Some(Self(via))
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
    push!(inputs; String, GString, "hello", "hello".into());
    push!(inputs; StringName, StringName, &"hello", "hello".into());
    pushs!(inputs; NodePath, NodePath, r#"^"hello""#, "hello".into(), true, true, None);
    push!(inputs; Vector2, Vector2, Vector2(12.5, -3.5), Vector2::new(12.5, -3.5));
    push!(inputs; Vector3, Vector3, Vector3(117.5, 100.0, -323.25), Vector3::new(117.5, 100.0, -323.25));
    push!(inputs; Vector4, Vector4, Vector4(-18.5, 24.75, -1.25, 777.875), Vector4::new(-18.5, 24.75, -1.25, 777.875));
    push!(inputs; Vector2i, Vector2i, Vector2i(-2147483648, 2147483647), Vector2i::new(-2147483648, 2147483647));
    push!(inputs; Vector3i, Vector3i, Vector3i(-1, -2147483648, 2147483647), Vector3i::new(-1, -2147483648, 2147483647));
    push!(inputs; Vector4i, Vector4i, Vector4i(-1, -2147483648, 2147483647, 1000), Vector4i::new(-1, -2147483648, 2147483647, 100));
    pushs!(inputs; Callable, Callable, "Callable()", Callable::invalid(), true, false, Some(quote! { Callable::invalid() }));
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
    push_newtype!(inputs; String, NewString(GString), "hello", NewString("hello".into()));
    push_newtype!(inputs; StringName, NewStringName(StringName), &"hello", NewStringName("hello".into()));
    push_newtype!(@s inputs; NodePath, NewNodePath(NodePath), r#"^"hello""#, NewNodePath("hello".into()));
    push_newtype!(inputs; Vector2, NewVector2(Vector2), Vector2(12.5, -3.5), NewVector2(Vector2::new(12.5, -3.5)));
    push_newtype!(inputs; Vector3, NewVector3(Vector3), Vector3(117.5, 100.0, -323.25), NewVector3(Vector3::new(117.5, 100.0, -323.25)));
    push_newtype!(inputs; Vector4, NewVector4(Vector4), Vector4(-18.5, 24.75, -1.25, 777.875), NewVector4(Vector4::new(-18.5, 24.75, -1.25, 777.875)));
    push_newtype!(inputs; Vector2i, NewVector2i(Vector2i), Vector2i(-2147483648, 2147483647), NewVector2i(Vector2i::new(-2147483648, 2147483647)));
    push_newtype!(inputs; Vector3i, NewVector3i(Vector3i), Vector3i(-1, -2147483648, 2147483647), NewVector3i(Vector3i::new(-1, -2147483648, 2147483647)));
    push_newtype!(inputs; Callable, NewCallable(Callable), Callable(), NewCallable(Callable::invalid()));

    // Data structures
    // TODO enable below, when GDScript has typed array literals, or find a hack with eval/lambdas
    /*pushs!(inputs; Array[int], Array<i32>,
        "(func() -> Array[int]: [-7, 12, 40])()",
        array![-7, 12, 40]
    );*/

    push!(inputs; Array, VariantArray,
        [-7, "godot", false, Vector2i(-77, 88)],
        varray![-7, "godot", false, Vector2i::new(-77, 88)]);

    pushs!(inputs; Dictionary, Dictionary,
        r#"{"key": 83, -3: Vector2(1, 2), 0.03: true}"#,
        dict! { "key": 83, (-3): Vector2::new(1.0, 2.0), 0.03: true },
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
        #![allow(clippy::partialeq_to_none)]

        use godot::builtin::*;
        use godot::builtin::meta::*;
        use godot::obj::{Gd, InstanceId};
        use godot::engine::global::Error;
        use godot::engine::{Node, Resource};

        #[derive(godot::bind::GodotClass)]
        #[class(init)]
        struct GenFfi {}

        #[allow(clippy::bool_comparison)] // i == true
        #[godot::bind::godot_api]
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

    let rust_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));
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
    inputs
        .iter()
        .map(|input| {
            let Input {
                ident,
                rust_ty,
                rust_val,
                ..
            } = input;

            let return_method = format_ident!("return_{}", ident);
            let accept_method = format_ident!("accept_{}", ident);
            let mirror_method = format_ident!("mirror_{}", ident);
            let return_static_method = format_ident!("return_static_{}", ident);
            let accept_static_method = format_ident!("accept_static_{}", ident);
            let mirror_static_method = format_ident!("mirror_static_{}", ident);

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
        .collect()
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

        let property = format_ident!("property_{ident}");
        let property_array = format_ident!("property_array_{ident}");
        let export = format_ident!("export_{ident}");
        let export_array = format_ident!("export_array_{ident}");

        let initializer = initializer
            .as_ref()
            .map(|init| quote! { #[init(default = #init)] });

        rust.extend([
            quote! {
                #[var]
                #initializer
                #property: #rust_ty
            },
            quote! { #[var] #property_array: Array<#rust_ty> },
        ]);

        gdscript.extend([
            format!("var {property}: {gdscript_ty}"),
            format!("var {property_array}: Array[{gdscript_ty}]"),
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

    let rust = quote! {
        #[derive(GodotClass)]
        #[class(base = Node, init)]
        pub struct PropertyTestsRust {
            #(#rust,)*

            #[export(file)]
            export_file: GString,
            #[export(file = "*.txt")]
            export_file_wildcard_txt: GString,
            #[export(global_file)]
            export_global_file: GString,
            #[export(global_file = "*.png")]
            export_global_file_wildcard_png: GString,
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

        #[godot_api]
        impl PropertyTestsRust {}
    };

    let gdscript = format!(
        r#"
{}
@export_file var export_file: String
@export_file("*.txt") var export_file_wildcard_txt: String
@export_global_file var export_global_file: String
@export_global_file("*.png") var export_global_file_wildcard_png: String
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
"#,
        gdscript.join("\n")
    );

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

    let ranges = find_repeated_ranges(&template);
    for m in ranges {
        file.write_all(template[last..m.before_start].as_bytes())?;

        replace_parts(&template[m.start..m.end], inputs, |replacement| {
            file.write_all(replacement.as_bytes())?;
            Ok(())
        })?;

        last = m.after_end;
    }
    file.write_all(template[last..].as_bytes())?;

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

fn find_repeated_ranges(entire: &str) -> Vec<Match> {
    const START_PAT: &str = "#(";
    const END_PAT: &str = "#)";

    let mut search_start = 0;
    let mut found = vec![];
    while let Some(start) = entire[search_start..].find(START_PAT) {
        let before_start = search_start + start;
        let start = before_start + START_PAT.len();
        if let Some(end) = entire[start..].find(END_PAT) {
            let end = start + end;
            let after_end = end + END_PAT.len();

            println!("Found {start}..{end}");
            found.push(Match {
                before_start,
                start,
                end,
                after_end,
            });
            search_start = after_end;
        } else {
            panic!("unmatched start pattern without end");
        }
    }

    found
}

#[derive(Debug)]
struct Match {
    before_start: usize,
    start: usize,
    end: usize,
    after_end: usize,
}
