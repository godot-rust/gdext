/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use godot_ffi as sys;

use crate::builtin::Variant;
use crate::meta::error::CallResult;
use crate::meta::{CallContext, PropertyInfo};

mod impls;

/// Represents a parameter list as Rust tuple where each tuple element is one parameter.
///
/// This trait only contains metadata for the parameter list, the actual functionality is contained in [`InParamTuple`] and
/// [`OutParamTuple`].
pub trait ParamTuple: Sized {
    /// The number of elements in this parameter list.
    const LEN: usize;

    /// The param info of the parameter at index `index`.
    #[doc(hidden)]
    fn param_info(
        index: usize,
        param_name: &str,
    ) -> Option<crate::registry::method::MethodParamOrReturnInfo>;

    /// The property info of the parameter at index `index`.
    fn property_info(index: usize, param_name: &str) -> Option<PropertyInfo> {
        Self::param_info(index, param_name).map(|param| param.info)
    }

    /// Return a string representing the arguments.
    fn format_args(&self) -> String;
}

/// Represents a parameter list that is received from some external location (usually Godot).
///
/// As an example, this would be used for user-defined functions that will be called from Godot, however this is _not_ used when
/// calling a Godot function from Rust code.
pub trait InParamTuple: ParamTuple {
    /// Converts `args_ptr` to `Self`, merging with default values if needed.
    ///
    /// # Safety
    ///
    /// - `args_ptr` must be a pointer to an array of length `arg_count`
    /// - Each element of `args_ptr` must be reborrowable as a `&Variant` with a lifetime that lasts for the duration of the call.
    /// - `arg_count + default_values.len()` must equal `Self::LEN`
    #[doc(hidden)]
    unsafe fn from_varcall_args(
        args_ptr: *const sys::GDExtensionConstVariantPtr,
        arg_count: usize,
        default_values: &[Variant],
        call_ctx: &CallContext,
    ) -> CallResult<Self>;

    /// Converts `args_ptr` to `Self` directly.
    ///
    /// # Safety
    ///
    /// - `args_ptr` must be a pointer to a valid array of length [`Self::LEN`](ParamTuple::LEN)
    /// - each element of `args_ptr` must be of the same type as each element of `Self`
    #[doc(hidden)] // Hidden since v0.3.2.
    unsafe fn from_ptrcall_args(
        args_ptr: *const sys::GDExtensionConstTypePtr,
        call_type: sys::PtrcallType,
        call_ctx: &CallContext,
    ) -> CallResult<Self>;

    /// Converts `array` to `Self` by calling [`from_variant`](crate::meta::FromGodot::from_variant) on each argument.
    fn from_variant_array(array: &[&Variant]) -> Self;
}

/// Represents a parameter list that is used to call some external code.
///
/// As an example, this would be used to call Godot functions through FFI, however this is _not_ used when Godot calls a user-defined
/// function.
pub trait OutParamTuple: ParamTuple {
    /// Call `f` on the tuple `self` by first converting `self` to an array of [`Variant`]s.
    fn with_variants<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[Variant]) -> R;

    /// Call `f` on the tuple `self` by first converting `self` to an array of [`Variant`] pointers.
    #[doc(hidden)] // Hidden since v0.3.2.
    fn with_variant_pointers<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[sys::GDExtensionConstVariantPtr]) -> R;

    /// Call `f` on the tuple `self` by first converting `self` to an array of Godot type pointers.
    #[doc(hidden)] // Hidden since v0.3.2.
    fn with_type_pointers<F, R>(self, f: F) -> R
    where
        F: FnOnce(&[sys::GDExtensionConstTypePtr]) -> R;

    /// Converts `array` to `Self` by calling [`to_variant`](crate::meta::ToGodot::to_variant) on each argument.
    fn to_variant_array(&self) -> Vec<Variant>;
}
