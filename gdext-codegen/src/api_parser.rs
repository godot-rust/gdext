use crate::godot_exe;
use std::collections::HashSet;

use miniserde::{json, Deserialize};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::path::Path;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(Deserialize)]
struct ExtensionApi {
    builtin_class_sizes: Vec<ClassSizes>,
    builtin_classes: Vec<BuiltinClass>,
    global_enums: Vec<GlobalEnum>,
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
    has_destructor: bool,
}

#[derive(Deserialize)]
struct GlobalEnum {
    name: String,
    values: Vec<EnumValue>,
}

#[derive(Deserialize)]
struct EnumValue {
    name: String,
    value: i32,
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Implementation

struct Tokens {
    opaque_types: Vec<TokenStream>,
    variant_enumerators: Vec<TokenStream>,
    variant_fn_decls: Vec<TokenStream>,
    variant_fn_inits: Vec<TokenStream>,
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
        let mut has_destructor_set = HashSet::new();
        for class in &model.builtin_classes {
            if class.has_destructor {
                has_destructor_set.insert(class.name.to_lowercase()); // normalized
            }
        }
        let has_destructor_set = has_destructor_set;

        for enum_ in &model.global_enums {
            if &enum_.name == "Variant.Type" {
                for ty in &enum_.values {
                    let type_name = ty
                        .name
                        .strip_prefix("TYPE_")
                        .expect("Enum name begins with 'TYPE_'");

                    if type_name == "NIL" || type_name == "MAX" {
                        continue;
                    }

                    // Lowercase without underscore, to map SHOUTY_CASE to shoutycase
                    let normalized = type_name.to_lowercase().replace("_", "");
                    let has_destructor = has_destructor_set.contains(&normalized);

                    let value = ty.value;
                    variant_enumerators.push(Self::quote_enumerator(type_name, value));

                    let (decl, init) = Self::quote_variant_convs(type_name, has_destructor);
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

    fn quote_enumerator(name: &str, value: i32) -> TokenStream {
        let enumerator = format_ident!("{}", name);
        let value = proc_macro2::Literal::i32_unsuffixed(value);

        quote! {
           #enumerator = #value
        }
    }

    fn quote_opaque_type(name: &str, size: usize) -> TokenStream {
        // Capitalize: "int" -> "Int"
        let (first, rest) = name.split_at(1);
        let ident = format_ident!("Opaque{}{}", first.to_uppercase(), rest);
        quote! {
            pub type #ident = crate::opaque::Opaque<#size>;
        }
    }

    fn quote_variant_convs(upper_name: &str, has_destructor: bool) -> (TokenStream, TokenStream) {
        let lowercase = upper_name.to_lowercase();

        let from_name = format_ident!("variant_from_{}", lowercase);
        let to_name = format_ident!("variant_to_{}", lowercase);

        let variant_type =
            format_ident!("GDNativeVariantType_GDNATIVE_VARIANT_TYPE_{}", upper_name);

        let destroy_decl_tokens: TokenStream;
        let destroy_init_tokens: TokenStream;

        if has_destructor {
            let destroy = format_ident!("destroy_{}", lowercase);

            destroy_decl_tokens = quote! {
                pub #destroy: unsafe extern "C" fn(GDNativeTypePtr),
            };

            destroy_init_tokens = quote! {
                #destroy: {
                    let dtor_fn = interface.variant_get_ptr_destructor.unwrap();
                    dtor_fn(crate:: #variant_type).unwrap()
                },
            };
        } else {
            destroy_decl_tokens = TokenStream::new();
            destroy_init_tokens = TokenStream::new();
        }

        // Field declaration
        let decl = quote! {
            pub #from_name: unsafe extern "C" fn(GDNativeVariantPtr, GDNativeTypePtr),
            pub #to_name: unsafe extern "C" fn(GDNativeTypePtr, GDNativeVariantPtr),
            #destroy_decl_tokens
        };

        // Field initialization in new()
        let init = quote! {
            #from_name: {
                let ctor_fn = interface.get_variant_from_type_constructor.unwrap();
                ctor_fn(crate:: #variant_type).unwrap()
            },
            #to_name: {
                let ctor_fn = interface.get_variant_to_type_constructor.unwrap();
                ctor_fn(crate:: #variant_type).unwrap()
            },
            #destroy_init_tokens
        };

        (decl, init)
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
