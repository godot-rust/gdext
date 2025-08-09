/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Macro implementations used by `godot-ffi` crate.

#![cfg(feature = "experimental-wasm")]

use std::env;
use std::sync::atomic::{AtomicU32, Ordering};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::util::bail;
use crate::ParseResult;

// Note: global state in proc-macros may become problematic in the future, see:
// https://users.rust-lang.org/t/simple-state-in-procedural-macro/68204/2
static FUNCTION_COUNTER: AtomicU32 = AtomicU32::new(0);

pub(super) fn wasm_declare_init_fn(input: TokenStream) -> ParseResult<TokenStream> {
    if !input.is_empty() {
        return bail!(input, "macro expects no arguments");
    }

    let crate_name = env::var("CARGO_CRATE_NAME")
        .expect("CARGO_CRATE_NAME env var not found. This macro must be run by Cargo.");

    let crate_version = env::var("CARGO_PKG_VERSION")
        .expect("CARGO_PKG_VERSION env var not found. This macro must be run by Cargo.")
        // SemVer version allows digits, alphanumerics, dots, hyphens and plus signs. Replacement may technically
        // map strings like "1.2.3-alpha.4" and "1.2.3+alpha.4" to the same identifier, but that's a very unlikely edge case.
        .replace(['.', '+', '-'], "_");

    let index = FUNCTION_COUNTER.fetch_add(1, Ordering::Relaxed);

    // Rust presently requires that statics with a custom `#[link_section]` must be a simple
    // list of bytes on the Wasm target (with no extra levels of indirection such as references).
    //
    // As such, instead we export a function with a known prefix to be used by the embedder.
    // This prefix is queried at load time, see godot-macros/src/gdextension.rs.
    let function_name =
        format_ident!("__godot_rust_registrant__{crate_name}__v{crate_version}__i{index}");

    let code = quote! {
        #[cfg(target_family = "wasm")] // Strictly speaking not necessary, as this macro is only invoked for Wasm.
        #[no_mangle]
        extern "C" fn #function_name() {
            __init();
        }
    };

    Ok(code)
}
