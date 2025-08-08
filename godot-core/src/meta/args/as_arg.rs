/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::CStr;

use crate::builtin::{GString, NodePath, StringName};
use crate::meta::sealed::Sealed;
use crate::meta::{CowArg, ToGodot};

/// Implicit conversions for arguments passed to Godot APIs.
///
/// An `impl AsArg<T>` parameter allows values to be passed which can be represented in the target type `T`. Note that unlike `From<T>`,
/// this trait is implemented more conservatively.
///
/// As a result, `AsArg<T>` is currently only implemented for certain argument types:
/// - `T` for by-value built-ins (typically `Copy`): `i32`, `bool`, `Vector3`, `Transform2D`, ...
/// - `&T` for by-ref built-ins: `GString`, `Array`, `Dictionary`, `Packed*Array`, `Variant`...
/// - `&str`, `&String` additionally for string types `GString`, `StringName`, `NodePath`.
///
/// See also the [`AsObjectArg`][crate::meta::AsObjectArg] trait which is specialized for object arguments. It may be merged with `AsArg`
/// in the future.
///
/// # Pass by value
/// Implicitly converting from `T` for by-ref built-ins is explicitly not supported. This emphasizes that there is no need to consume the object,
/// thus discourages unnecessary cloning.
///
/// # Performance for strings
/// Godot has three string types: [`GString`], [`StringName`] and [`NodePath`]. Conversions between those three, as well as between `String` and
/// them, is generally expensive because of allocations, re-encoding, validations, hashing, etc. While this doesn't matter for a few strings
/// passed to engine APIs, it can become a problematic when passing long strings in a hot loop.
///
/// In the case of strings, we allow implicit conversion from Rust types `&str`, `&String` and `&'static CStr` (`StringName` only).
/// While these conversions are not free, those are either explicit because a string literal is used, or they are unsurprising, because Godot
/// cannot deal with raw Rust types. On the other hand, `GString` and `StringName` are sometimes used almost interchangeably (example:
/// [`Node::set_name`](crate::classes::Node::set_name) takes `GString` but [`Node::get_name`](crate::classes::Node::get_name) returns `StringName`).
///
/// If you want to convert between Godot's string types for the sake of argument passing, each type provides an `arg()` method, such as
/// [`GString::arg()`]. You cannot use this method in other contexts.
///
/// # Using the trait
/// `AsArg` is meant to be used from the function call site, not the declaration site. If you declare a parameter as `impl AsArg<...>` yourself,
/// you can only forward it as-is to a Godot API -- there are no stable APIs to access the inner object yet.
///
/// If you want to pass your own types to a Godot API i.e. to emit a signal, you should implement the [`ParamType`] trait.
#[diagnostic::on_unimplemented(
    message = "Argument of type `{Self}` cannot be passed to an `impl AsArg<{T}>` parameter",
    note = "if you pass by value, consider borrowing instead.",
    note = "GString/StringName/NodePath aren't implicitly convertible for performance reasons; use their `arg()` method.",
    note = "see also `AsArg` docs: https://godot-rust.github.io/docs/gdext/master/godot/meta/trait.AsArg.html"
)]
pub trait AsArg<T: ToGodot>
where
    Self: Sized,
{
    // The usage of the CowArg return type introduces a small runtime penalty for values that implement Copy. Currently, the usage
    // ergonomics out weigh the runtime cost. Using the CowArg allows us to create a blanket implementation of the trait for all types that
    // implement ToGodot.
    #[doc(hidden)]
    fn into_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r;
}

/// Generic abstraction over `T` and `&T` that should be passed as `AsArg<T>`.
#[doc(hidden)]
pub fn val_into_arg<'r, T>(arg: T) -> impl AsArg<T> + 'r
where
    T: ToGodot + 'r,
{
    CowArg::Owned(arg)
}

impl<T> AsArg<T> for &T
where
    T: ToGodot + ParamType<ArgPassing = ByRef>,
{
    fn into_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r,
    {
        CowArg::Borrowed(self)
    }
}

impl<T> AsArg<T> for T
where
    T: ToGodot + ParamType<ArgPassing = ByValue>,
{
    fn into_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r,
    {
        CowArg::Owned(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls

/// Converts `impl AsArg<T>` into a locally valid `&T`.
///
/// This cannot be done via function, since an intermediate variable (the Cow) is needed, which would go out of scope
/// once the reference is returned. Could use more fancy syntax like `arg_into_ref! { let path = ref; }` or `let path = arg_into_ref!(path)`,
/// but still isn't obvious enough to avoid doc lookup and might give a wrong idea about the scope. So being more exotic is a feature.
#[macro_export]
#[doc(hidden)] // Doesn't work at re-export.
macro_rules! arg_into_ref {
    ($arg_variable:ident) => {
        // Non-generic version allows type inference. Only applicable for CowArg types.
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_as_ref();
    };
    ($arg_variable:ident: $T:ty) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable: &$T = $arg_variable.cow_as_ref();
    };
}

/// Converts `impl AsArg<T>` into a locally valid `T`.
///
/// A macro for consistency with [`arg_into_ref`][crate::arg_into_ref].
#[macro_export]
#[doc(hidden)] // Doesn't work at re-export.
macro_rules! arg_into_owned {
    ($arg_variable:ident) => {
        // Non-generic version allows type inference. Only applicable for CowArg types.
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_into_owned();
    };
    ($arg_variable:ident: $T:ty) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable: $T = $crate::meta::ParamType::arg_into_owned($arg_variable);
    };
    (infer $arg_variable:ident) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_into_owned();
    };
}

#[macro_export]
macro_rules! impl_asarg_by_value {
    ($T:ty) => {
        impl $crate::meta::ParamType for $T {
            type ArgPassing = $crate::meta::ByValue;
        }
    };
}

#[macro_export]
macro_rules! impl_asarg_by_ref {
    ($T:ty) => {
        impl $crate::meta::ParamType for $T {
            type ArgPassing = $crate::meta::ByRef;
        }
    };
}

#[macro_export]
macro_rules! declare_arg_method {
    ($ ($docs:tt)+ ) => {
        $( $docs )+
        ///
        /// # Generic bounds
        /// The bounds are implementation-defined and may change at any time. Do not use this function in a generic context requiring `T`
        /// -- use the `From` trait or [`ParamType`][crate::meta::ParamType] in that case.
        pub fn arg<T>(&self) -> impl $crate::meta::AsArg<T>
        where
            for<'a> T: From<&'a Self>
                + $crate::meta::ToGodot
                + 'a,
        {
            $crate::meta::CowArg::Owned(T::from(self))
        }
    };
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls

/// `CowArg` can itself be passed as an argument (internal only).
///
/// Allows forwarding of `impl AsArg<T>` arguments to both another signature of `impl AsArg<T>` and signature of `T` for `Copy` types.
/// This is necessary for packed array dispatching to different "inner" backend signatures.
impl<T> AsArg<T> for CowArg<'_, T>
where
    for<'r> T: ToGodot,
{
    fn into_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r,
    {
        self
    }
}

// impl<'a, T> ParamType for CowArg<'a, T> {
//     type Type<'v> = CowArg<'v, T>
//         where Self: 'v;
// }

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GString

// Note: for all string types S, `impl AsArg<S> for &mut String` is not yet provided, but we can add them if needed.

impl AsArg<GString> for &str {
    fn into_arg<'r>(self) -> CowArg<'r, GString> {
        CowArg::Owned(GString::from(self))
    }
}

impl AsArg<GString> for &String {
    fn into_arg<'r>(self) -> CowArg<'r, GString> {
        CowArg::Owned(GString::from(self))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// StringName

impl AsArg<StringName> for &str {
    fn into_arg<'r>(self) -> CowArg<'r, StringName> {
        CowArg::Owned(StringName::from(self))
    }
}

impl AsArg<StringName> for &String {
    fn into_arg<'r>(self) -> CowArg<'r, StringName> {
        CowArg::Owned(StringName::from(self))
    }
}

#[cfg(since_api = "4.2")]
impl AsArg<StringName> for &'static CStr {
    fn into_arg<'r>(self) -> CowArg<'r, StringName> {
        CowArg::Owned(StringName::from(self))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// NodePath

impl AsArg<NodePath> for &str {
    fn into_arg<'r>(self) -> CowArg<'r, NodePath> {
        CowArg::Owned(NodePath::from(self))
    }
}

impl AsArg<NodePath> for &String {
    fn into_arg<'r>(self) -> CowArg<'r, NodePath> {
        CowArg::Owned(NodePath::from(self))
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------

/// Implemented for all parameter types `T` that are allowed to receive [impl `AsArg<T>`][AsArg].
///
/// **Deprecated**: This trait is considered deprecated and will be removed in 0.4. It is still required to be implemented by types that should
/// be passed `AsArg` in the current version, though.
//
// ParamType used to be a subtrait of GodotType, but this can be too restrictive. For example, DynGd is not a "Godot canonical type"
// (GodotType), however it's still useful to store it in arrays -- which requires AsArg and subsequently ParamType.
//
// TODO(v0.4): merge ParamType::ArgPassing into ToGodot::ToVia, reducing redundancy on user side.
pub trait ParamType: ToGodot + Sized + 'static
// GodotType bound not required right now, but conceptually should always be the case.
{
    type ArgPassing: ArgPassing;

    #[deprecated(
        since = "0.3.2",
        note = "This method is no longer needed and will be removed in 0.4"
    )]
    fn owned_to_arg(self) -> impl AsArg<Self> {
        val_into_arg(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Argument passing (mutually exclusive by-val or by-ref).

pub trait ArgPassing: Sealed {}

pub enum ByValue {}
impl ArgPassing for ByValue {}
impl Sealed for ByValue {}

pub enum ByRef {}
impl ArgPassing for ByRef {}
impl Sealed for ByRef {}
