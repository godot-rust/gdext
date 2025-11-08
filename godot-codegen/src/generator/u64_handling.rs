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
//! The `godot_macros::class::data_models::u64_handling` module provides the same functionality for proc macros.

use proc_macro2::TokenStream;
use quote::quote;

use crate::models::domain::RustTy;

/// Checks if a type is `u64`, which requires `i64` for FFI.
///
/// Counterpart: `godot_macros::class::data_models::func::is_u64_type`.
pub fn is_u64_type(rust_ty: &RustTy) -> bool {
    matches!(rust_ty, RustTy::BuiltinIdent { ty, .. } if ty == "u64")
}

/// Returns `i64` for `u64` return types, otherwise the original type.
///
/// Used in `type CallRet` declarations where public API uses `u64` but FFI uses `i64`.
///
/// Counterpart: `godot_macros::class::data_models::func::SignatureInfo::substitute_return_type`.
pub fn substitute_return_type(return_type: &TokenStream, rust_ty: Option<&RustTy>) -> TokenStream {
    if rust_ty.is_some_and(is_u64_type) {
        quote! { i64 }
    } else {
        return_type.clone()
    }
}

/// Returns `as i64` cast for `u64` return types, empty otherwise.
///
/// Used to cast FFI return values back to `u64` in the public API.
///
/// Counterpart: `godot_macros::class::data_models::func::SignatureInfo::maybe_return_cast`.
pub fn maybe_return_cast(rust_ty: Option<&RustTy>) -> TokenStream {
    if rust_ty.is_some_and(is_u64_type) {
        quote! { as u64 }
    } else {
        TokenStream::new()
    }
}

/// Returns `as i64` cast for `u64` parameter/value types, empty otherwise.
///
/// Used when passing `u64` parameters to FFI or storing them in fields, where FFI expects `i64`.
pub fn cast_param_value(name: &proc_macro2::Ident, rust_ty: &RustTy) -> Option<TokenStream> {
    if is_u64_type(rust_ty) {
        Some(quote! { #name as i64 })
    } else {
        None
    }
}

/// Returns `i64` substitution for `u64` parameter types, otherwise the original type.
///
/// Used in `type CallParams` declarations and internal parameter declarations
/// where FFI expects `i64` instead of `u64`.
pub fn substitute_param_type(param_ty: &TokenStream, rust_ty: &RustTy) -> TokenStream {
    if is_u64_type(rust_ty) {
        quote! { i64 }
    } else {
        param_ty.clone()
    }
}
