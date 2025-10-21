/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::context::Context;

/// Creates `GodotConvert`, `ToGodot` and `FromGodot` impl
/// for SysPointerTypes – various pointer types declared in `gdextension_interface`
/// and used as parameters in exposed Godot APIs.
pub fn make_godotconvert_for_systypes(ctx: &mut Context) -> Vec<TokenStream> {
    ctx.cached_sys_pointer_types().map(|sys_pointer_type| {
        quote! {
            impl crate::meta::GodotConvert for #sys_pointer_type {
                type Via = i64;
            }

            impl crate::meta::ToGodot for #sys_pointer_type {
                type Pass = crate::meta::ByValue;
                fn to_godot(&self) -> Self::Via {
                    *self as i64
                }
            }

            impl crate::meta::FromGodot for #sys_pointer_type {
                fn try_from_godot(via: Self::Via) -> Result <Self, crate::meta::error::ConvertError> {
                    Ok(via as Self)
                }
            }
        }
    }).collect()
}
