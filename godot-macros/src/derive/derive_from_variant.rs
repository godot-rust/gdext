/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use venial::{Declaration, NamedStructFields, StructFields, TupleField, TupleStructFields};

use crate::util::{decl_get_info, has_attr, DeclInfo};
use crate::ParseResult;

fn has_attr_skip(attributes: &[venial::Attribute]) -> bool {
    has_attr(attributes, "variant", "skip")
}

pub fn derive_from_godot(decl: Declaration) -> ParseResult<TokenStream> {
    let DeclInfo {
        where_,
        generic_params,
        name,
        name_string,
    } = decl_get_info(&decl);

    let err = format!("missing expected value {name_string}");
    let mut body = quote! {
        let root = {
            let dict = variant.try_to::<::godot::builtin::Dictionary>()?;
            let value = dict.get(#name_string);
            value.ok_or(ConvertError::with_cause_value(#err, dict))?
        };
    };

    match decl {
        Declaration::Struct(s) => match s.fields {
            StructFields::Unit => make_unit_struct(&mut body),
            StructFields::Tuple(fields) if fields.fields.len() == 1 => {
                make_new_type_struct(&mut body, fields)
            }
            StructFields::Tuple(fields) => make_tuple_struct(fields, &mut body, &name),
            StructFields::Named(fields) => make_named_struct(fields, &mut body, &name),
        },
        Declaration::Enum(enum_) => {
            if enum_.variants.is_empty() {
                // Uninhabited enums have no values, so we cannot convert an actual Variant into them.
                body = quote! {
                    panic!("cannot convert Variant into uninhabited enum {}", #name_string);
                }
            } else {
                let mut matches = quote! {};
                for (enum_v, _) in &enum_.variants.inner {
                    let variant_name = enum_v.name.clone();
                    let variant_name_string = enum_v.name.to_string();
                    let if_let_content = match &enum_v.contents {
                        _ if has_attr_skip(&enum_v.attributes) => {
                            quote! {
                                if root == Variant::nil() {
                                    return Ok(Self::default());
                                }
                            }
                        }
                        StructFields::Unit if !has_attr_skip(&enum_v.attributes) => {
                            quote! {
                                let child = root.try_to::<String>();
                                if let Ok(child) = child {
                                    if child == #variant_name_string {
                                        return Ok(Self::#variant_name);
                                    }
                                }
                            }
                        }
                        StructFields::Unit => quote! {},
                        StructFields::Tuple(fields) if fields.fields.len() == 1 => {
                            let (field, _) = fields.fields.first().unwrap();
                            if has_attr_skip(&field.attributes) {
                                make_enum_new_type_skipped(
                                    field,
                                    &variant_name,
                                    &variant_name_string,
                                )
                            } else {
                                make_enum_new_type(field, &variant_name, &variant_name_string)
                            }
                        }
                        StructFields::Tuple(fields) => {
                            make_enum_tuple(fields, &variant_name, &variant_name_string)
                        }
                        StructFields::Named(fields) => {
                            make_enum_named(fields, &variant_name, &variant_name_string)
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
                    Err(ConvertError::new())
                };
            }
        }

        // decl_get_info() above ensured that no other cases are possible.
        _ => unreachable!(),
    }

    let gen = generic_params.as_ref().map(|x| x.as_inline_args());
    Ok(quote! {
        impl #generic_params ::godot::builtin::meta::FromGodot for #name #gen #where_ {
            fn try_from_godot(
                variant: ::godot::builtin::Variant
            ) -> Result<Self, ::godot::builtin::meta::ConvertError> {
                use ::godot::builtin::meta::ConvertError;
                let variant = &variant;
                #body
            }
        }
    })
}

fn make_named_struct(
    fields: venial::NamedStructFields,
    body: &mut TokenStream,
    name: &impl ToTokens,
) {
    let fields = fields.fields.iter().map(|(field, _)| {
        let ident = &field.name;
        let string_ident = &field.name.to_string();

        if has_attr_skip(&field.attributes) {
            (quote! {}, quote! { #ident: #name::default().#ident })
        } else {
            let err = format!("missing expected value {string_ident}");
            (
                quote! {
                    let #ident = match root.get(#string_ident) {
                        Some(value) => value,
                        None => return Err(ConvertError::with_cause_value(#err, root)),
                    };
                },
                quote! { #ident: #ident.try_to()? },
            )
        }
    });
    let (set_idents, set_self): (Vec<_>, Vec<_>) = fields.unzip();
    *body = quote! {
        #body
        let root = root.try_to::<::godot::builtin::Dictionary>()?;
        #(
            #set_idents
        )*
        Ok(Self { #(#set_self,)* })
    }
}

fn make_tuple_struct(
    fields: venial::TupleStructFields,
    body: &mut TokenStream,
    name: &impl ToTokens,
) {
    let ident_and_set = fields.fields.iter().enumerate().map(|(k, (f, _))| {
        let ident = format_ident!("__{}", k);
        let field_type = f.ty.to_token_stream();
        (
            ident.clone(),
            if has_attr_skip(&f.attributes) {
                quote! {
                    let #ident = <#name as Default>::default().#ident;
                }
            } else {
                quote! {
                    let #ident = match root.pop_front() {
                        Some(value) => value.try_to::<#field_type>()?,
                        None => return Err(ConvertError::with_cause_value("missing expected value", root)),
                    };
                }
            },
        )
    });
    let (idents, ident_set): (Vec<_>, Vec<_>) = ident_and_set.unzip();
    *body = quote! {
        #body
        let mut root = root.try_to::<::godot::builtin::VariantArray>()?;
        #(
            #ident_set
        )*
        Ok(Self(
            #(#idents,)*
        ))
    };
}

fn make_new_type_struct(body: &mut TokenStream, fields: venial::TupleStructFields) {
    *body = if has_attr_skip(&fields.fields.first().unwrap().0.attributes) {
        quote! { Ok(Self::default()) }
    } else {
        quote! {
            #body
            let root = root.try_to()?;
            Ok(Self(root))
        }
    }
}

fn make_unit_struct(body: &mut TokenStream) {
    *body = quote! {
        #body
        return Ok(Self);
    }
}

fn make_enum_new_type(
    field: &TupleField,
    variant_name: &impl ToTokens,
    variant_name_string: &impl ToTokens,
) -> TokenStream {
    let field_type = &field.ty;
    quote! {
        if let Ok(child) = root.try_to::<::godot::builtin::Dictionary>() {
            if let Some(variant) = child.get(#variant_name_string) {
                return Ok(Self::#variant_name(variant.try_to::<#field_type>()?));
            }
        }
    }
}

fn make_enum_new_type_skipped(
    field: &TupleField,
    variant_name: &impl ToTokens,
    variant_name_string: &impl ToTokens,
) -> TokenStream {
    let field_type = &field.ty;
    quote! {
        if let Ok(child) = root.try_to::<::godot::builtin::Dictionary>() {
            if let Some(v) = child.get(#variant_name_string) {
                if v.is_nil() {
                    return Ok(Self::#variant_name(
                        <#field_type as Default>::default(),
                    ));
                }
            }
        }
    }
}

fn make_enum_tuple(
    fields: &TupleStructFields,
    variant_name: &impl ToTokens,
    variant_name_string: &impl ToTokens,
) -> TokenStream {
    let fields = fields.fields.iter().enumerate().map(|(k, (field, _))| {
        let ident = format_ident!("__{k}");
        let field_type = &field.ty;
        let set_ident = if has_attr_skip(&field.attributes) {
            quote! {
                let #ident = <#field_type as Default>::default();
            }
        } else {
            quote! {
                let #ident = variant.pop_front()
                    .ok_or(ConvertError::with_cause_value("missing expected value", &variant))?
                    .try_to::<#field_type>()?;
            }
        };
        (ident.to_token_stream(), set_ident)
    });
    let (idents, set_idents): (Vec<_>, Vec<_>) = fields.unzip();

    quote! {
        let child = root.try_to::<::godot::builtin::Dictionary>();
        if let Ok(child) = child {
            if let Some(variant) = child.get(#variant_name_string) {
                let mut variant = variant.try_to::<::godot::builtin::VariantArray>()?;
                #(#set_idents)*
                return Ok(Self::#variant_name(#(#idents ,)*));
            }
        }
    }
}
fn make_enum_named(
    fields: &NamedStructFields,
    variant_name: &impl ToTokens,
    variant_name_string: &impl ToTokens,
) -> TokenStream {
    let fields = fields.fields.iter().map(|(field, _)| {
        let field_name = &field.name;
        let field_name_string = &field.name.to_string();
        let field_type = &field.ty;
        let set_field = if has_attr(&field.attributes, "variant", "skip") {
            quote! {
                let #field_name = <#field_type as Default>::default();
            }
        } else {
            let err = format!("missing expected value {field_name_string}");
            quote! {
                let #field_name = variant.get(#field_name_string)
                    .ok_or(ConvertError::with_cause_value(#err, &variant))?
                    .try_to::<#field_type>()?;
            }
        };
        (field_name.to_token_stream(), set_field)
    });

    let (fields, set_fields): (Vec<_>, Vec<_>) = fields.unzip();
    quote! {
        if let Ok(root) = root.try_to::<::godot::builtin::Dictionary>() {
            if let Some(variant) = root.get(#variant_name_string) {
                let variant = variant.try_to::<::godot::builtin::Dictionary>()?;
                #(
                    #set_fields
                )*
                return Ok(Self::#variant_name {
                    #( #fields, )*
                });
            }
        }
    }
}
