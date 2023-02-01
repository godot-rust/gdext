/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::{to_pascal_case, to_rust_type, to_snake_case};
use crate::{ident, util, Context};

struct CentralItems {
    opaque_types: Vec<TokenStream>,
    variant_ty_enumerators_pascal: Vec<Ident>,
    variant_ty_enumerators_rust: Vec<TokenStream>,
    variant_ty_enumerators_ord: Vec<Literal>,
    variant_op_enumerators_pascal: Vec<Ident>,
    variant_op_enumerators_ord: Vec<Literal>,
    variant_fn_decls: Vec<TokenStream>,
    variant_fn_inits: Vec<TokenStream>,
    global_enum_defs: Vec<TokenStream>,
}

pub(crate) struct TypeNames {
    /// Name in JSON: "int" or "PackedVector2Array"
    pub json_builtin_name: String,

    /// "packed_vector2_array"
    pub snake_case: String,

    /// "PACKED_VECTOR2_ARRAY"
    //pub shout_case: String,

    /// GDEXTENSION_VARIANT_TYPE_PACKED_VECTOR2_ARRAY
    pub sys_variant_type: Ident,
}

/// Allows collecting all builtin TypeNames before generating methods
pub(crate) struct BuiltinTypeInfo<'a> {
    pub value: i32,
    pub type_names: TypeNames,

    /// If `variant_get_ptr_destructor` returns a non-null function pointer for this type.
    /// List is directly sourced from extension_api.json (information would also be in variant_destruct.cpp).
    pub has_destructor: bool,
    pub constructors: Option<&'a Vec<Constructor>>,
    pub operators: Option<&'a Vec<Operator>>,
}

pub(crate) fn generate_sys_central_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    build_config: &str,
    sys_gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let central_items = make_central_items(api, build_config, ctx);
    let sys_code = make_sys_code(&central_items);

    write_file(sys_gen_path, "central.rs", sys_code, out_files);
}

pub(crate) fn generate_sys_mod_file(core_gen_path: &Path, out_files: &mut Vec<PathBuf>) {
    let code = quote! {
        pub mod central;
        pub mod gdextension_interface;
    };

    write_file(core_gen_path, "mod.rs", code.to_string(), out_files);
}

pub(crate) fn generate_core_mod_file(core_gen_path: &Path, out_files: &mut Vec<PathBuf>) {
    // When invoked by another crate during unit-test (not integration test), don't run generator
    let code = quote! {
        pub mod central;
        pub mod classes;
        pub mod builtin_classes;
        pub mod utilities;
    };

    write_file(core_gen_path, "mod.rs", code.to_string(), out_files);
}

pub(crate) fn generate_core_central_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    build_config: &str,
    core_gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let central_items = make_central_items(api, build_config, ctx);
    let core_code = make_core_code(&central_items);

    write_file(core_gen_path, "central.rs", core_code, out_files);
}

pub(crate) fn write_file(
    gen_path: &Path,
    filename: &str,
    code: String,
    out_files: &mut Vec<PathBuf>,
) {
    let _ = std::fs::create_dir_all(gen_path);
    let out_path = gen_path.join(filename);

    std::fs::write(&out_path, code).unwrap_or_else(|e| {
        panic!(
            "failed to write code file to {};\n\t{}",
            out_path.display(),
            e
        )
    });
    out_files.push(out_path);
}

fn make_sys_code(central_items: &CentralItems) -> String {
    let CentralItems {
        opaque_types,
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_ord,
        variant_op_enumerators_pascal,
        variant_op_enumerators_ord,
        variant_fn_decls,
        variant_fn_inits,
        ..
    } = central_items;

    let sys_tokens = quote! {
        use crate::{GDExtensionVariantPtr, GDExtensionTypePtr, GDExtensionConstTypePtr, GodotFfi, ffi_methods};

        pub mod types {
            #(#opaque_types)*
        }

        pub struct GlobalMethodTable {
            #(#variant_fn_decls)*
        }

        impl GlobalMethodTable {
            pub(crate) unsafe fn new(interface: &crate::GDExtensionInterface) -> Self {
                Self {
                    #(#variant_fn_inits)*
                }
            }
        }

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantType {
            Nil = 0,
            #(
                #variant_ty_enumerators_pascal = #variant_ty_enumerators_ord,
            )*
        }

        impl VariantType {
            #[doc(hidden)]
            pub fn from_sys(enumerator: crate::GDExtensionVariantType) -> Self {
                // Annoying, but only stable alternative is transmute(), which dictates enum size
                match enumerator {
                    0 => Self::Nil,
                    #(
                        #variant_ty_enumerators_ord => Self::#variant_ty_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant type {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> crate::GDExtensionVariantType {
                self as _
            }
        }

        impl GodotFfi for VariantType {
            ffi_methods! { type GDExtensionTypePtr = *mut Self; .. }
        }

        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
        #[repr(i32)]
        pub enum VariantOperator {
            #(
                #variant_op_enumerators_pascal = #variant_op_enumerators_ord,
            )*
        }

        impl VariantOperator {
            #[doc(hidden)]
            pub fn from_sys(enumerator: crate::GDExtensionVariantOperator) -> Self {
                match enumerator {
                    #(
                        #variant_op_enumerators_ord => Self::#variant_op_enumerators_pascal,
                    )*
                    _ => unreachable!("invalid variant operator {}", enumerator)
                }
            }

            #[doc(hidden)]
            pub fn sys(self) -> crate::GDExtensionVariantOperator {
                self as _
            }
        }

        impl GodotFfi for VariantOperator {
            ffi_methods! { type GDExtensionTypePtr = *mut Self; .. }
        }
    };

    sys_tokens.to_string()
}

fn make_core_code(central_items: &CentralItems) -> String {
    let CentralItems {
        variant_ty_enumerators_pascal,
        variant_ty_enumerators_rust,
        global_enum_defs,
        ..
    } = central_items;

    // TODO impl Clone, Debug, PartialEq, PartialOrd, Hash for VariantDispatch
    // TODO could use try_to().unwrap_unchecked(), since type is already verified. Also directly overload from_variant().
    // But this requires that all the variant types support this
    let core_tokens = quote! {
        use crate::builtin::*;
        use crate::engine::Object;
        use crate::obj::Gd;

        #[allow(dead_code)]
        pub enum VariantDispatch {
            Nil,
            #(
                #variant_ty_enumerators_pascal(#variant_ty_enumerators_rust),
            )*
        }

        #[cfg(FALSE)]
        impl FromVariant for VariantDispatch {
            fn try_from_variant(variant: &Variant) -> Result<Self, VariantConversionError> {
                let dispatch = match variant.get_type() {
                    VariantType::Nil => Self::Nil,
                    #(
                        VariantType::#variant_ty_enumerators_pascal
                            => Self::#variant_ty_enumerators_pascal(variant.to::<#variant_ty_enumerators_rust>()),
                    )*
                };

                Ok(dispatch)
            }
        }

        pub mod global {
            use crate::sys;
            #( #global_enum_defs )*
        }
    };

    core_tokens.to_string()
}

fn make_central_items(api: &ExtensionApi, build_config: &str, ctx: &mut Context) -> CentralItems {
    let mut opaque_types = vec![];
    for class in &api.builtin_class_sizes {
        if class.build_configuration == build_config {
            for ClassSize { name, size } in &class.sizes {
                opaque_types.push(make_opaque_type(name, *size));
            }

            break;
        }
    }

    let builtin_types_map = collect_builtin_types(api);
    let variant_operators = collect_variant_operators(api);

    // Generate builtin methods, now with info for all types available.
    // Separate vectors because that makes usage in quote! easier.
    let len = builtin_types_map.len();

    let mut result = CentralItems {
        opaque_types,
        variant_ty_enumerators_pascal: Vec::with_capacity(len),
        variant_ty_enumerators_rust: Vec::with_capacity(len),
        variant_ty_enumerators_ord: Vec::with_capacity(len),
        variant_op_enumerators_pascal: Vec::new(),
        variant_op_enumerators_ord: Vec::new(),
        variant_fn_decls: Vec::with_capacity(len),
        variant_fn_inits: Vec::with_capacity(len),
        global_enum_defs: Vec::new(),
    };

    let mut builtin_types: Vec<_> = builtin_types_map.values().collect();
    builtin_types.sort_by_key(|info| info.value);

    // Note: NIL is not part of this iteration, it will be added manually
    for ty in builtin_types {
        // Note: both are token streams, containing multiple function declarations/initializations
        let (decls, inits) = make_variant_fns(
            &ty.type_names,
            ty.has_destructor,
            ty.constructors,
            ty.operators,
            &builtin_types_map,
        );

        let (pascal_name, rust_ty, ord) = make_enumerator(&ty.type_names, ty.value, ctx);

        result.variant_ty_enumerators_pascal.push(pascal_name);
        result.variant_ty_enumerators_rust.push(rust_ty);
        result.variant_ty_enumerators_ord.push(ord);
        result.variant_fn_decls.push(decls);
        result.variant_fn_inits.push(inits);
    }

    for op in variant_operators {
        let name = op
            .name
            .strip_prefix("OP_")
            .expect("expected `OP_` prefix for variant operators");

        if name == "MAX" {
            continue;
        }

        result
            .variant_op_enumerators_pascal
            .push(ident(&shout_to_pascal(name)));
        result
            .variant_op_enumerators_ord
            .push(Literal::i32_unsuffixed(op.value));
    }

    for enum_ in api.global_enums.iter() {
        // Skip those enums which are already explicitly handled
        if matches!(enum_.name.as_str(), "Variant.Type" | "Variant.Operator") {
            continue;
        }

        let def = util::make_enum_definition(enum_);
        result.global_enum_defs.push(def);
    }

    result
}

/// Creates a map from "normalized" class names (lowercase without underscore, makes it easy to map from different conventions)
/// to meta type information, including all the type name variants
fn collect_builtin_classes(api: &ExtensionApi) -> HashMap<String, &BuiltinClass> {
    let mut class_map = HashMap::new();
    for class in &api.builtin_classes {
        let normalized_name = class.name.to_ascii_lowercase();

        class_map.insert(normalized_name, class);
    }

    class_map
}

/// Returns map from "PackedStringArray" to all the info
pub(crate) fn collect_builtin_types(api: &ExtensionApi) -> HashMap<String, BuiltinTypeInfo<'_>> {
    let class_map = collect_builtin_classes(api);

    let variant_type_enum = api
        .global_enums
        .iter()
        .find(|e| &e.name == "Variant.Type")
        .expect("missing enum for VariantType in JSON");

    // Collect all `BuiltinTypeInfo`s
    let mut builtin_types_map = HashMap::new();
    for ty in &variant_type_enum.values {
        let shout_case = ty
            .name
            .strip_prefix("TYPE_")
            .expect("enum name begins with 'TYPE_'");

        if shout_case == "NIL" || shout_case == "MAX" {
            continue;
        }

        // Lowercase without underscore, to map SHOUTY_CASE to shoutycase
        let normalized = shout_case.to_ascii_lowercase().replace('_', "");

        // TODO cut down on the number of cached functions generated
        // e.g. there's no point in providing operator< for int
        let class_name: String;
        let has_destructor: bool;
        let constructors: Option<&Vec<Constructor>>;
        let operators: Option<&Vec<Operator>>;
        if let Some(class) = class_map.get(&normalized) {
            class_name = class.name.clone();
            has_destructor = class.has_destructor;
            constructors = Some(&class.constructors);
            operators = Some(&class.operators);
        } else {
            assert_eq!(normalized, "object");
            class_name = "Object".to_string();
            has_destructor = false;
            constructors = None;
            operators = None;
        }

        let type_names = TypeNames {
            json_builtin_name: class_name.clone(),
            snake_case: to_snake_case(&class_name),
            //shout_case: shout_case.to_string(),
            sys_variant_type: format_ident!("GDEXTENSION_VARIANT_TYPE_{}", shout_case),
        };

        let value = ty.value;

        builtin_types_map.insert(
            type_names.json_builtin_name.clone(),
            BuiltinTypeInfo {
                value,
                type_names,
                has_destructor,
                constructors,
                operators,
            },
        );
    }
    builtin_types_map
}

fn collect_variant_operators(api: &ExtensionApi) -> Vec<&EnumConstant> {
    let variant_operator_enum = api
        .global_enums
        .iter()
        .find(|e| &e.name == "Variant.Operator")
        .expect("missing enum for VariantOperator in JSON");

    variant_operator_enum.values.iter().collect()
}

fn make_enumerator(
    type_names: &TypeNames,
    value: i32,
    ctx: &mut Context,
) -> (Ident, TokenStream, Literal) {
    let enumerator_name = &type_names.json_builtin_name;
    let pascal_name = to_pascal_case(enumerator_name);
    let rust_ty = to_rust_type(enumerator_name, ctx);
    let ord = Literal::i32_unsuffixed(value);

    (ident(&pascal_name), rust_ty.to_token_stream(), ord)
}

fn make_opaque_type(name: &str, size: usize) -> TokenStream {
    let name = to_pascal_case(name);
    let (first, rest) = name.split_at(1);

    // Capitalize: "int" -> "Int"
    let ident = format_ident!("Opaque{}{}", first.to_ascii_uppercase(), rest);
    quote! {
        pub type #ident = crate::opaque::Opaque<#size>;
    }
}

fn make_variant_fns(
    type_names: &TypeNames,
    has_destructor: bool,
    constructors: Option<&Vec<Constructor>>,
    operators: Option<&Vec<Operator>>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (TokenStream, TokenStream) {
    let (construct_decls, construct_inits) =
        make_construct_fns(type_names, constructors, builtin_types);
    let (destroy_decls, destroy_inits) = make_destroy_fns(type_names, has_destructor);
    let (op_eq_decls, op_eq_inits) = make_operator_fns(type_names, operators, "==", "EQUAL");
    let (op_lt_decls, op_lt_inits) = make_operator_fns(type_names, operators, "<", "LESS");

    let to_variant = format_ident!("{}_to_variant", type_names.snake_case);
    let from_variant = format_ident!("{}_from_variant", type_names.snake_case);

    let to_variant_error = format_load_error(&to_variant);
    let from_variant_error = format_load_error(&from_variant);

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate:: #variant_type };

    // Field declaration
    let decl = quote! {
        pub #to_variant: unsafe extern "C" fn(GDExtensionVariantPtr, GDExtensionTypePtr),
        pub #from_variant: unsafe extern "C" fn(GDExtensionTypePtr, GDExtensionVariantPtr),
        #op_eq_decls
        #op_lt_decls
        #construct_decls
        #destroy_decls
    };

    // Field initialization in new()
    let init = quote! {
        #to_variant: {
            let ctor_fn = interface.get_variant_from_type_constructor.unwrap();
            ctor_fn(#variant_type).expect(#to_variant_error)
        },
        #from_variant:  {
            let ctor_fn = interface.get_variant_to_type_constructor.unwrap();
            ctor_fn(#variant_type).expect(#from_variant_error)
        },
        #op_eq_inits
        #op_lt_inits
        #construct_inits
        #destroy_inits
    };

    (decl, init)
}

fn make_construct_fns(
    type_names: &TypeNames,
    constructors: Option<&Vec<Constructor>>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (TokenStream, TokenStream) {
    let constructors = match constructors {
        Some(c) => c,
        None => return (TokenStream::new(), TokenStream::new()),
    };

    if is_trivial(type_names) {
        return (TokenStream::new(), TokenStream::new());
    }

    // Constructor vec layout:
    //   [0]: default constructor
    //   [1]: copy constructor
    //   [2]: (optional) typically the most common conversion constructor (e.g. StringName -> String)
    //  rest: (optional) other conversion constructors and multi-arg constructors (e.g. Vector3(x, y, z))

    // Sanity checks -- ensure format is as expected
    for (i, c) in constructors.iter().enumerate() {
        assert_eq!(i, c.index);
    }

    assert!(constructors[0].arguments.is_none());

    if let Some(args) = &constructors[1].arguments {
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "from");
        assert_eq!(args[0].type_, type_names.json_builtin_name);
    } else {
        panic!(
            "type {}: no constructor args found for copy constructor",
            type_names.json_builtin_name
        );
    }

    let construct_default = format_ident!("{}_construct_default", type_names.snake_case);
    let construct_copy = format_ident!("{}_construct_copy", type_names.snake_case);
    let construct_default_error = format_load_error(&construct_default);
    let construct_copy_error = format_load_error(&construct_copy);
    let variant_type = &type_names.sys_variant_type;

    let (construct_extra_decls, construct_extra_inits) =
        make_extra_constructors(type_names, constructors, builtin_types);

    // Generic signature:  fn(base: GDExtensionTypePtr, args: *const GDExtensionTypePtr)
    let decls = quote! {
        pub #construct_default: unsafe extern "C" fn(GDExtensionTypePtr, *const GDExtensionConstTypePtr),
        pub #construct_copy: unsafe extern "C" fn(GDExtensionTypePtr, *const GDExtensionConstTypePtr),
        #(#construct_extra_decls)*
    };

    let inits = quote! {
        #construct_default: {
            let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
            ctor_fn(crate:: #variant_type, 0i32).expect(#construct_default_error)
        },
        #construct_copy: {
            let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
            ctor_fn(crate:: #variant_type, 1i32).expect(#construct_copy_error)
        },
        #(#construct_extra_inits)*
    };

    (decls, inits)
}

/// Lists special cases for useful constructors
fn make_extra_constructors(
    type_names: &TypeNames,
    constructors: &Vec<Constructor>,
    builtin_types: &HashMap<String, BuiltinTypeInfo>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut extra_decls = Vec::with_capacity(constructors.len() - 2);
    let mut extra_inits = Vec::with_capacity(constructors.len() - 2);
    let variant_type = &type_names.sys_variant_type;

    for (i, ctor) in constructors.iter().enumerate().skip(2) {
        if let Some(args) = &ctor.arguments {
            let type_name = &type_names.snake_case;
            let ident = if args.len() == 1 && args[0].name == "from" {
                // Conversion constructor is named according to the source type
                // String(NodePath from) => string_from_node_path
                let arg_type = &builtin_types[&args[0].type_].type_names.snake_case;
                format_ident!("{type_name}_from_{arg_type}")
            } else {
                // Type-specific constructor is named according to the argument names
                // Vector3(float x, float y, float z) => vector3_from_x_y_z
                let mut arg_names = args
                    .iter()
                    .fold(String::new(), |acc, arg| acc + &arg.name + "_");
                arg_names.pop(); // remove trailing '_'
                format_ident!("{type_name}_from_{arg_names}")
            };

            let err = format_load_error(&ident);
            extra_decls.push(quote! {
                pub #ident: unsafe extern "C" fn(GDExtensionTypePtr, *const GDExtensionConstTypePtr),
            });

            let i = i as i32;
            extra_inits.push(quote! {
               #ident: {
                    let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
                    ctor_fn(crate:: #variant_type, #i).expect(#err)
                },
            });
        }
    }

    (extra_decls, extra_inits)
}

fn make_destroy_fns(type_names: &TypeNames, has_destructor: bool) -> (TokenStream, TokenStream) {
    if !has_destructor || is_trivial(type_names) {
        return (TokenStream::new(), TokenStream::new());
    }

    let destroy = format_ident!("{}_destroy", type_names.snake_case);
    let variant_type = &type_names.sys_variant_type;

    let decls = quote! {
        pub #destroy: unsafe extern "C" fn(GDExtensionTypePtr),
    };

    let inits = quote! {
        #destroy: {
            let dtor_fn = interface.variant_get_ptr_destructor.unwrap();
            dtor_fn(crate:: #variant_type).unwrap()
        },
    };

    (decls, inits)
}

fn make_operator_fns(
    type_names: &TypeNames,
    operators: Option<&Vec<Operator>>,
    json_name: &str,
    sys_name: &str,
) -> (TokenStream, TokenStream) {
    if operators.is_none()
        || !operators.unwrap().iter().any(|op| op.name == json_name)
        || is_trivial(type_names)
    {
        return (TokenStream::new(), TokenStream::new());
    }

    let operator = format_ident!(
        "{}_operator_{}",
        type_names.snake_case,
        sys_name.to_ascii_lowercase()
    );
    let error = format_load_error(&operator);

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate:: #variant_type };
    let sys_ident = format_ident!("GDEXTENSION_VARIANT_OP_{}", sys_name);

    // Field declaration
    let decl = quote! {
        pub #operator: unsafe extern "C" fn(GDExtensionConstTypePtr, GDExtensionConstTypePtr, GDExtensionTypePtr),
    };

    // Field initialization in new()
    let init = quote! {
        #operator: {
            let op_finder = interface.variant_get_ptr_operator_evaluator.unwrap();
            op_finder(
                crate::#sys_ident,
                #variant_type,
                #variant_type,
            ).expect(#error)
        },
    };

    (decl, init)
}

fn format_load_error(ident: &impl std::fmt::Display) -> String {
    format!("failed to load GDExtension function `{ident}`")
}

/// Returns true if the type is so trivial that most of its operations are directly provided by Rust, and there is no need
/// to expose the construct/destruct/operator methods from Godot
fn is_trivial(type_names: &TypeNames) -> bool {
    let list = ["bool", "int", "float"];

    list.contains(&type_names.json_builtin_name.as_str())
}

fn shout_to_pascal(shout_case: &str) -> String {
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
