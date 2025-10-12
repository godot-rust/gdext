/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::util::ident;

#[allow(unused)]
pub enum SysTypeParam {
    /// `* mut SysType`
    Mut(&'static str),
    /// `* const SysType`
    Const(&'static str),
}

impl SysTypeParam {
    pub fn type_(&self) -> &'static str {
        match self {
            SysTypeParam::Mut(s) | SysTypeParam::Const(s) => s,
        }
    }

    fn to_ident(&self) -> Ident {
        match self {
            SysTypeParam::Mut(s) | SysTypeParam::Const(s) => ident(s),
        }
    }
}

impl ToTokens for SysTypeParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let type_ident = self.to_ident();
        match self {
            SysTypeParam::Mut(_) => quote! { * mut crate::sys::#type_ident },
            SysTypeParam::Const(_) => quote! { * const crate::sys::#type_ident },
        }
        .to_tokens(tokens);
    }
}

/// SysTypes used as parameters in various APIs defined in `extension_api.json`.
// Currently hardcoded and it probably will stay this way – extracting types from gdextension_interface is way too messy.
// Must be different abstraction to avoid clashes with other types passed as pointers (e.g. *Glyph).
pub static SYS_PARAMS: &[SysTypeParam] = &[
    #[cfg(since_api = "4.6")]
    SysTypeParam::Const("GDExtensionInitializationFunction"),
];

/// Creates `GodotConvert`, `ToGodot` and `FromGodot` impl
/// for SysTypes – various pointer types declared in `gdextension_interface`.
pub fn make_godotconvert_for_systypes() -> Vec<TokenStream> {
    let mut tokens = vec![];
    for sys_type_param in SYS_PARAMS {
        tokens.push(
            quote! {
                    impl crate::meta::GodotConvert for #sys_type_param {
                        type Via = i64;

                    }

                    impl crate::meta::ToGodot for #sys_type_param {
                        type Pass = crate::meta::ByValue;
                        fn to_godot(&self) -> Self::Via {
                            * self as i64
                        }
                    }

                    impl crate::meta::FromGodot for #sys_type_param {
                        fn try_from_godot(via: Self::Via) -> Result < Self, crate::meta::error::ConvertError > {
                            Ok(via as Self)
                        }
                    }
            }
        )
    }
    tokens
}
