/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Macro implementations used by `godot-ffi` crate.

#![cfg(feature = "experimental-wasm")]

use crate::util::bail;
use crate::ParseResult;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn wasm_declare_init_fn(input: TokenStream) -> ParseResult<TokenStream> {
    if !input.is_empty() {
        return bail!(input, "macro expects no arguments");
    }

    // Create sufficiently unique identifier without entire `uuid` (let alone `rand`) crate dependency.
    let a = unsafe { libc::rand() };
    let b = unsafe { libc::rand() };

    // Rust presently requires that statics with a custom `#[link_section]` must be a simple
    // list of bytes on the Wasm target (with no extra levels of indirection such as references).
    //
    // As such, instead we export a function with a random name of known prefix to be used by the embedder.
    // This prefix is queried at load time, see godot-macros/src/gdextension.rs.
    let function_name = format_ident!("__godot_rust_registrant_{a}_{b}");

    let code = quote! {
        #[cfg(target_family = "wasm")] // Strictly speaking not necessary, as this macro is only invoked for Wasm.
        #[no_mangle]
        extern "C" fn #function_name() {
            __init();
        }
    };

    Ok(code)
}
