/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use venial::Declaration;

use crate::util::bail;
use crate::ParseResult;

use super::c_style_enum::CStyleEnum;
use super::godot_attribute::{GodotAttribute, ViaType};
use super::newtype::NewtypeStruct;

/// Stores all relevant data to derive `GodotConvert` and other related traits.
pub struct GodotConvert {
    /// The name of the type we're deriving for.
    pub ty_name: Ident,
    /// The data from the type and `godot` attribute.
    pub convert_type: ConvertType,
}

impl GodotConvert {
    pub fn parse_declaration(declaration: Declaration) -> ParseResult<Self> {
        let (name, where_clause, generic_params) = match &declaration {
            venial::Declaration::Struct(struct_) => (
                struct_.name.clone(),
                &struct_.where_clause,
                &struct_.generic_params,
            ),
            venial::Declaration::Enum(enum_) => (
                enum_.name.clone(),
                &enum_.where_clause,
                &enum_.generic_params,
            ),
            other => {
                return bail!(
                    other,
                    "`GodotConvert` only supports structs and enums currently"
                )
            }
        };

        if let Some(where_clause) = where_clause {
            return bail!(
                where_clause,
                "`GodotConvert` does not support where clauses"
            );
        }

        if let Some(generic_params) = generic_params {
            return bail!(generic_params, "`GodotConvert` does not support generics");
        }

        let data = ConvertType::parse_declaration(declaration)?;

        Ok(Self {
            ty_name: name,
            convert_type: data,
        })
    }
}

/// Stores what kind of `GodotConvert` derive we're doing.
pub enum ConvertType {
    /// Deriving for a newtype struct.
    NewType { field: NewtypeStruct },
    /// Deriving for an enum.
    Enum { variants: CStyleEnum, via: ViaType },
}

impl ConvertType {
    pub fn parse_declaration(declaration: Declaration) -> ParseResult<Self> {
        let attribute = GodotAttribute::parse_attribute(&declaration)?;

        match &declaration {
            Declaration::Struct(struct_) => {
                let GodotAttribute::Transparent { .. } = attribute else {
                    return bail!(attribute.span(), "`GodotConvert` on structs only works with `#[godot(transparent)]` currently");
                };

                Ok(Self::NewType {
                    field: NewtypeStruct::parse_struct(struct_)?,
                })
            }
            Declaration::Enum(enum_) => {
                let GodotAttribute::Via { via_type, .. } = attribute else {
                    return bail!(
                        attribute.span(),
                        "`GodotConvert` on enums requires `#[godot(via = ...)]` currently"
                    );
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

    /// Returns the type for use in `type Via = <type>;` in `GodotConvert` implementations.
    pub fn via_type(&self) -> TokenStream {
        match self {
            ConvertType::NewType { field } => field.ty.to_token_stream(),
            ConvertType::Enum { via, .. } => via.to_token_stream(),
        }
    }
}
