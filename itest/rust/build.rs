use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::io::Write;
use std::path::Path;

type IoResult = std::io::Result<()>;

macro_rules! push {
    ($inputs:ident; $ident:ident : $ty:ty = $val:expr; $gd_ty:ident) => {
        $inputs.push(Input {
            ident: stringify!($ident),
            rust_ty: quote! { $ty },
            rust_val: quote! { $val },
            gdscript_ty: stringify!($gd_ty),
        });
    };
}

fn main() {
    let mut inputs = vec![];
    push!(inputs; int: i32 = 42; int);
    push!(inputs; bool: bool = true; bool);

    let methods = generate_rust_methods(&inputs);

    let rust_tokens = quote::quote! {
        #[derive(gdext_macros::GodotClass)]
        #[godot(init)]
        struct GenFfi {}

        #[gdext_macros::godot_api]
        impl GenFfi {
            #(#methods)*
        }
    };

    let rust_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gen"));
    let godot_input_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/input"));
    let godot_output_dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../godot/gen"));

    let rust_file = rust_output_dir.join("rust_ffi.rs");
    let gdscript_template = godot_input_dir.join("GenFfiTests.template.gd");
    let gdscript_file = godot_output_dir.join("GenFfiTests.gd");

    std::fs::create_dir_all(rust_output_dir).expect("create Rust parent dir");
    std::fs::create_dir_all(godot_output_dir).expect("create GDScript parent dir");
    std::fs::write(rust_file, rust_tokens.to_string()).expect("write to Rust file");
    write_gdscript_code(&inputs, &gdscript_template, &gdscript_file)
        .expect("write to GDScript file");

    println!("cargo:rerun-if-changed={}", gdscript_template.display());

}

struct Input {
    ident: &'static str,
    rust_ty: TokenStream,
    rust_val: TokenStream,
    gdscript_ty: &'static str,
}

fn generate_rust_methods(inputs: &Vec<Input>) -> Vec<TokenStream> {
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

            quote! {
                #[godot]
                fn #return_method(&self) -> #rust_ty {
                    #rust_val
                }

                #[godot]
                fn #accept_method(&self, i: #rust_ty) -> bool {
                    i == #rust_val
                }

                #[godot]
                fn #mirror_method(&self, i: #rust_ty) -> #rust_ty {
                    i
                }
            }
        })
        .collect()
}

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
    dbg!(&ranges);
    for m in ranges {
        file.write_all(&template[last..m.before_start].as_bytes())?;

        replace_parts(&template[m.start..m.end], inputs, |replacement| {
            file.write_all(replacement.as_bytes())?;
            Ok(())
        })?;

        last = m.after_end;
    }
    file.write_all(&template[last..].as_bytes())?;

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
            rust_val,
            ..
        } = input;

        let replaced = repeat_part
            .replace("IDENT", ident)
            .replace("TYPE", gdscript_ty)
            .replace("VAL", &rust_val.to_string());

        visitor(&replaced)?;
    }

    Ok(())
}

fn find_repeated_ranges(entire: &str) -> Vec<Match> {
    const START_PAT: &'static str = "#(";
    const END_PAT: &'static str = "#)";

    let mut search_start = 0;
    let mut found = vec![];
    loop {
        if let Some(start) = entire[search_start..].find(START_PAT) {
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
        } else {
            break;
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
