/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Code generation for gdextension_interface.json types and interface functions.
//!
//! Not yet integrated into the build pipeline; currently invoked only from tests.

// All public functions are currently only called from #[cfg(test)].
#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::quote;

use crate::models::header_json::{HeaderJson, HeaderReturnValue, HeaderType};
use crate::util::{ident, safe_ident};

/// Generate Rust type definitions from header JSON.
pub fn generate_header_types(header: &HeaderJson) -> TokenStream {
    let type_definitions = header.types.iter().map(generate_type_definition);

    quote! {
        #( #type_definitions )*
    }
}

/// Generate the GDExtensionInterface struct with function pointer fields.
pub fn generate_gdextension_interface(header: &HeaderJson) -> TokenStream {
    let fields = header.interface.iter().map(|func| {
        let field_name = ident(&func.name);
        let func_ptr_ty = generate_function_pointer_type(func);

        // Build comprehensive documentation including parameter docs
        let mut doc_parts = Vec::new();

        // Main description
        if !func.description.is_empty() {
            doc_parts.push(func.description.join("\n"));
        }

        // Parameter documentation
        if !func.arguments.is_empty() {
            let mut has_documented_params = false;
            let mut param_docs = Vec::new();

            for arg in &func.arguments {
                if let Some(name) = &arg.name {
                    if let Some(desc_lines) = &arg.description {
                        if !desc_lines.is_empty() {
                            has_documented_params = true;
                            let param_doc = format!("- `{}` - {}", name, desc_lines.join(" "));
                            param_docs.push(param_doc);
                        }
                    }
                }
            }

            if has_documented_params {
                doc_parts.push(String::new()); // Empty line before section
                doc_parts.push("## Parameters".to_string());
                doc_parts.extend(param_docs);
            }
        }

        // Return value documentation
        // Only show if non-void and has description
        let is_void = func.return_value.type_ == "void";
        if !is_void {
            if let Some(ret_desc_lines) = &func.return_value.description {
                if !ret_desc_lines.is_empty() {
                    doc_parts.push(String::new()); // Empty line before return
                    doc_parts.push("## Return value".to_string());
                    doc_parts.push(ret_desc_lines.join(" "));
                }
            }
        }

        if !doc_parts.is_empty() {
            let doc_str = doc_parts.join("\n");
            quote! {
                #[doc = #doc_str]
                pub #field_name: #func_ptr_ty,
            }
        } else {
            quote! {
                pub #field_name: #func_ptr_ty,
            }
        }
    });

    quote! {
        #[repr(C)]
        pub struct GDExtensionInterface {
            #( #fields )*
        }
    }
}

/// Generate an inline function pointer type for a single interface function.
fn generate_function_pointer_type(
    func: &crate::models::header_json::HeaderInterfaceFunction,
) -> TokenStream {
    let params = generate_params(&func.arguments);
    let return_clause = map_return_clause(&func.return_value);

    quote! {
        Option<unsafe extern "C" fn(#( #params ),*) #return_clause>
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

    if type_def.is_const == Some(true) {
        quote! {
            pub type #name = *const std::ffi::c_void;
        }
    } else {
        quote! {
            pub type #name = *mut std::ffi::c_void;
        }
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
                let field_type = map_c_type(&member.type_);
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
    let return_clause = type_def
        .return_value
        .as_ref()
        .map(map_return_clause)
        .unwrap_or_default();

    let params = type_def
        .arguments
        .as_ref()
        .map(|args| generate_params(args))
        .unwrap_or_default();

    quote! {
        pub type #name = unsafe extern "C" fn(#( #params ),*) #return_clause;
    }
}

/// Generate named parameter tokens from a list of arguments.
fn generate_params(args: &[crate::models::header_json::HeaderArgument]) -> Vec<TokenStream> {
    args.iter()
        .map(|arg| {
            let param_type = map_c_type(&arg.type_);
            if let Some(param_name_str) = &arg.name {
                if param_name_str.is_empty() {
                    quote! { #param_type }
                } else {
                    let param_name = safe_ident(param_name_str);
                    quote! { #param_name: #param_type }
                }
            } else {
                quote! { #param_type }
            }
        })
        .collect()
}

fn map_c_type(c_type: &str) -> TokenStream {
    // Code duplication: pointer parsing logic - conv/type_conversions.rs::to_rust_type_uncached().

    // Strip const qualifier and remember if it was const
    let (is_const, c_type) = if c_type.starts_with("const ") {
        (true, c_type.strip_prefix("const ").unwrap().trim())
    } else {
        (false, c_type)
    };

    // Handle pointer types
    if c_type.ends_with('*') {
        let base_type = c_type.trim_end_matches('*').trim();
        let inner = map_c_type_as_pointee(base_type);

        return if is_const {
            quote! { *const #inner }
        } else {
            quote! { *mut #inner }
        };
    }

    // Base types
    map_c_base_type(c_type)
}

/// Map a C type that appears as the pointee of a pointer.
/// `void` maps to `std::ffi::c_void` (not `()`) so that `void*` becomes `*mut c_void`.
fn map_c_type_as_pointee(c_type: &str) -> TokenStream {
    if c_type == "void" {
        quote! { std::ffi::c_void }
    } else {
        map_c_type(c_type)
    }
}

/// Map a C base type (non-pointer) to a Rust type.
fn map_c_base_type(c_type: &str) -> TokenStream {
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

/// Map a return value to a return clause (`-> T`), or empty for void.
fn map_return_clause(return_value: &HeaderReturnValue) -> TokenStream {
    if return_value.type_ == "void" {
        TokenStream::new()
    } else {
        let ty = map_c_type(&return_value.type_);
        quote! { -> #ty }
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use nanoserde::DeJson;

    use super::*;

    fn load_header() -> HeaderJson {
        let json_str = std::fs::read_to_string("../json/gdextension_interface.json")
            .expect("failed to read JSON file");
        DeJson::deserialize_json(&json_str).expect("failed to deserialize JSON")
    }

    #[test]
    fn test_generate_header_code() {
        let header = load_header();

        // Test type generation
        let type_code = generate_header_types(&header);
        let type_str = type_code.to_string();

        // Verify enum generation
        assert!(type_str.contains("pub type GDExtensionVariantType"));
        assert!(type_str.contains("GDEXTENSION_VARIANT_TYPE_NIL"));

        // Verify struct generation
        assert!(type_str.contains("pub struct GDExtensionCallError"));

        // Verify const handles use *const
        assert!(type_str.contains("* const std :: ffi :: c_void"));

        // Verify GDExtensionInterface struct generation
        let struct_code = generate_gdextension_interface(&header);
        let struct_str = struct_code.to_string();
        assert!(struct_str.contains("# [repr (C)]"));
        assert!(struct_str.contains("pub struct GDExtensionInterface"));
        assert!(struct_str.contains("Option <"));
    }

    #[test]
    fn write_generated_header_to_file() {
        let header = load_header();

        let type_code = generate_header_types(&header);
        let struct_code = generate_gdextension_interface(&header);

        let combined = quote! {
            // Generated from gdextension_interface.json

            #type_code

            #struct_code
        };

        let output_path = "src/generator/generated_header.rs";
        std::fs::write(output_path, combined.to_string()).expect("failed to write generated file");

        std::process::Command::new("rustfmt")
            .arg(output_path)
            .status()
            .expect("failed to run rustfmt");
    }
}
