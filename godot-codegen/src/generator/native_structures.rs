/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::context::Context;
use crate::generator::builtins;
use crate::models::domain::{ExtensionApi, ModName, NativeStructure, TyName};
use crate::util::ident;
use crate::{conv, util, SubmitFn};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;

pub fn generate_native_structures_files(
    api: &ExtensionApi,
    ctx: &mut Context,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let _ = std::fs::remove_dir_all(gen_path);
    std::fs::create_dir_all(gen_path).expect("create native directory");

    let mut modules = vec![];
    for native_structure in api.native_structures.iter() {
        let module_name = ModName::from_godot(&native_structure.name);
        let class_name = TyName::from_godot(&native_structure.name);

        let generated_class = make_native_structure(native_structure, &class_name, ctx);
        let file_contents = generated_class.code;

        let out_path = gen_path.join(format!("{}.rs", module_name.rust_mod));

        submit_fn(out_path, file_contents);

        modules.push(builtins::GeneratedBuiltinModule {
            symbol_ident: class_name.rust_ty.clone(),
            module_name,
        });
    }

    let out_path = gen_path.join("mod.rs");
    let mod_contents = builtins::make_builtin_module_file(modules);

    submit_fn(out_path, mod_contents);
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct NativeStructuresField {
    pub field_name: String,
    pub field_type: String,
    pub array_size: Option<usize>,
}

fn make_native_structure(
    structure: &NativeStructure,
    class_name: &TyName,
    ctx: &mut Context,
) -> builtins::GeneratedBuiltin {
    let class_name = &class_name.rust_ty;

    let imports = util::make_imports();
    let fields = make_native_structure_fields(&structure.format, ctx);
    let doc = format!("[`ToGodot`] and [`FromGodot`] are implemented for `*mut {class_name}` and `*const {class_name}`.");

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        #imports
        use crate::meta::{GodotConvert, FromGodot, ToGodot};

        /// Native structure; can be passed via pointer in APIs that are not exposed to GDScript.
        ///
        #[doc = #doc]
        #[derive(Clone, PartialEq, Debug)]
        #[repr(C)]
        pub struct #class_name {
            #fields
        }

        impl GodotConvert for *mut #class_name {
            type Via = i64;
        }

        impl ToGodot for *mut #class_name {
            fn to_godot(&self) -> Self::Via {
                *self as i64
            }
        }

        impl FromGodot for *mut #class_name {
            fn try_from_godot(via: Self::Via) -> Result<Self, crate::meta::error::ConvertError> {
                Ok(via as Self)
            }
        }

        impl GodotConvert for *const #class_name {
            type Via = i64;
        }

        impl ToGodot for *const #class_name {
            fn to_godot(&self) -> Self::Via {
                *self as i64
            }
        }

        impl FromGodot for *const #class_name {
            fn try_from_godot(via: Self::Via) -> Result<Self, crate::meta::error::ConvertError> {
                Ok(via as Self)
            }
        }
    };
    // note: TypePtr -> ObjectPtr conversion OK?

    builtins::GeneratedBuiltin { code: tokens }
}

fn make_native_structure_fields(format_str: &str, ctx: &mut Context) -> TokenStream {
    let fields = parse_native_structures_format(format_str)
        .expect("Could not parse native_structures format field");

    let field_definitions = fields
        .into_iter()
        .map(|field| make_native_structure_field_definition(field, ctx));

    quote! {
        #( #field_definitions )*
    }
}

fn make_native_structure_field_definition(
    field: NativeStructuresField,
    ctx: &mut Context,
) -> TokenStream {
    let field_type = normalize_native_structure_field_type(&field.field_type);
    let field_type = conv::to_rust_type_abi(&field_type, ctx);

    // Make array if needed.
    let field_type = if let Some(size) = field.array_size {
        quote! { [#field_type; #size] }
    } else {
        quote! { #field_type }
    };

    let field_name = ident(&conv::to_snake_case(&field.field_name));

    quote! {
        pub #field_name: #field_type,
    }
}

fn normalize_native_structure_field_type(field_type: &str) -> String {
    // native_structures uses a different format for enums than the
    // rest of the JSON file. If we detect a scoped field, convert it
    // to the enum format expected by to_rust_type.
    if field_type.contains("::") {
        let with_dot = field_type.replace("::", ".");
        format!("enum::{}", with_dot)
    } else {
        field_type.to_string()
    }
}

/// Parse a string of semicolon-separated C-style type declarations. Fail with `None` if any errors occur.
pub(crate) fn parse_native_structures_format(input: &str) -> Option<Vec<NativeStructuresField>> {
    input
        .split(';')
        .filter(|var| !var.trim().is_empty())
        .map(|var| {
            let mut parts = var.trim().splitn(2, ' ');
            let mut field_type = parts.next()?.to_owned();
            let mut field_name = parts.next()?.to_owned();

            // If the field is a pointer, put the star on the type, not the name.
            if field_name.starts_with('*') {
                field_name.remove(0);
                field_type.push('*');
            }

            // If Godot provided a default value, ignore it.
            // TODO We might use these if we synthetically generate constructors in the future
            if let Some(index) = field_name.find(" = ") {
                field_name.truncate(index);
            }

            // If the field is an array, store array size separately.
            // Not part of type because fixed-size arrays are not a concept in the JSON outside of native structures.
            let mut array_size = None;
            if let Some(index) = field_name.find('[') {
                array_size = Some(field_name[index + 1..field_name.len() - 1].parse().ok()?);
                field_name.truncate(index);
            }

            Some(NativeStructuresField {
                field_name,
                field_type,
                array_size,
            })
        })
        .collect()
}
