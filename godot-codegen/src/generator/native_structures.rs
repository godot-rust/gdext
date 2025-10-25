/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::context::Context;
use crate::generator::builtins;
use crate::models::domain::{ExtensionApi, ModName, NativeStructure, TyName};
use crate::util::ident;
use crate::{conv, special_cases, util, SubmitFn};

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
        // Some may be excluded in minimal codegen, because they hold codegen-excluded classes as fields.
        if special_cases::is_native_struct_excluded(&native_structure.name) {
            continue;
        }

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
    /// Identifier for the field name, e.g. `collider`.
    pub field_name: String,

    /// Type which has a raw format that is latter mapped to `RustTy`.
    ///
    /// Corresponds to other Godot type names, e.g. `Object*` or `enum::TextServer.Direction`.
    pub field_type: String,

    /// If the field is an array, this contains the number of elements.
    pub array_size: Option<usize>,
}

fn make_native_structure(
    structure: &NativeStructure,
    class_name: &TyName,
    ctx: &mut Context,
) -> builtins::GeneratedBuiltin {
    let class_name = &class_name.rust_ty;

    // TODO for native structures holding object pointers, we should encapsulate the raw object, as follows:
    // - Make raw object pointer field private (and maybe also ID field, to keep in sync).
    // - Add constructor that takes Gd<Object> instead of ID/raw-ptr fields.
    // - Add getter and setter methods with Gd<Object> (already present).
    // - Add Drop impl, which decrements refcount if the constructor was used, and does nothing if FromGodot pointer conversion was used.

    let imports = util::make_imports();
    let (fields, methods) = make_native_structure_fields_and_methods(structure, ctx);
    let doc = format!("[`ToGodot`] and [`FromGodot`] are implemented for `*mut {class_name}` and `*const {class_name}`.");

    // mod re_export needed, because class should not appear inside the file module, and we can't re-export private struct as pub
    let tokens = quote! {
        #imports
        use std::ffi::c_void; // for opaque object pointer fields
        use crate::meta::{GodotConvert, FromGodot, ToGodot};

        /// Native structure; can be passed via pointer in APIs that are not exposed to GDScript.
        ///
        #[doc = #doc]
        #[derive(Clone, PartialEq, Debug)]
        #[repr(C)]
        pub struct #class_name {
            #fields
        }

        impl #class_name {
            #methods
        }

        impl GodotConvert for *mut #class_name {
            type Via = i64;
        }

        impl ToGodot for *mut #class_name {
            type Pass = crate::meta::ByValue;

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
            type Pass = crate::meta::ByValue;

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

fn make_native_structure_fields_and_methods(
    structure: &NativeStructure,
    ctx: &mut Context,
) -> (TokenStream, TokenStream) {
    let fields = parse_native_structures_format(&structure.format)
        .expect("Could not parse native_structures format field");

    let mut field_definitions = vec![];
    let mut accessors = vec![];

    for field in fields {
        let (field_def, accessor) = make_native_structure_field_and_accessor(field, ctx);
        field_definitions.push(field_def);
        if let Some(accessor) = accessor {
            accessors.push(accessor);
        }
    }

    let fields = quote! { #( #field_definitions )* };
    let methods = quote! { #( #accessors )* };

    (fields, methods)
}

fn make_native_structure_field_and_accessor(
    field: NativeStructuresField,
    ctx: &mut Context,
) -> (TokenStream, Option<TokenStream>) {
    let field_type = normalize_native_structure_field_type(&field.field_type);
    let (field_type, is_object_ptr) = conv::to_rust_type_abi(&field_type, ctx);

    // Make array if needed.
    let field_type = if let Some(size) = field.array_size {
        quote! { [#field_type; #size] }
    } else {
        quote! { #field_type }
    };

    let snake_field_name = ident(&conv::to_snake_case(&field.field_name));

    let (field_name, accessor);
    if is_object_ptr {
        // Highlight that the pointer field is internal/opaque.
        field_name = format_ident!("raw_{}_ptr", snake_field_name);

        // Generate method that converts from instance ID.
        let getter_name = format_ident!("get_{}", snake_field_name);
        let setter_name = format_ident!("set_{}", snake_field_name);
        let id_field_name = format_ident!("{}_id", snake_field_name);

        // Current native structures treat all object pointers as Object (even if concrete ones like `collider` might be Node).
        // Having node also avoids the lifetime issue with reference-counted objects (unclear if gdext should increment refcount or not).
        // If other fields use different classes in the future, we'll need to update this.
        accessor = Some(quote! {
            /// Returns the object as a `Gd<Node>`, or `None` if it no longer exists.
            pub fn #getter_name(&self) -> Option<Gd<Object>> {
                crate::obj::InstanceId::try_from_u64(self.#id_field_name.id)
                    .and_then(|id| Gd::try_from_instance_id(id).ok())

                // Sanity check for consistency (if Some(...)):
                // let ptr = self.#field_name as sys::GDExtensionObjectPtr;
                // unsafe { Gd::from_obj_sys(ptr) }
            }

            /// Sets the object from a `Gd` pointer holding `Node` or a derived class.
            pub fn #setter_name<T>(&mut self, #snake_field_name: Gd<T>)
            where T: crate::obj::Inherits<Object> {
                use crate::meta::GodotType as _;

                let obj = #snake_field_name.upcast();

                #[cfg(safeguards_balanced)]
                assert!(obj.is_instance_valid(), "provided node is dead");

                let id = obj.instance_id().to_u64();

                self.#id_field_name = ObjectId { id };
                self.#field_name = obj.obj_sys() as *mut std::ffi::c_void;
            }
        });
    } else {
        field_name = snake_field_name;
        accessor = None;
    };

    let field_def = quote! {
        pub #field_name: #field_type,
    };

    (field_def, accessor)
}

/// Native structures use a different format for enums than the rest of the JSON file.
/// If we detect a scoped field, convert it to the enum format expected by to_rust_type().
///
/// Example: `TextServer::Direction` -> `enum::TextServer.Direction`.
fn normalize_native_structure_field_type(field_type: &str) -> String {
    if field_type.contains("::") {
        let with_dot = field_type.replace("::", ".");
        format!("enum::{with_dot}")
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
            // Not part of type because fixed-size arrays are not a concept in the JSON outside native structures.
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
