use crate::godot_exe;
use std::collections::HashMap;

use miniserde::{json, Deserialize};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::path::Path;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(Deserialize)]
struct ExtensionApi {
    builtin_class_sizes: Vec<ClassSizes>,
    builtin_classes: Vec<BuiltinClass>,
    classes: Vec<Class>,
    global_enums: Vec<Enum>,
}

#[derive(Deserialize)]
struct ClassSizes {
    build_configuration: String,
    sizes: Vec<ClassSize>,
}

#[derive(Deserialize)]
struct ClassSize {
    name: String,
    size: usize,
}

#[derive(Deserialize)]
struct BuiltinClass {
    name: String,
    constructors: Vec<Constructor>,
    has_destructor: bool,
}

#[derive(Deserialize)]
struct Class {
    name: String,
    is_refcounted: bool,
    is_instantiable: bool,
    inherits: String,
    api_type: String,
    constants: Vec<Constant>,
    enums: Vec<Enum>,
    methods: Vec<Method>
}

#[derive(Deserialize)]
struct Constructor {
    index: usize,
    arguments: Option<Vec<MethodArg>>,
}

#[derive(Deserialize)]
struct MethodArg {
    name: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize)]
struct MethodReturn {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Deserialize)]
struct Enum {
    name: String,
    values: Vec<Constant>,
}

#[derive(Deserialize)]
struct Constant {
    name: String,
    value: i32,
}

#[derive(Deserialize)]
struct Method {
    name: String,
    is_const: bool,
    is_vararg: bool,
    is_static: bool,
    is_virtual: bool,
    hash: u64,
    arguments: Vec<MethodArg>,
    return_value: Option<MethodReturn>
}



// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

struct Tokens {
    opaque_types: Vec<TokenStream>,
    variant_enumerators: Vec<TokenStream>,
    variant_fn_decls: Vec<TokenStream>,
    variant_fn_inits: Vec<TokenStream>,
}

struct TypeNames {
    /// "PackedVector2Array"
    pascal_case: String,

    /// "packed_vector2_array"
    snake_case: String,

    /// "PACKED_VECTOR2_ARRAY"
    shout_case: String,

    /// GDNativeVariantType_GDNATIVE_VARIANT_TYPE_PACKED_VECTOR2_ARRAY
    sys_variant_type: Ident,
}

pub struct ApiParser {}

impl ApiParser {
    pub fn generate_file(gen_path: &Path) {
        let tokens = Self::load_extension_api();
        let Tokens {
            opaque_types,
            variant_enumerators,
            variant_fn_decls,
            variant_fn_inits,
        } = tokens;

        let tokens = quote! {
            #![allow(dead_code)]
            use crate::{GDNativeVariantPtr, GDNativeTypePtr};

            pub mod types {
                #(#opaque_types)*
            }

            pub struct InterfaceCache {
                #(#variant_fn_decls)*
            }

            impl InterfaceCache {
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
        let out_path = gen_path.join("extensions.rs");
        std::fs::write(&out_path, string).expect("failed to write extension file");
        Self::format_file_if_needed(&out_path);
    }

    fn load_extension_api() -> Tokens {
        let build_config = "float_64"; // TODO infer this

        let json: String = godot_exe::load_extension_api_json();
        let model: ExtensionApi = json::from_str(&json).expect("failed to deserialize JSON");

        let mut opaque_types = vec![];
        let mut variant_enumerators = vec![];
        let mut variant_fn_decls = vec![];
        let mut variant_fn_inits = vec![];

        for class in &model.builtin_class_sizes {
            if &class.build_configuration == build_config {
                for ClassSize { name, size } in &class.sizes {
                    opaque_types.push(Self::quote_opaque_type(name, *size));
                }

                break;
            }
        }

        // Find variant types, for which `variant_get_ptr_destructor` returns a non-null function pointer.
        // List is directly sourced from extension_api.json (information would also be in variant_destruct.cpp).
        let mut class_map = HashMap::new();
        for class in &model.builtin_classes {
            let normalized_name = class.name.to_lowercase();

            class_map.insert(normalized_name, class);
        }

        let class_map = class_map;

        for enum_ in &model.global_enums {
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

                    let pascal_case: String;
                    let has_destructor: bool;
                    let constructors: Option<&Vec<Constructor>>;
                    if let Some(class) = class_map.get(&normalized) {
                        pascal_case = class.name.clone();
                        has_destructor = class.has_destructor;
                        constructors = Some(&class.constructors);
                    } else {
                        assert_eq!(normalized, "object");
                        pascal_case = "Object".to_string();
                        has_destructor = false;
                        constructors = None;
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
                    variant_enumerators.push(Self::quote_enumerator(&type_names, value));

                    let (decl, init) =
                        Self::quote_variant_fns(&type_names, has_destructor, constructors);

                    variant_fn_decls.push(decl);
                    variant_fn_inits.push(init);
                }

                break;
            }
        }

        Tokens {
            opaque_types,
            variant_enumerators,
            variant_fn_decls,
            variant_fn_inits,
        }
    }

    fn quote_enumerator(type_names: &TypeNames, value: i32) -> TokenStream {
        let enumerator = format_ident!("{}", type_names.shout_case);
        let value = proc_macro2::Literal::i32_unsuffixed(value);

        quote! {
           #enumerator = #value
        }
    }

    fn quote_opaque_type(name: &str, size: usize) -> TokenStream {
        // Capitalize: "int" -> "Int"
        let (first, rest) = name.split_at(1);
        let ident = format_ident!("Opaque{}{}", first.to_uppercase(), rest);
        //let upper = format_ident!("SIZE_{}", name.to_uppercase());
        quote! {
            pub type #ident = crate::opaque::Opaque<#size>;
            //pub const #upper: usize = #size;
        }
    }

    fn quote_variant_fns(
        type_names: &TypeNames,
        has_destructor: bool,
        constructors: Option<&Vec<Constructor>>,
    ) -> (TokenStream, TokenStream) {
        let (destroy_decls, destroy_inits) = Self::quote_destroy_fns(&type_names, has_destructor);

        let (construct_decls, construct_inits) =
            Self::quote_construct_fns(&type_names, constructors);

        let to_variant = format_ident!("{}_to_variant", type_names.snake_case);
        let from_variant = format_ident!("{}_from_variant", type_names.snake_case);
        let variant_type = &type_names.sys_variant_type;

        // Field declaration
        let decl = quote! {
            pub #to_variant: unsafe extern "C" fn(GDNativeVariantPtr, GDNativeTypePtr),
            pub #from_variant: unsafe extern "C" fn(GDNativeTypePtr, GDNativeVariantPtr),
            #construct_decls
            #destroy_decls
        };

        // Field initialization in new()
        let init = quote! {
            #to_variant: {
                let ctor_fn = interface.get_variant_from_type_constructor.unwrap();
                ctor_fn(crate:: #variant_type).unwrap()
            },
            #from_variant: {
                let ctor_fn = interface.get_variant_to_type_constructor.unwrap();
                ctor_fn(crate:: #variant_type).unwrap()
            },
            #construct_inits
            #destroy_inits
        };

        (decl, init)
    }

    fn quote_construct_fns(
        type_names: &TypeNames,
        constructors: Option<&Vec<Constructor>>,
    ) -> (TokenStream, TokenStream) {
        let constructors = match constructors {
            Some(c) => c,
            None => return (TokenStream::new(), TokenStream::new()),
        };

        // Constructor vec layout:
        //   [0]: default constructor
        //   [1]: copy constructor
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
        let variant_type = &type_names.sys_variant_type;

        // Generic signature:  fn(base: GDNativeTypePtr, args: *const GDNativeTypePtr)
        let decls = quote! {
            pub #construct_default: unsafe extern "C" fn(GDNativeTypePtr, *const GDNativeTypePtr),
            pub #construct_copy: unsafe extern "C" fn(GDNativeTypePtr, *const GDNativeTypePtr),
        };

        let inits = quote! {
            #construct_default: {
                let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
                ctor_fn(crate:: #variant_type, 0i32).unwrap()
            },
            #construct_copy: {
                let ctor_fn = interface.variant_get_ptr_constructor.unwrap();
                ctor_fn(crate:: #variant_type, 1i32).unwrap()
            },
        };

        (decls, inits)
    }

    fn quote_destroy_fns(
        type_names: &TypeNames,
        has_destructor: bool,
    ) -> (TokenStream, TokenStream) {
        if !has_destructor {
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

    //#[cfg(feature = "formatted")]
    fn format_file_if_needed(output_rs: &Path) {
        print!(
            "Formatting generated file: {}... ",
            output_rs
                .canonicalize()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap()
        );

        let output = std::process::Command::new("rustup")
            .arg("run")
            .arg("stable")
            .arg("rustfmt")
            .arg("--edition=2021")
            .arg(output_rs)
            .output();

        match output {
            Ok(_) => println!("Done."),
            Err(err) => {
                println!("Failed.");
                println!("Error: {}", err);
            }
        }
    }
}
