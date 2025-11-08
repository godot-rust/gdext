/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Codegen helpers to special-case `u64`, which doesn't have `ToGodot/FromGodot` impls.
//!
//! The impls are not provided since `u64` is not a natively supported type in GDScript (e.g. cannot be stored in a variant without altering the
//! value). So `#[func]` does not support it. However, engine APIs may still need it, so there is codegen/macro support.
//!
//! The `godot_codegen::generator::u64_handling` module provides the same functionality for codegen.

use proc_macro2::TokenStream;
use quote::quote;

/// Checks if a type is `u64`, which requires `i64` for FFI with casting.
///
/// Counterpart: `godot_codegen::generator::u64_handling::is_u64_type`.
pub(crate) fn is_u64_type(ty_expr: &venial::TypeExpr) -> bool {
    ty_expr.tokens.len() == 1 && ty_expr.tokens[0].to_string() == "u64"
}

/// Returns `i64` for u64 types, otherwise the original type.
///
/// Counterpart: `godot_codegen::generator::u64_handling::substitute_return_type`.
pub(crate) fn substitute_return_type(return_type: &TokenStream, is_u64: bool) -> TokenStream {
    if is_u64 {
        quote! { i64 }
    } else {
        return_type.clone()
    }
}

/// Returns `as i64` cast for u64 types, empty otherwise. 
/// 
/// Counterpart: `godot_codegen::generator::u64_handling::maybe_return_cast`.
pub(crate) fn maybe_return_cast(is_u64: bool) -> TokenStream {
    if is_u64 {
        quote! { as i64 }
    } else {
        TokenStream::new()
    }
}
