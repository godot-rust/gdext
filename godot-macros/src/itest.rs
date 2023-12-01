/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use venial::{Declaration, Error, FnParam, Function};

use crate::util::{bail, path_ends_with, KvParser};
use crate::ParseResult;

pub fn attribute_itest(input_decl: Declaration) -> ParseResult<TokenStream> {
    let func = match input_decl {
        Declaration::Function(f) => f,
        _ => return bail!(&input_decl, "#[itest] can only be applied to functions"),
    };

    // Note: allow attributes for things like #[rustfmt] or #[clippy]
    if func.generic_params.is_some()
        || func.params.len() > 1
        || func.return_ty.is_some()
        || func.where_clause.is_some()
    {
        return bad_signature(&func);
    }

    let mut attr = KvParser::parse_required(&func.attributes, "itest", &func.name)?;
    let skipped = attr.handle_alone("skip")?;
    let focused = attr.handle_alone("focus")?;
    attr.finish()?;

    if skipped && focused {
        return bail!(
            func.name,
            "#[itest]: keys `skip` and `focus` are mutually exclusive",
        );
    }

    let test_name = &func.name;
    let test_name_str = func.name.to_string();

    // Detect parameter name chosen by user, or unused fallback
    let param = if let Some((param, _punct)) = func.params.first() {
        if let FnParam::Typed(param) = param {
            // Correct parameter type (crude macro check) -> reuse parameter name
            if path_ends_with(&param.ty.tokens, "TestContext") {
                param.to_token_stream()
            } else {
                return bad_signature(&func);
            }
        } else {
            return bad_signature(&func);
        }
    } else {
        quote! { __unused_context: &crate::framework::TestContext }
    };

    let body = &func.body;

    Ok(quote! {
        pub fn #test_name(#param) {
            #body
        }

        ::godot::sys::plugin_add!(__GODOT_ITEST in crate::framework; crate::framework::RustTestCase {
            name: #test_name_str,
            skipped: #skipped,
            focused: #focused,
            file: std::file!(),
            line: std::line!(),
            function: #test_name,
        });
    })
}

fn bad_signature(func: &Function) -> Result<TokenStream, Error> {
    bail!(
        func,
        "#[itest] function must have one of these signatures:\
        \n  fn {f}() {{ ... }}\
        \n  fn {f}(ctx: &TestContext) {{ ... }}",
        f = func.name,
    )
}
