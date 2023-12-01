/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use venial::{Declaration, StructFields};

use crate::util::{decl_get_info, has_attr, DeclInfo};
use crate::ParseResult;

pub fn derive_to_godot(decl: Declaration) -> ParseResult<TokenStream> {
    let mut body = quote! {
        let mut root = ::godot::builtin::Dictionary::new();
    };

    let DeclInfo {
        where_,
        generic_params,
        name,
        name_string,
    } = decl_get_info(&decl);

    match &decl {
        Declaration::Struct(struct_) => match &struct_.fields {
            StructFields::Unit => make_struct_unit(&mut body, name_string),
            StructFields::Tuple(fields) => make_struct_tuple(&mut body, fields, name_string),
            StructFields::Named(named_struct) => {
                make_struct_named(&mut body, named_struct, name_string);
            }
        },
        Declaration::Enum(enum_) => {
            let arms = enum_.variants.iter().map(|(enum_v, _)| {
                let variant_name = enum_v.name.clone();
                let variant_name_string = enum_v.name.to_string();
                let fields = match &enum_v.contents {
                    StructFields::Unit => quote! {},
                    StructFields::Tuple(s) => make_tuple_enum_field(s),
                    StructFields::Named(named) => make_named_enum_field(named),
                };
                let arm_content = match &enum_v.contents {
                    _ if has_attr(&enum_v.attributes, "variant", "skip") => quote! {
                        return ::godot::builtin::dict! {
                            #name_string: ::godot::builtin::Variant::nil()
                        }.to_variant();
                    },
                    StructFields::Unit => quote! { #variant_name_string.to_variant() },

                    StructFields::Tuple(fields) => make_enum_tuple_arm(fields, variant_name_string),
                    StructFields::Named(fields) => make_enum_named_arm(fields, variant_name_string),
                };
                quote! {
                    Self::#variant_name #fields => {
                        #arm_content
                    }
                }
            });

            body = quote! {
                #body
                let content = match core::clone::Clone::clone(self) {
                    #(
                        #arms
                    )*
                };
                root.insert(#name_string, content);
            };
        }

        // decl_get_info() above ensured that no other cases are possible.
        _ => unreachable!(),
    };

    body = quote! {
        #body
        root.to_variant()
    };

    let gen = generic_params.as_ref().map(|x| x.as_inline_args());

    // We need to allow unreachable code for uninhabited enums, because it uses match self {}.
    // This is safe, since we can't ever have a value to call to_variant on it anyway.
    let allow_unreachable = if matches!(&decl, Declaration::Enum(e) if e.variants.is_empty()) {
        quote! {
            #[allow(unreachable_code)]
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        impl #generic_params ::godot::builtin::meta::ToGodot for #name #gen #where_ {
            #allow_unreachable
             fn to_godot(&self) -> ::godot::builtin::Variant {
                #body
            }
        }
    })
}

fn make_named_enum_field(named: &venial::NamedStructFields) -> TokenStream {
    let fields = named.fields.iter().map(|(field, _)| {
        let field_name = field.name.to_token_stream();
        if has_attr(&field.attributes, "variant", "skip") {
            quote! { #field_name : _ }
        } else {
            field_name
        }
    });
    quote! { { #(#fields ,)* } }
}

fn make_tuple_enum_field(s: &venial::TupleStructFields) -> TokenStream {
    let fields = s.fields.iter().enumerate().map(|(k, (f, _))| {
        if has_attr(&f.attributes, "variant", "skip") {
            quote! { _ }
        } else {
            format_ident!("__{}", k).to_token_stream()
        }
    });

    quote! {
        (#(#fields ,)*)
    }
}

fn make_enum_named_arm(
    fields: &venial::NamedStructFields,
    variant_name_string: String,
) -> TokenStream {
    let fields = fields
        .fields
        .iter()
        .filter(|(f, _)| !has_attr(&f.attributes, "variant", "skip"))
        .map(|(field, _)| (field.name.clone(), field.name.to_string()))
        .map(|(ident, ident_string)| {
            quote! {
                root.insert(#ident_string,#ident.to_variant());
            }
        });

    quote! {
        let mut root = ::godot::builtin::Dictionary::new();
        #(
            #fields
        )*
        ::godot::builtin::dict! { #variant_name_string: root }.to_variant()
    }
}

fn make_enum_tuple_arm(
    fields: &venial::TupleStructFields,
    variant_name_string: String,
) -> TokenStream {
    if fields.fields.len() == 1 {
        let res = if has_attr(
            &fields.fields.first().unwrap().0.attributes,
            "variant",
            "skip",
        ) {
            quote! { godot::builtin::Variant::nil() }
        } else {
            quote! { __0 }
        };
        return quote! { godot::builtin::dict! { #variant_name_string : #res }.to_variant() };
    }
    let fields = fields
        .fields
        .iter()
        .enumerate()
        .filter(|(_, (f, _))| !has_attr(&f.attributes, "variant", "skip"))
        .map(|(k, _)| format_ident!("__{}", k))
        .map(|ident| {
            quote! {
                root.push(#ident.to_variant());
            }
        });
    quote! {
        let mut root = godot::builtin::VariantArray::new();
        #(
            #fields

        )*
        godot::builtin::dict!{ #variant_name_string: root }.to_variant()
    }
}

fn make_struct_named(
    body: &mut TokenStream,
    fields: &venial::NamedStructFields,
    string_ident: String,
) {
    let fields = fields
        .fields
        .items()
        .filter(|f| !has_attr(&f.attributes, "variant", "skip"))
        .map(|nf| {
            let field_name = nf.name.clone();
            let field_name_string = nf.name.to_string();

            quote! {
                fields.insert(#field_name_string, self.#field_name.to_variant());
            }
        });

    *body = quote! {
        #body
        let mut fields = godot::builtin::Dictionary::new();
        #(
            #fields
        )*
        root.insert(#string_ident, fields.to_variant());
    };
}

fn make_struct_tuple(
    body: &mut TokenStream,
    fields: &venial::TupleStructFields,
    string_ident: String,
) {
    if fields.fields.len() == 1
        && !has_attr(
            &fields.fields.first().unwrap().0.attributes,
            "variant",
            "skip",
        )
    {
        *body = quote! {
            #body
            root.insert(#string_ident, self.0.to_variant());
        };

        return;
    }
    let fields = fields
        .fields
        .iter()
        .filter(|(f, _)| !has_attr(&f.attributes, "variant", "skip"))
        .enumerate()
        .map(|(k, _)| proc_macro2::Literal::usize_unsuffixed(k))
        .map(|ident| {
            quote! {
                fields.push(self.#ident.to_variant());
            }
        });

    *body = quote! {
        #body
        let mut fields = godot::builtin::VariantArray::new();
        #(
            #fields
        )*
        root.insert(#string_ident, fields.to_variant());
    };
}

fn make_struct_unit(body: &mut TokenStream, string_ident: String) {
    *body = quote! {
        #body
        root.insert(#string_ident, godot::builtin::Variant::nil());
    }
}
