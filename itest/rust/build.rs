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

/// Push with GDScript expr in string
macro_rules! pushs {
    ($inputs:ident; $GDScriptTy:expr, $RustTy:ty, $gdscript_val:expr, $rust_val:expr) => {
        $inputs.push(Input {
            ident: stringify!($RustTy)
                .to_ascii_lowercase()
                .replace("<", "_")
                .replace(">", ""),
            gdscript_ty: stringify!($GDScriptTy),
            gdscript_val: $gdscript_val,
            rust_ty: quote! { $RustTy },
            rust_val: quote! { $rust_val },
        });
    };
}

/// Push simple GDScript expression, outside string
macro_rules! push {
    ($inputs:ident; $GDScriptTy:expr, $RustTy:ty, $val:expr) => {
        push!($inputs; $GDScriptTy, $RustTy, $val, $val);
    };

    ($inputs:ident; $GDScriptTy:expr, $RustTy:ty, $gdscript_val:expr, $rust_val:expr) => {
        pushs!($inputs; $GDScriptTy, $RustTy, stringify!($gdscript_val), $rust_val);
    };
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
    push!(inputs; String, GodotString, "hello", "hello".into());
    push!(inputs; StringName, StringName, &"hello", "hello".into());
    pushs!(inputs; NodePath, NodePath, r#"^"hello""#, "hello".into());
    push!(inputs; Vector2, Vector2, Vector2(12.5, -3.5), Vector2::new(12.5, -3.5));
    push!(inputs; Vector3, Vector3, Vector3(117.5, 100.0, -323.25), Vector3::new(117.5, 100.0, -323.25));
    push!(inputs; Vector4, Vector4, Vector4(-18.5, 24.75, -1.25, 777.875), Vector4::new(-18.5, 24.75, -1.25, 777.875));
    push!(inputs; Vector2i, Vector2i, Vector2i(-2147483648, 2147483647), Vector2i::new(-2147483648, 2147483647));
    push!(inputs; Vector3i, Vector3i, Vector3i(-1, -2147483648, 2147483647), Vector3i::new(-1, -2147483648, 2147483647));
    push!(inputs; Callable, Callable, Callable(), Callable::default());

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
        dict! { "key": 83, (-3): Vector2::new(1.0, 2.0), 0.03: true }
    );

    // Composite
    push!(inputs; int, InstanceId, -1, InstanceId::from_nonzero(0xFFFFFFFFFFFFFFF));
    push!(inputs; Variant, Variant, 123, 123i64.to_variant());

    // EngineEnum
    push!(inputs; int, Error, 0, Error::OK);

    inputs
}

fn main() {
    let inputs = collect_inputs();
    let methods = generate_rust_methods(&inputs);

    let rust_tokens = quote::quote! {
        use godot::builtin::*;
        use godot::obj::InstanceId;
        use godot::engine::global::Error;

        #[derive(godot::bind::GodotClass)]
        #[class(init)]
        struct GenFfi {}

        #[allow(clippy::bool_comparison)] // i == true
        #[godot::bind::godot_api]
        impl GenFfi {
            #(#methods)*
        }
    };

    let rust_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));
    let godot_input_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/input"));
    let godot_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/gen"));

    let rust_file = rust_output_dir.join("gen_ffi.rs");
    let gdscript_template = godot_input_dir.join("GenFfiTests.template.gd");
    let gdscript_file = godot_output_dir.join("GenFfiTests.gd");

    std::fs::create_dir_all(rust_output_dir).expect("create Rust parent dir");
    std::fs::create_dir_all(godot_output_dir).expect("create GDScript parent dir");
    std::fs::write(&rust_file, rust_tokens.to_string()).expect("write to Rust file");
    write_gdscript_code(&inputs, &gdscript_template, &gdscript_file)
        .expect("write to GDScript file");

    println!("cargo:rerun-if-changed={}", gdscript_template.display());

    rustfmt_if_needed(vec![rust_file]);
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

struct Input {
    ident: String,
    gdscript_ty: &'static str,
    gdscript_val: &'static str,
    rust_ty: TokenStream,
    rust_val: TokenStream,
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
