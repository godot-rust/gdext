/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::Variant;

use super::{CallContext, CallResult, PropertyInfo};
use godot_ffi as sys;

mod impls;

/// Represents a parameter list as Rust tuple.
///
/// Each tuple element is one parameter.
pub trait ParamTuple: Sized {
    const LEN: usize;

    fn property_info(index: usize, param_name: &str) -> PropertyInfo;
    fn param_info(
        index: usize,
        param_name: &str,
    ) -> Option<crate::registry::method::MethodParamOrReturnInfo>;
    fn format_args(&self) -> String;
}

pub trait InParamTuple: ParamTuple {
    unsafe fn from_varcall_args(
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        call_ctx: &CallContext,
    ) -> CallResult<Self>;

    unsafe fn from_ptrcall_args(
        args_ptr: *const sys::GDExtensionConstTypePtr,
        call_type: sys::PtrcallType,
        call_ctx: &CallContext,
    ) -> Self;

    fn from_variant_array(array: &[&Variant]) -> Self;
}

pub trait OutParamTuple: ParamTuple {
    fn with_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[Variant], &[sys::GDExtensionConstVariantPtr]) -> R;

    fn with_ptr_args<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[sys::GDExtensionConstTypePtr]) -> R;

    fn to_variant_array(&self) -> Vec<Variant>;
}
