use crate::godot_exe;

use miniserde::{json, Deserialize};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::path::Path;

// ----------------------------------------------------------------------------------------------------------------------------------------------
// JSON models

#[derive(Deserialize)]
struct ExtensionApi {
    builtin_class_sizes: Vec<ClassSizes>,
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
    variant_conv_decls: Vec<TokenStream>,
    variant_conv_inits: Vec<TokenStream>,
}

pub struct ApiParser {}

impl ApiParser {
    pub fn generate_file(gen_path: &Path) {
        let tokens = Self::load_extension_api();
        let Tokens {
            opaque_types,
            variant_enumerators,
            variant_conv_decls,
            variant_conv_inits,
        } = tokens;

        let tokens = quote! {
            #![allow(dead_code)]
            use crate::{GDNativeVariantPtr, GDNativeTypePtr};

            pub mod types {
                #(#opaque_types)*
            }

            pub struct InterfaceCache {
                #(#variant_conv_decls)*
            }

            impl InterfaceCache {
                pub(crate) unsafe fn new(interface: &crate::GDNativeInterface) -> Self {
                    Self {
                        #(#variant_conv_inits)*
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
        let mut variant_conv_decls = vec![];
        let mut variant_conv_inits = vec![];

        for class in &model.builtin_class_sizes {
            if &class.build_configuration == build_config {
                for ClassSize { name, size } in &class.sizes {
                    opaque_types.push(Self::quote_opaque_type(name, *size));
                }

                break;
            }
        }

        for enum_ in &model.global_enums {
            if &enum_.name == "Variant.Type" {
                for ty in &enum_.values {
                    let name = ty
                        .name
                        .strip_prefix("TYPE_")
                        .expect("Enum name begins with 'TYPE_'");

                    if name == "NIL" || name == "MAX" {
                        continue;
                    }

                    let value = ty.value;
                    variant_enumerators.push(Self::quote_enumerator(name, value));

                    let (decl, init) = Self::quote_variant_convs(name);
                    variant_conv_decls.push(decl);
                    variant_conv_inits.push(init);
                }

                break;
            }
        }

        Tokens {
            opaque_types,
            variant_enumerators,
            variant_conv_decls,
            variant_conv_inits,
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

    fn quote_variant_convs(upper_name: &str) -> (TokenStream, TokenStream) {
        let from_name = format_ident!("variant_from_{}", upper_name.to_lowercase());
        let to_name = format_ident!("variant_to_{}", upper_name.to_lowercase());

        let variant_type =
            format_ident!("GDNativeVariantType_GDNATIVE_VARIANT_TYPE_{}", upper_name);

        // Field declaration
        let decl = quote! {
            pub #from_name: unsafe extern "C" fn(GDNativeVariantPtr, GDNativeTypePtr),
            pub #to_name: unsafe extern "C" fn(GDNativeTypePtr, GDNativeVariantPtr),
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
