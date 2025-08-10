/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;

use super::c_style_enum::CStyleEnum;
use super::godot_attribute::{GodotAttribute, ViaType};
use super::newtype::NewtypeStruct;
use crate::util::bail;
use crate::ParseResult;

/// Stores all relevant data to derive `GodotConvert` and other related traits.
pub struct GodotConvert {
    /// The name of the type we're deriving for.
    pub ty_name: Ident,
    /// The data from the type and `godot` attribute.
    pub convert_type: ConvertType,
}

impl GodotConvert {
    pub fn parse_declaration(item: venial::Item) -> ParseResult<Self> {
        let (name, where_clause, generic_params) = match &item {
            venial::Item::Struct(struct_) => (
                struct_.name.clone(),
                &struct_.where_clause,
                &struct_.generic_params,
            ),
            venial::Item::Enum(enum_) => (
                enum_.name.clone(),
                &enum_.where_clause,
                &enum_.generic_params,
            ),
            other => {
                return bail!(
                    other,
                    "#[derive(GodotConvert)] only supports structs and enums"
                )
            }
        };

        if let Some(generic_params) = generic_params {
            return bail!(
                generic_params,
                "#[derive(GodotConvert)] does not support lifetimes or generic parameters"
            );
        }

        // Is this check even necessary? What's the use case of where clauses without generics?
        // For traits, one can imagine `Self: SomeBound`, but for structs/enums?
        if let Some(where_clause) = where_clause {
            return bail!(
                where_clause,
                "#[derive(GodotConvert)] does not support where clauses"
            );
        }

        let data = ConvertType::parse_declaration(item)?;

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
    pub fn parse_declaration(item: venial::Item) -> ParseResult<Self> {
        let attribute = GodotAttribute::parse_attribute(&item)?;

        match &item {
            venial::Item::Struct(struct_) => {
                let GodotAttribute::Transparent { .. } = attribute else {
                    return bail!(attribute.span(), "#[derive(GodotConvert)] on structs currently only works with #[godot(transparent)]");
                };

                Ok(Self::NewType {
                    field: NewtypeStruct::parse_struct(struct_)?,
                })
            }
            venial::Item::Enum(enum_) => {
                let GodotAttribute::Via { via_type, .. } = attribute else {
                    return bail!(
                        attribute.span(),
                        "#[derive(GodotConvert)] on enums requires #[godot(via = ...)]"
                    );
                };

                Ok(Self::Enum {
                    variants: CStyleEnum::parse_enum(enum_)?,
                    via: via_type,
                })
            }
            _ => unreachable!(), // already checked outside.
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
