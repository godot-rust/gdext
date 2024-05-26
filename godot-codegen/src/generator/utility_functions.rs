/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::generator::functions_common;
use crate::generator::functions_common::{FnCode, FnReceiver};
use crate::models::domain::{ExtensionApi, Function, UtilityFunction};
use crate::{util, SubmitFn};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::path::Path;

pub(crate) fn generate_utilities_file(
    api: &ExtensionApi,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    // Note: category unused -> could be their own mod.
    let utility_fn_defs = api
        .utility_functions
        .iter()
        .map(make_utility_function_definition);

    let imports = util::make_imports();

    let tokens = quote! {
        #imports

        #(#utility_fn_defs)*
    };

    submit_fn(gen_path.join("utilities.rs"), tokens);
}

pub(crate) fn make_utility_function_ptr_name(godot_function_name: &str) -> Ident {
    util::safe_ident(godot_function_name)
}

pub(crate) fn make_utility_function_definition(function: &UtilityFunction) -> TokenStream {
    let function_name_str = function.name();
    let fn_ptr = make_utility_function_ptr_name(function_name_str);

    let ptrcall_invocation = quote! {
        let utility_fn = sys::utility_function_table().#fn_ptr;

        <CallSig as PtrcallSignatureTuple>::out_utility_ptrcall(
            utility_fn,
            #function_name_str,
            args
        )
    };

    let varcall_invocation = quote! {
        let utility_fn = sys::utility_function_table().#fn_ptr;

        <CallSig as VarcallSignatureTuple>::out_utility_ptrcall_varargs(
            utility_fn,
            #function_name_str,
            args,
            varargs
        )
    };

    let definition = functions_common::make_function_definition(
        function,
        &FnCode {
            receiver: FnReceiver::global_function(),
            varcall_invocation,
            ptrcall_invocation,
        },
        None,
        &TokenStream::new(),
    );

    // Utility functions have no builders.
    definition.into_functions_only()
}
