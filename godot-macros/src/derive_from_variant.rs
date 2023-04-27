/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::{decl_get_info, DeclInfo};
use crate::ParseResult;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use venial::{Declaration, StructFields};

pub fn transform(decl: Declaration) -> ParseResult<TokenStream> {
    let DeclInfo {
        where_,
        generic_params,
        name,
        name_string,
    } = decl_get_info(&decl);
    let mut body = quote! {
        let root = variant.try_to::<godot::builtin::Dictionary>()?;
        let root = root.get(#name_string).ok_or(godot::builtin::VariantConversionError::BadType)?;
    };

    match decl {
        Declaration::Struct(s) => match s.fields {
            venial::StructFields::Unit => {
                body = quote! {
                    #body
                    return Ok(Self);
                }
            }
            venial::StructFields::Tuple(fields) => {
                if fields.fields.len() == 1 {
                    body = quote! {
                        #body
                        let root = root.try_to()?;
                        Ok(Self(root))
                    };
                } else {
                    let ident_and_set = fields.fields.iter().enumerate().map(|(k, _)| {
                        let ident = format_ident!("__{}", k);
                        (
                            ident.clone(),
                            quote! {
                                let #ident = root.pop_front().ok_or(godot::builtin::VariantConversionError::MissingValue)?;
                            },

                        )
                    });
                    let (idents, ident_set): (Vec<_>, Vec<_>) = ident_and_set.unzip();
                    body = quote! {
                        #body
                        let mut root = root.try_to::<godot::builtin::Array<godot::builtin::Variant>>()?;
                        #(
                            #ident_set

                        )*
                        Ok(Self(
                            #(#idents.try_to()?,)*
                        ))
                    };
                }
            }
            venial::StructFields::Named(fields) => {
                let fields = fields.fields.iter().map(|(field, _)|{
                    let ident = &field.name;
                    let string_ident = &field.name.to_string();
                    (
                        quote!{
                            let #ident = root.get(#string_ident).ok_or(godot::builtin::VariantConversionError::MissingValue)?;
                        },

                        quote!{
                            #ident :#ident.try_to()?
                        }
                    )

                });
                let (set_idents, set_self): (Vec<_>, Vec<_>) = fields.unzip();
                body = quote! {
                    #body
                    let root = root.try_to::<godot::builtin::Dictionary>()?;
                    #(
                        #set_idents
                    )*
                    Ok(Self{ #(#set_self,)* })
                }
            }
        },
        Declaration::Enum(enum_) => {
            if enum_.variants.is_empty() {
                body = quote! {
                    panic!();
                }
            } else {
                let mut matches = quote! {};
                for (enum_v, _) in &enum_.variants.inner {
                    let variant_name = enum_v.name.clone();
                    let variant_name_string = enum_v.name.to_string();
                    let if_let_content = match &enum_v.contents {
                        StructFields::Unit => quote! {
                                let child = root.try_to::<String>();
                                if child == Ok(String::from(#variant_name_string)) {
                                    return Ok(Self::#variant_name);
                                }
                        },
                        StructFields::Tuple(fields) => {
                            if fields.fields.len() == 1 {
                                let (field, _) = fields.fields.first().unwrap();
                                let field_type = &field.ty;
                                quote! {
                                    let child = root.try_to::<godot::builtin::Dictionary>();
                                    if let Ok(child) = child {
                                        if let Some(variant) = child.get(#variant_name_string) {
                                            return Ok(Self::#variant_name(variant.try_to::<#field_type>()?));
                                        }
                                    }
                                }
                            } else {
                                let fields = fields.fields.iter().enumerate()
                                .map(|(k, (field, _))|{
                                    let ident = format_ident!("__{k}");
                                    let field_type = &field.ty;
                                    (
                                        quote!{#ident},

                                        quote!{
                                            let #ident = variant
                                                            .pop_front()
                                                            .ok_or(godot::builtin::VariantConversionError::MissingValue)?
                                                            .try_to::<#field_type>()?;
                                    })
                                });
                                let (idents, set_idents): (Vec<_>, Vec<_>) = fields.unzip();

                                quote! {
                                    let child = root.try_to::<godot::builtin::Dictionary>();
                                    if let Ok(child) = child {
                                        if let Some(variant) = child.get(#variant_name_string) {
                                            let mut variant = variant.try_to::<godot::builtin::Array<godot::builtin::Variant>>()?;
                                            #(#set_idents)*
                                            return Ok(Self::#variant_name(#(#idents ,)*));
                                        }
                                    }
                                }
                            }
                        }
                        StructFields::Named(fields) => {
                            let fields = fields.fields.iter().map(|(field, _)| {
                                let field_name = &field.name;
                                let field_name_string = &field.name.to_string();
                                let field_type = &field.ty;
                                (
                                    quote!{#field_name},
                                    quote!{
                                        let #field_name = variant.get(#field_name_string).ok_or(godot::builtin::VariantConversionError::MissingValue)?.try_to::<#field_type>()?;
                                    }
                                )
                            });
                            let (fields, set_fields): (Vec<_>, Vec<_>) = fields.unzip();
                            quote! {
                                if let Ok(root) = root.try_to::<godot::builtin::Dictionary>() {
                                    if let Some(variant) = root.get(#variant_name_string) {
                                        let variant = variant.try_to::<godot::builtin::Dictionary>()?;
                                        #(
                                            #set_fields
                                        )*
                                        return Ok(Self::#variant_name{ #(#fields,)* });
                                    }
                                }
                            }
                        }
                    };
                    matches = quote! {
                        #matches
                        #if_let_content
                    };
                }
                body = quote! {
                    #body
                    #matches
                    Err(godot::builtin::VariantConversionError::MissingValue)
                };
            }
        }
        _ => unreachable!(),
    }

    let gen = generic_params.as_ref().map(|x| x.as_inline_args());
    Ok(quote! {
        impl #generic_params godot::builtin::FromVariant for #name #gen #where_ {
            fn try_from_variant(
                variant: &godot::builtin::Variant
            ) -> Result<Self, godot::builtin::VariantConversionError> {
                #body
            }
        }
    })
}
