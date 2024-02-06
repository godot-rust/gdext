/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use venial::{Declaration, TyExpr};

use crate::{
    util::{bail, decl_get_info, ident, DeclInfo, KvParser},
    ParseResult,
};

pub struct GodotConvert {
    pub name: Ident,
    pub data: ConvertData,
}

impl GodotConvert {
    pub fn parse_declaration(declaration: Declaration) -> ParseResult<Self> {
        let DeclInfo { name, where_, generic_params, .. } = decl_get_info(&declaration);
        
        if let Some(where_) = where_ {
            return bail!(where_, "where clauses are currently not supported for `GodotConvert`")
        }

        if let Some(generic_params) = generic_params {
            return bail!(generic_params, "generics are currently not supported for `GodotConvert`")
        }

        let data = ConvertData::parse_declaration(&declaration)?;

        Ok(Self { name, data })
    }

    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn data(&self) -> &ConvertData {
        &self.data
    }
}

pub enum ConvertData {
    NewType { field: NewtypeField },
    Enum { variants: CStyleEnum, via: ViaType },
}

impl ConvertData {
    pub fn parse_declaration(declaration: &Declaration) -> ParseResult<Self> {
        let attribute = GodotAttribute::parse_attribute(declaration)?;

        match declaration {
            Declaration::Struct(struct_) => {
                if let GodotAttribute::Via { via_type: ty } = attribute {
                    return bail!(ty.span(), "`GodotConvert` on structs only works with `#[godot(transparent)]` currently");
                }

                Ok(Self::NewType {
                    field: NewtypeField::parse_struct(struct_)?,
                })
            }
            Declaration::Enum(enum_) => {
                let via_type = match attribute {
                    GodotAttribute::Transparent { span } => {
                        return bail!(
                            span,
                            "`GodotConvert` on enums requires `#[godot(via = ..)]` currently"
                        )
                    }
                    GodotAttribute::Via { via_type: ty } => ty,
                };

                Ok(Self::Enum {
                    variants: CStyleEnum::parse_enum(enum_)?,
                    via: via_type,
                })
            }
            _ => bail!(
                declaration,
                "`GodotConvert` only supports structs and enums currently"
            ),
        }
    }

    pub fn via_type(&self) -> TokenStream {
        match self {
            ConvertData::NewType { field } => field.ty.to_token_stream(),
            ConvertData::Enum { variants, via } => via.to_token_stream(),
        }
    }
}

pub enum GodotAttribute {
    Transparent { span: Span },
    Via { via_type: ViaType },
}
impl GodotAttribute {
    pub fn parse_attribute(declaration: &Declaration) -> ParseResult<Self> {
        let mut parser = KvParser::parse_required(declaration.attributes(), "godot", declaration)?;

        let span = parser.span();

        if parser.handle_alone("transparent")? {
            return Ok(Self::Transparent { span });
        }

        let via_type = parser.handle_ident_required("via")?;

        let span = via_type.span();

        let via_type = match via_type.to_string().as_str() {
            "GString" => ViaType::GString(span),
            "i8" => ViaType::Int(span, ViaInt::I8),
            "i16" => ViaType::Int(span, ViaInt::I16),
            "i32" => ViaType::Int(span, ViaInt::I32),
            "i64" => ViaType::Int(span, ViaInt::I64),
            "u8" => ViaType::Int(span, ViaInt::U8),
            "u16" => ViaType::Int(span, ViaInt::U16),
            "u32" => ViaType::Int(span, ViaInt::U32),
            other => return bail!(via_type, "Via type `{}` is not supported, expected one of: GString, i8, i16, i32, i64, u8, u16, u32", other)
        };

        Ok(Self::Via { via_type })
    }

    pub fn span(&self) -> Span {
        match self {
            GodotAttribute::Transparent { span } => span.clone(),
            GodotAttribute::Via { via_type } => via_type.span(),
        }
    }
}

pub enum ViaType {
    GString(Span),
    Int(Span, ViaInt),
}

impl ViaType {
    fn span(&self) -> Span {
        match self {
            ViaType::GString(span) => span.clone(),
            ViaType::Int(span, _) => span.clone(),
        }
    }

    fn to_token_stream(&self) -> TokenStream {
        match self {
            ViaType::GString(_) => quote! { ::godot::builtin::GString },
            ViaType::Int(_, int) => {
                let id = int.to_ident();
                quote! { #id }
            },
        }
    }
}

pub enum ViaInt {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
}

impl ViaInt {
    pub fn to_ident(&self) -> Ident {
        match self {
            ViaInt::I8 => ident("i8" ),
            ViaInt::I16 => ident("i16" ),
            ViaInt::I32 => ident("i32" ),
            ViaInt::I64 => ident("i64" ),
            ViaInt::U8 => ident("u8" ),
            ViaInt::U16 => ident("u16" ),
            ViaInt::U32 => ident("u32" ),
        }
    }
}

pub struct NewtypeField {
    // If none, then it's the first field of a tuple-struct.
    pub name: Option<Ident>,
    pub ty: TyExpr,
}

impl NewtypeField {
    pub fn parse_struct(struct_: &venial::Struct) -> ParseResult<NewtypeField> {
        match &struct_.fields {
            venial::StructFields::Unit => return bail!(&struct_.fields, "GodotConvert expects a struct with a single field, unit structs are currently not supported"),
            venial::StructFields::Tuple(fields) => {
                if fields.fields.len() != 1 {
                    return bail!(&fields.fields, "GodotConvert expects a struct with a single field, not {} fields", fields.fields.len())
                }
    
                let (field, _) = fields.fields[0].clone();
    
                Ok(NewtypeField { name: None, ty: field.ty })
            },
            venial::StructFields::Named(fields) => {
                if fields.fields.len() != 1 {
                    return bail!(&fields.fields, "GodotConvert expects a struct with a single field, not {} fields", fields.fields.len())
                }
    
                let (field, _) = fields.fields[0].clone();
    
                Ok(NewtypeField { name: Some(field.name), ty: field.ty })
            },
        }
    }

    pub fn field_name(&self) -> TokenStream {
        match &self.name {
            Some(name) => quote! { #name },
            None => quote! { 0 },
        }
    }



}

#[derive(Debug, Clone)]
pub struct CStyleEnumVariant {
    pub name: Ident,
    pub discriminant: Option<TokenTree>,
}

impl CStyleEnumVariant {
    pub fn parse_enum_variant(enum_variant: &venial::EnumVariant) -> ParseResult<Self> {
        match enum_variant.contents {
            venial::StructFields::Unit => {}
            _ => {
                return bail!(
                    &enum_variant.contents,
                    "GodotConvert only supports c-style enums (enums without fields)"
                )
            }
        }

        Ok(Self {
            name: enum_variant.name.clone(),
            discriminant: enum_variant.value.as_ref().map(|val| &val.value).cloned(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CStyleEnum {
    names: Vec<Ident>,
    discriminants: Vec<i64>,
}

impl CStyleEnum {
    pub fn parse_enum(enum_: &venial::Enum) -> ParseResult<Self> {
        let variants = enum_
            .variants
            .items()
            .map(|enum_variant| CStyleEnumVariant::parse_enum_variant(enum_variant))
            .collect::<ParseResult<Vec<_>>>()?;

        let mut key_values = Self::to_key_value_assignment(variants)?
            .into_iter()
            .collect::<Vec<_>>();

        key_values.sort_by_key(|kv| kv.0);

        let mut names = Vec::new();
        let mut discriminants = Vec::new();

        for (key, value) in key_values.into_iter() {
            names.push(value);
            discriminants.push(key);
        }

        Ok(Self {
            names,
            discriminants,
        })
    }

    fn to_key_value_assignment(
        variants: Vec<CStyleEnumVariant>,
    ) -> ParseResult<HashMap<i64, Ident>> {
        let mut unassigned_names = std::collections::VecDeque::new();
        let mut discriminants = HashMap::new();

        for variant in variants.iter() {
            if let Some(disc) = &variant.discriminant {
                let Ok(disc_int) = disc.to_string().parse::<i64>() else {
                    return bail!(disc, "expected integer literal discriminant");
                };

                discriminants.insert(disc_int, variant.name.clone());
            } else {
                unassigned_names.push_back(variant.name.clone());
            }
        }

        for i in 0.. {
            if unassigned_names.is_empty() {
                break;
            }

            if discriminants.contains_key(&i) {
                continue;
            }

            let name = unassigned_names.pop_front().unwrap();

            discriminants.insert(i, name);
        }
        Ok(discriminants)
    }

    pub fn discriminants(&self) -> &[i64] {
        &self.discriminants
    }

    pub fn names(&self) -> &[Ident] {
        &self.names
    }

    pub fn to_int_hint(&self) -> String {
        self.names
            .iter()
            .zip(self.discriminants.iter())
            .map(|(name, discrim)| format!("{name}:{discrim}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn to_string_hint(&self) -> String {
        self.names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}
