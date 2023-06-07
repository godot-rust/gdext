/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::path::Path;

use quote::quote;

use crate::class_generator::make_utility_function_definition;
use crate::Context;
use crate::{api_parser::*, SubmitFn};

pub(crate) fn generate_utilities_file(
    api: &ExtensionApi,
    ctx: &mut Context,
    gen_path: &Path,
    submit_fn: &mut SubmitFn,
) {
    let mut utility_fn_defs = vec![];
    for utility_fn in &api.utility_functions {
        // note: category unused -> could be their own mod

        let def = make_utility_function_definition(utility_fn, ctx);
        utility_fn_defs.push(def);
    }

    let tokens = quote! {
        //! Global utility functions.
        //!
        //! A list of global-scope built-in functions.
        //! For global enums and constants, check out the [`global` module][crate::engine::global].
        //!
        //! See also [Godot docs for `@GlobalScope`](https://docs.godotengine.org/en/stable/classes/class_@globalscope.html#methods).

        use godot_ffi as sys;
        use crate::builtin::*;
        use crate::obj::Gd;
        use crate::engine::Object;
        use sys::GodotFfi as _;

        #(#utility_fn_defs)*
    };

    submit_fn(gen_path.join("utilities.rs"), tokens);
}
