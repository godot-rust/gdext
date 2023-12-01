/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;
use venial::{Declaration, Error, Function};

use crate::util::{bail, KvParser};
use crate::ParseResult;

const DEFAULT_REPETITIONS: usize = 100;

pub fn attribute_bench(input_decl: Declaration) -> ParseResult<TokenStream> {
    let func = match input_decl {
        Declaration::Function(f) => f,
        _ => return bail!(&input_decl, "#[bench] can only be applied to functions"),
    };

    // Note: allow attributes for things like #[rustfmt] or #[clippy]
    if func.generic_params.is_some() || !func.params.is_empty() || func.where_clause.is_some() {
        return bad_signature(&func);
    }

    // Ignore -> (), as no one does that by accident.
    // We need `ret` to make sure the type is correct and to avoid unused imports (by IDEs).
    let Some(ret) = func.return_ty else {
        return bail!(
            func,
            "#[bench] function must return a value from its computation, to prevent optimizing the operation away"
        );
    };

    let mut attr = KvParser::parse_required(&func.attributes, "bench", &func.name)?;
    let repetitions = attr.handle_usize("repeat")?.unwrap_or(DEFAULT_REPETITIONS);
    attr.finish()?;

    let bench_name = &func.name;
    let bench_name_str = func.name.to_string();

    let body = &func.body;

    Ok(quote! {
        pub fn #bench_name() {
            for _ in 0..#repetitions {
                let __ret: #ret = #body;
                crate::common::bench_used(__ret);
            }
        }

        ::godot::sys::plugin_add!(__GODOT_BENCH in crate::framework; crate::framework::RustBenchmark {
            name: #bench_name_str,
            file: std::file!(),
            line: std::line!(),
            function: #bench_name,
            repetitions: #repetitions,
        });
    })
}

fn bad_signature(func: &Function) -> Result<TokenStream, Error> {
    bail!(
        func,
        "#[bench] function must have one of these signatures:\
        \n  fn {f}() {{ ... }}",
        f = func.name,
    )
}
