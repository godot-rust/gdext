/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use quote::quote;

use crate::class_generator::make_utility_function_definition;
use crate::models::domain::ExtensionApi;
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
        //! Global utility functions.
        //!
        //! A list of global-scope built-in functions.
        //! For global enums and constants, check out the [`global` module][crate::engine::global].
        //!
        //! See also [Godot docs for `@GlobalScope`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#methods).

        #imports

        #(#utility_fn_defs)*
    };

    submit_fn(gen_path.join("utilities.rs"), tokens);
}
