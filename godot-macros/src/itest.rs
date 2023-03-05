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
    let test_name_str = func.name.to_string();
    let body = &func.body;

    Ok(quote! {
       /*#[doc(hidden)]
        #[must_use]
        pub fn #test_name() -> bool {
            println!(#init_msg);

            // Explicit type to prevent tests from returning a value
            let success: Option<()> = godot::private::handle_panic(
                || #error_msg,
                || #body
            );

            success.is_some()
        }*/

        pub fn #test_name() {
            #body
        }

        ::godot::sys::plugin_add!(__GODOT_ITEST in crate; crate::RustTestCase {
            name: #test_name_str,
            skipped: false,
            file: std::file!(),
            line: std::line!(),
            function: #test_name,
        });
    })
}
