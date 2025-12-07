/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Code generation for gdextension_interface.json types and interface functions.

use proc_macro2::TokenStream;
use quote::quote;

use crate::models::header_json::{HeaderJson, HeaderReturnValue, HeaderType};
use crate::util::{ident, safe_ident};

/// Generate Rust type definitions and function signatures from header JSON.
pub fn generate_header_types(header: &HeaderJson) -> TokenStream {
    let type_definitions = header.types.iter().map(generate_type_definition);

    quote! {
        #( #type_definitions )*
    }
}

/// Generate Rust interface function type signatures.
pub fn generate_interface_functions(header: &HeaderJson) -> TokenStream {
    let function_types = header.interface.iter().map(|func| {
        let name = ident(&func.name);
        let return_type = map_return_type(&func.return_value);
        let params = func.arguments.iter().map(|arg| {
            let param_type = map_type(&arg.type_);
            quote! { #param_type }
        });

        quote! {
            pub type #name = unsafe extern "C" fn(#( #params ),*) -> #return_type;
        }
    });

    quote! {
        #( #function_types )*
    }
}

fn generate_type_definition(type_def: &HeaderType) -> TokenStream {
    match type_def.kind.as_str() {
        "enum" => generate_enum_type(type_def),
        "handle" => generate_handle_type(type_def),
        "alias" => generate_alias_type(type_def),
        "struct" => generate_struct_type(type_def),
        "function" => generate_function_type(type_def),
        _ => TokenStream::new(),
    }
}

fn generate_enum_type(type_def: &HeaderType) -> TokenStream {
    let name = ident(&type_def.name);

    let values = if let Some(vals) = &type_def.values {
        vals.iter()
            .map(|val| {
                let variant_name = ident(&val.name);
                // Use unsuffixed literals to match bindgen output.
                let variant_value = proc_macro2::Literal::i32_unsuffixed(val.value as i32);
                quote! {
                    pub const #variant_name: #name = #variant_value;
                }
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    // Note: C enums are implementation-defined but typically 'int' (signed, usually 32 bit).
    quote! {
        pub type #name = std::ffi::c_int;

        #( #values )*
    }
}

fn generate_handle_type(type_def: &HeaderType) -> TokenStream {
    let name = ident(&type_def.name);

    quote! {
        pub type #name = *mut std::ffi::c_void;
    }
}

fn generate_alias_type(type_def: &HeaderType) -> TokenStream {
    let name = ident(&type_def.name);
    let target_type = type_def.type_.as_ref().map(|t| map_c_type(t));

    quote! {
        pub type #name = #target_type;
    }
}

fn generate_struct_type(type_def: &HeaderType) -> TokenStream {
    let name = ident(&type_def.name);

    let fields = if let Some(members) = &type_def.members {
        members
            .iter()
            .map(|member| {
                let field_name = safe_ident(&member.name);
                let field_type = map_type(&member.type_);
                quote! {
                    pub #field_name: #field_type,
                }
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    quote! {
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        pub struct #name {
            #( #fields )*
        }
    }
}

fn generate_function_type(type_def: &HeaderType) -> TokenStream {
    let name = ident(&type_def.name);
    let return_type = type_def
        .return_value
        .as_ref()
        .map(map_return_type)
        .unwrap_or_else(|| quote! { () });

    let params = if let Some(args) = &type_def.arguments {
        args.iter()
            .map(|arg| {
                let param_type = map_type(&arg.type_);
                quote! { #param_type }
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    quote! {
        pub type #name = unsafe extern "C" fn(#( #params ),*) -> #return_type;
    }
}

fn map_type(godot_type: &str) -> TokenStream {
    // All types go through map_c_type which handles pointers and const
    map_c_type(godot_type)
}

fn map_c_type(c_type: &str) -> TokenStream {
    // Note: C type mappings duplicated in type_conversions.rs; consider extracting to shared utility.
    // Strip const qualifier and remember if it was const
    let (is_const, c_type) = if c_type.starts_with("const ") {
        (true, c_type.strip_prefix("const ").unwrap().trim())
    } else {
        (false, c_type)
    };

    // Handle pointer types
    if c_type.ends_with('*') {
        let base_type = c_type.trim_end_matches('*').trim();
        let inner = map_c_type(base_type);

        return if is_const {
            quote! { *const #inner }
        } else {
            quote! { *mut #inner }
        };
    }

    // Base types - use standard Rust types for simplicity and portability
    match c_type {
        "void" => quote! { () },
        "char" => quote! { std::ffi::c_char },
        "int" => quote! { std::ffi::c_int }, // Only appears once in current JSON (worker_thread_pool_add_native_group_task).
        "int8_t" => quote! { i8 },
        "int16_t" => quote! { i16 },
        "int32_t" => quote! { i32 },
        "int64_t" => quote! { i64 },
        "uint8_t" => quote! { u8 },
        "uint16_t" => quote! { u16 },
        "uint32_t" => quote! { u32 },
        "uint64_t" => quote! { u64 },
        "size_t" => quote! { usize },
        "float" => quote! { f32 },
        "double" => quote! { f64 },
        _ => {
            // Fallback: use the type as-is (should be a GDExtension type)
            let type_ident = ident(c_type);
            quote! { #type_ident }
        }
    }
}

fn map_return_type(return_value: &HeaderReturnValue) -> TokenStream {
    map_type(&return_value.type_)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use nanoserde::DeJson;

    use super::*;

    #[test]
    fn test_generate_header_code() {
        let json_str = std::fs::read_to_string("../json/gdextension_interface.json")
            .expect("failed to read JSON file");
        let header: HeaderJson =
            DeJson::deserialize_json(&json_str).expect("failed to deserialize JSON");

        // Test type generation
        let type_code = generate_header_types(&header);
        let type_str = type_code.to_string();

        // Verify enum generation
        assert!(type_str.contains("pub type GDExtensionVariantType"));
        assert!(type_str.contains("GDEXTENSION_VARIANT_TYPE_NIL"));

        // Verify struct generation
        assert!(type_str.contains("pub struct GDExtensionCallError"));

        // Test interface function generation
        let interface_code = generate_interface_functions(&header);
        let interface_str = interface_code.to_string();

        // Verify function types are generated
        assert!(interface_str.contains("pub type"));
        assert!(interface_str.contains("unsafe extern \"C\" fn"));
    }

    #[test]
    fn write_generated_output_to_file() {
        let json_str = std::fs::read_to_string("../json/gdextension_interface.json")
            .expect("failed to read JSON file");
        let header: HeaderJson =
            DeJson::deserialize_json(&json_str).expect("failed to deserialize JSON");

        // Generate both types and interface functions
        let type_code = generate_header_types(&header);
        let interface_code = generate_interface_functions(&header);

        // Combine into a single file
        let combined = quote! {
            // Generated from gdextension_interface.json

            #type_code

            #interface_code
        };

        // Write to file in the same directory as header_codegen.rs
        let output_path = "src/generator/generated_header.rs";
        std::fs::write(output_path, combined.to_string()).expect("failed to write generated file");

        // Format with rustfmt
        std::process::Command::new("rustfmt")
            .arg(output_path)
            .status()
            .expect("failed to run rustfmt");
    }
}
