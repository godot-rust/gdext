/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::generator::functions_common;
use crate::generator::functions_common::{FnCode, FnReceiver};
use crate::models::domain::{ExtensionApi, Function, UtilityFunction};
use crate::{util, SubmitFn};

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

pub(crate) fn make_utility_function_ptr_name(function: &dyn Function) -> Ident {
    function.name_ident()
}

pub(crate) fn make_utility_function_definition(function: &UtilityFunction) -> TokenStream {
    let function_ident = make_utility_function_ptr_name(function);
    let function_name_str = function.name();

    let ptrcall_invocation = quote! {
        let utility_fn = sys::utility_function_table().#function_ident;

        Signature::<CallParams, CallRet>::out_utility_ptrcall(
            utility_fn,
            #function_name_str,
            args
        )
    };

    let varcall_invocation = quote! {
        let utility_fn = sys::utility_function_table().#function_ident;

        Signature::<CallParams, CallRet>::out_utility_ptrcall_varargs(
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
            is_virtual_required: false,
            is_varcall_fallible: false,
        },
        None,
        &TokenStream::new(),
    );

    // Utility functions have no builders.
    definition.into_functions_only()
}
