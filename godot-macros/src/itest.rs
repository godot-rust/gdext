/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::util::bail;
use proc_macro2::TokenStream;
use quote::quote;
use venial::{Declaration, Error};

pub fn transform(input: TokenStream) -> Result<TokenStream, Error> {
    let input_decl = venial::parse_declaration(input)?;

    let func = match input_decl {
        Declaration::Function(f) => f,
        _ => return bail("#[itest] can only be applied to functions", &input_decl),
    };

    // Note: allow attributes for things like #[rustfmt] or #[clippy]
    if func.generic_params.is_some()
        || !func.params.is_empty()
        || func.return_ty.is_some()
        || func.where_clause.is_some()
    {
        return bail(
            format!("#[itest] must be of form:  fn {}() {{ ... }}", func.name),
            &func,
        );
    }

    let test_name = &func.name;
    let init_msg = format!("   -- {}", test_name);
    let error_msg = format!("   !! Test {} failed", test_name);
    let body = &func.body;

    Ok(quote! {
        #[doc(hidden)]
        #[must_use]
        pub fn #test_name() -> bool {
            println!(#init_msg);

            let result = ::std::panic::catch_unwind(
                || #body
            );

            if let Err(e) = result {
                ::godot::log::godot_error!(#error_msg);
                ::godot::private::print_panic(e);
                false
            } else {
                true
            }
        }
    })
}
