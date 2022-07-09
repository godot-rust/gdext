use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::api_parser::*;
use crate::util::ident;
use crate::Context;

struct CentralItems {
    opaque_types: Vec<TokenStream>,
    variant_enumerators: Vec<TokenStream>,
    variant_fn_decls: Vec<TokenStream>,
    variant_fn_inits: Vec<TokenStream>,
}

struct TypeNames {
    /// "int" or "PackedVector2Array"
    pascal_case: String,

    /// "packed_vector2_array"
    snake_case: String,

    /// "PACKED_VECTOR2_ARRAY"
    shout_case: String,

    /// GDNativeVariantType_GDNATIVE_VARIANT_TYPE_PACKED_VECTOR2_ARRAY
    sys_variant_type: Ident,
}

pub(crate) fn generate_central_file(
    api: &ExtensionApi,
    _ctx: &Context,
    build_config: &str,
    gen_path: &Path,
    out_files: &mut Vec<PathBuf>,
) {
    let CentralItems {
        opaque_types,
        variant_enumerators,
        variant_fn_decls,
        variant_fn_inits,
    } = make_central_items(api, build_config);

    let tokens = quote! {
        #![allow(dead_code)]
        use crate::{GDNativeVariantPtr, GDNativeTypePtr};

        pub mod types {
            #(#opaque_types)*
        }

        pub struct GlobalMethodTable {
            #(#variant_fn_decls)*
        }

        impl GlobalMethodTable {
            pub(crate) unsafe fn new(interface: &crate::GDNativeInterface) -> Self {
                Self {
                    #(#variant_fn_inits)*
                }
            }
        }

        pub enum VariantType {
            #(#variant_enumerators),*
        }
    };

    let string = tokens.to_string();

    let _ = std::fs::create_dir(gen_path);
    let out_path = gen_path.join("central.rs");
    std::fs::write(&out_path, string).expect("failed to write central extension file");

    out_files.push(out_path);
}

fn make_central_items(api: &ExtensionApi, build_config: &str) -> CentralItems {
    let mut opaque_types = vec![];
    for class in &api.builtin_class_sizes {
        if &class.build_configuration == build_config {
            for ClassSize { name, size } in &class.sizes {
                opaque_types.push(make_opaque_type(name, *size));
            }

            break;
        }
    }

    // Find variant types, for which `variant_get_ptr_destructor` returns a non-null function pointer.
    // List is directly sourced from extension_api.json (information would also be in variant_destruct.cpp).
    let mut class_map = HashMap::new();
    for class in &api.builtin_classes {
        let normalized_name = class.name.to_lowercase();

        class_map.insert(normalized_name, class);
    }

    let class_map = class_map;

    // Find enum with variant types, and generate mapping code
    let mut variant_enumerators = vec![];
    let mut variant_fn_decls = vec![];
    let mut variant_fn_inits = vec![];
    for enum_ in &api.global_enums {
        if &enum_.name == "Variant.Type" {
            for ty in &enum_.values {
                let shout_case = ty
                    .name
                    .strip_prefix("TYPE_")
                    .expect("Enum name begins with 'TYPE_'");

                if shout_case == "NIL" || shout_case == "MAX" {
                    continue;
                }

                // Lowercase without underscore, to map SHOUTY_CASE to shoutycase
                let normalized = shout_case.to_lowercase().replace("_", "");

                // TODO cut down on the number of cached functions generated
                // e.g. there's no point in providing operator< for int
                let pascal_case: String;
                let has_destructor: bool;
                let constructors: Option<&Vec<Constructor>>;
                let operators: Option<&Vec<Operator>>;
                if let Some(class) = class_map.get(&normalized) {
                    pascal_case = class.name.clone();
                    has_destructor = class.has_destructor;
                    constructors = Some(&class.constructors);
                    operators = Some(&class.operators);
                } else {
                    assert_eq!(normalized, "object");
                    pascal_case = "Object".to_string();
                    has_destructor = false;
                    constructors = None;
                    operators = None;
                }

                let type_names = TypeNames {
                    pascal_case,
                    snake_case: shout_case.to_lowercase(),
                    shout_case: shout_case.to_string(),
                    sys_variant_type: format_ident!(
                        "GDNativeVariantType_GDNATIVE_VARIANT_TYPE_{}",
                        shout_case
                    ),
                };

                let value = ty.value;
                variant_enumerators.push(make_enumerator(&type_names, value));

                let (decl, init) =
                    make_variant_fns(&type_names, has_destructor, constructors, operators);

                variant_fn_decls.push(decl);
                variant_fn_inits.push(init);
            }

            break;
        }
    }

    CentralItems {
        opaque_types,
        variant_enumerators,
        variant_fn_decls,
        variant_fn_inits,
    }
}

fn make_enumerator(type_names: &TypeNames, value: i32) -> TokenStream {
    let enumerator = format_ident!("{}", type_names.shout_case);
    let value = proc_macro2::Literal::i32_unsuffixed(value);

    quote! {
       #enumerator = #value
    }
}

fn make_opaque_type(name: &str, size: usize) -> TokenStream {
    // Capitalize: "int" -> "Int"
    let (first, rest) = name.split_at(1);
    let ident = format_ident!("Opaque{}{}", first.to_uppercase(), rest);
    //let upper = format_ident!("SIZE_{}", name.to_uppercase());
    quote! {
        pub type #ident = crate::opaque::Opaque<#size>;
        //pub const #upper: usize = #size;
    }
}

fn make_variant_fns(
    type_names: &TypeNames,
    has_destructor: bool,
    constructors: Option<&Vec<Constructor>>,
    operators: Option<&Vec<Operator>>,
) -> (TokenStream, TokenStream) {
    let (construct_decls, construct_inits) = make_construct_fns(&type_names, constructors);
    let (destroy_decls, destroy_inits) = make_destroy_fns(&type_names, has_destructor);
    let (op_eq_decls, op_eq_inits) = make_operator_fns(&type_names, operators, "==", "EQUAL");
    let (op_lt_decls, op_lt_inits) = make_operator_fns(&type_names, operators, "<", "LESS");

    let to_variant = format_ident!("{}_to_variant", type_names.snake_case);
    let from_variant = format_ident!("{}_from_variant", type_names.snake_case);

    let to_variant_error = format_load_error(&to_variant);
    let from_variant_error = format_load_error(&from_variant);

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate:: #variant_type };

    // Field declaration
    let decl = quote! {
        pub #to_variant: unsafe extern "C" fn(GDNativeVariantPtr, GDNativeTypePtr),
        pub #from_variant: unsafe extern "C" fn(GDNativeTypePtr, GDNativeVariantPtr),
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
    //  rest: not interesting for now

    // Sanity checks -- ensure format is as expected
    for (i, c) in constructors.iter().enumerate() {
        assert_eq!(i, c.index);
    }

    assert!(constructors[0].arguments.is_none());

    if let Some(args) = &constructors[1].arguments {
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "from");
        assert_eq!(args[0].type_, type_names.pascal_case);
    } else {
        panic!(
            "type {}: no constructor args found for copy constructor",
            type_names.pascal_case
        );
    }

    let construct_default = format_ident!("{}_construct_default", type_names.snake_case);
    let construct_copy = format_ident!("{}_construct_copy", type_names.snake_case);
    let construct_default_error = format_load_error(&construct_default);
    let construct_copy_error = format_load_error(&construct_copy);
    let variant_type = &type_names.sys_variant_type;

    let (construct_extra, construct_extra_index, construct_extra_error) =
        make_extra_constructors(type_names);

    // Generic signature:  fn(base: GDNativeTypePtr, args: *const GDNativeTypePtr)
    let decls = quote! {
        pub #construct_default: unsafe extern "C" fn(GDNativeTypePtr, *const GDNativeTypePtr),
        pub #construct_copy: unsafe extern "C" fn(GDNativeTypePtr, *const GDNativeTypePtr),
        #(
            pub #construct_extra: unsafe extern "C" fn(GDNativeTypePtr, *const GDNativeTypePtr),
        )*
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
        #(
            #construct_extra: {
                let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
                ctor_fn(crate:: #variant_type, #construct_extra_index).expect(#construct_extra_error)
            },
        )*
    };

    (decls, inits)
}

/// Lists special cases for useful constructors
fn make_extra_constructors(type_names: &TypeNames) -> (Vec<Ident>, Vec<i32>, Vec<String>) {
    // TODO instead of finding ctors by index, more robust would be to look up from JSON
    let one_vec = match type_names.pascal_case.as_str() {
        "NodePath" => {
            vec![("node_path_from_string", 2)]
        }
        "StringName" => {
            vec![("string_name_from_string", 2)]
        }
        "String" => {
            vec![
                ("string_from_string_name", 2), //
                ("string_from_node_path", 3),
            ]
        }
        _ => vec![],
    };

    // unzip() for 3
    (
        one_vec.iter().map(|(name, _)| ident(name)).collect(),
        one_vec.iter().map(|(_, index)| *index).collect(),
        one_vec
            .iter()
            .map(|(name, _)| format_load_error(name))
            .collect(),
    )
}

fn make_destroy_fns(type_names: &TypeNames, has_destructor: bool) -> (TokenStream, TokenStream) {
    if !has_destructor || is_trivial(type_names) {
        return (TokenStream::new(), TokenStream::new());
    }

    let destroy = format_ident!("{}_destroy", type_names.snake_case);
    let variant_type = &type_names.sys_variant_type;

    let decls = quote! {
        pub #destroy: unsafe extern "C" fn(GDNativeTypePtr),
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
        || !operators.unwrap().iter().any(|op| &op.name == json_name)
        || is_trivial(type_names)
    {
        return (TokenStream::new(), TokenStream::new());
    }

    let operator = format_ident!(
        "{}_operator_{}",
        type_names.snake_case,
        sys_name.to_lowercase()
    );
    let error = format_load_error(&operator);

    let variant_type = &type_names.sys_variant_type;
    let variant_type = quote! { crate:: #variant_type };
    let sys_ident = format_ident!("GDNativeVariantOperator_GDNATIVE_VARIANT_OP_{}", sys_name);

    // Field declaration
    let decl = quote! {
        pub #operator: unsafe extern "C" fn(GDNativeTypePtr, GDNativeTypePtr, GDNativeTypePtr),
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
    format!(
        "failed to load GDExtension function `{}`",
        ident.to_string()
    )
}

/// Returns true if the type is so trivial that most of its operations are directly provided by Rust, and there is no need
/// to expose the construct/destruct/operator methods from Godot
fn is_trivial(type_names: &TypeNames) -> bool {
    let list = ["bool", "int", "float"];

    list.contains(&type_names.pascal_case.as_str())
}
