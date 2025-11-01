/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use proc_macro2::TokenStream;
use quote::quote;

use crate::util::{bail, retain_attributes_except, KvParser};
use crate::ParseResult;

const DEFAULT_REPETITIONS: usize = 100;

pub fn attribute_bench(input_decl: venial::Item) -> ParseResult<TokenStream> {
    let func = match input_decl {
        venial::Item::Function(f) => f,
        _ => return bail!(&input_decl, "#[bench] can only be applied to functions"),
    };

    // Disallow generics, but allow attributes for things like #[rustfmt] or #[clippy].
    if func.generic_params.is_some() || func.where_clause.is_some() {
        return bad_signature(&func);
    }

    let mut attr = KvParser::parse_required(&func.attributes, "bench", &func.name)?;
    let manual = attr.handle_alone("manual")?;
    let repetitions = attr.handle_usize("repeat")?;
    attr.finish()?;

    // Validate attribute combinations.
    if manual && repetitions.is_some() {
        return bail!(
            func,
            "#[bench(manual)] cannot be combined with `repeat` -- pass repetitions to bench_measure() instead"
        );
    }

    let repetitions = repetitions.unwrap_or(DEFAULT_REPETITIONS);

    // Validate parameter count.
    if !func.params.is_empty() {
        return bad_signature(&func);
    }

    let bench_name = &func.name;
    let bench_name_str = func.name.to_string();

    let body = &func.body;

    // Filter out #[bench] itself, but preserve other attributes like #[allow], #[expect], etc.
    let other_attributes: Vec<_> = retain_attributes_except(&func.attributes, "bench").collect();

    let generated_fn = if manual {
        // Manual mode: user calls bench_measure() directly.
        let ret = func.return_ty;
        quote! {
            #(#other_attributes)*
            // Don't return crate::framework::BenchResult here, to keep user imports working naturally.
            pub fn #bench_name() -> #ret {
                #body
            }
        }
    } else {
        // Automatic mode: framework controls timing.
        // Ignore `-> ()`, as no one does that by accident.
        // We need `ret` to make sure the type is correct and to avoid unused imports (by IDEs).
        let Some(ret) = func.return_ty else {
            return bail!(
                func,
                "#[bench] function must return a value from its computation, to prevent optimizing the operation away"
            );
        };

        quote! {
            #(#other_attributes)*
            pub fn #bench_name() -> crate::framework::BenchResult {
                crate::framework::bench_measure(#repetitions, || {
                    let __ret: #ret = #body;
                    __ret // passed onto bench_used() by caller.
                })
            }
        }
    };

    Ok(quote! {
        #generated_fn

        ::godot::sys::plugin_add!(crate::framework::__GODOT_BENCH; crate::framework::RustBenchmark {
            name: #bench_name_str,
            file: std::file!(),
            line: std::line!(),
            function: #bench_name,
        });
    })
}

fn bad_signature(func: &venial::Function) -> Result<TokenStream, venial::Error> {
    bail!(
        func,
        "#[bench] function must have one of these signatures:\
        \n\
        \n(1)  #[bench]\
        \n     fn {f}() -> T {{ ... }}\
        \n\
        \n(2)  #[bench(manual)]\
        \n     fn {f}() -> BenchResult {{ ... /* call to bench_measure() */ }}",
        f = func.name,
    )
}
