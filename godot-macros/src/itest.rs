/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::util::{
    bail, extract_typename, ident, path_ends_with, retain_attributes_except, KvParser,
};
use crate::ParseResult;

pub fn attribute_itest(input_item: venial::Item) -> ParseResult<TokenStream> {
    let func = match input_item {
        venial::Item::Function(f) => f,
        _ => return bail!(&input_item, "#[itest] can only be applied to functions"),
    };

    let mut attr = KvParser::parse_required(&func.attributes, "itest", &func.name)?;
    let skipped = attr.handle_alone("skip")?;
    let focused = attr.handle_alone("focus")?;
    let is_async = attr.handle_alone("async")?;
    attr.finish()?;

    // Note: allow attributes for things like #[rustfmt] or #[clippy]
    if func.generic_params.is_some()
        || func.params.len() > 1
        || (func.return_ty.is_some() && !is_async)
        || func.where_clause.is_some()
    {
        return bad_signature(&func);
    }

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
        if let venial::FnParam::Typed(param) = param {
            // Correct parameter type (crude macro check) -> reuse parameter name
            if path_ends_with(&param.ty.tokens, "TestContext") {
                param.to_token_stream()
            } else if is_async {
                return bad_async_signature(&func);
            } else {
                return bad_signature(&func);
            }
        } else if is_async {
            return bad_async_signature(&func);
        } else {
            return bad_signature(&func);
        }
    } else {
        quote! { __unused_context: &crate::framework::TestContext }
    };

    let return_ty = func.return_ty.as_ref();
    if is_async
        && return_ty
            .and_then(extract_typename)
            .is_none_or(|segment| segment.ident != "TaskHandle")
    {
        return bad_async_signature(&func);
    }

    let body = &func.body;

    let (return_tokens, test_case_ty, plugin_name);
    if is_async {
        let [arrow, arrow_head] = func.tk_return_arrow.unwrap();
        return_tokens = quote! { #arrow #arrow_head #return_ty }; // retain span.
        test_case_ty = quote! { crate::framework::AsyncRustTestCase };
        plugin_name = ident("__GODOT_ASYNC_ITEST");
    } else {
        return_tokens = TokenStream::new();
        test_case_ty = quote! { crate::framework::RustTestCase };
        plugin_name = ident("__GODOT_ITEST");
    };

    // Filter out #[itest] itself, but preserve other attributes like #[allow], #[expect], etc.
    let other_attributes = retain_attributes_except(&func.attributes, "itest");

    Ok(quote! {
        #(#other_attributes)*
        pub fn #test_name(#param) #return_tokens {
            #body
        }

        ::godot::sys::plugin_add!(crate::framework::#plugin_name; #test_case_ty {
            name: #test_name_str,
            skipped: #skipped,
            focused: #focused,
            file: std::file!(),
            line: std::line!(),
            function: #test_name,
        });
    })
}

fn bad_signature(func: &venial::Function) -> Result<TokenStream, venial::Error> {
    bail!(
        func,
        "#[itest] function must have one of these signatures:\
        \n  fn {f}() {{ ... }}\
        \n  fn {f}(ctx: &TestContext) {{ ... }}",
        f = func.name,
    )
}

fn bad_async_signature(func: &venial::Function) -> Result<TokenStream, venial::Error> {
    bail!(
        func,
        "#[itest(async)] function must have one of these signatures:\
        \n  fn {f}() -> TaskHandle {{ ... }}\
        \n  fn {f}(ctx: &TestContext) -> TaskHandle {{ ... }}",
        f = func.name,
    )
}
