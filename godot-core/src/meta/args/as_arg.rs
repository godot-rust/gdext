/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ffi::CStr;

use crate::builtin::{GString, NodePath, StringName, Variant};
use crate::meta::sealed::Sealed;
use crate::meta::traits::GodotFfiVariant;
use crate::meta::{CowArg, GodotType, ToGodot};

/// Implicit conversions for arguments passed to Godot APIs.
///
/// An `impl AsArg<T>` parameter allows values to be passed which can be represented in the target type `T`. Note that unlike `From<T>`,
/// this trait is implemented more conservatively.
///
/// As a result, `AsArg<T>` is currently only implemented for certain argument types:
/// - `T` for **by-value** built-ins: `i32`, `bool`, `Vector3`, `Transform2D`...
///   - These all implement `ToGodot<Pass = ByValue>` and typically also `Copy`.
/// - `&T` for **by-ref** built-ins: `GString`, `Array`, `Dictionary`, `PackedArray`, `Variant`...
///   - These all implement `ToGodot<Pass = ByRef>`.
/// - `&str`, `&String` additionally for string types `GString`, `StringName`, `NodePath`.
///
/// See also the [`AsObjectArg`][crate::meta::AsObjectArg] trait which is specialized for object arguments. It may be merged with `AsArg`
/// in the future.
///
/// # Owned values vs. references
/// Implicitly converting from `T` for **by-ref** built-ins is explicitly not supported, i.e. you need to pass `&variant` instead of `variant`.
/// This emphasizes that there is no need to consume the object, thus discourages unnecessary cloning. Similarly, you cannot pass by-value
/// types like `i32` by reference.
///
/// Sometimes, you need exactly that for generic programming though: consistently pass `T` or `&T`. For this purpose, the global functions
/// [`owned_into_arg()`] and [`ref_to_arg()`] are provided.
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
/// The blanket implementations of `AsArg` for `T` (in case of `Pass = ByValue`) and `&T` (`Pass = ByRef`) should readily enable most use
/// cases, as long as your type already supports `ToGodot`. In the majority of cases, you'll simply use by-value passing, e.g. for enums.
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

impl<T> AsArg<T> for &T
where
    T: ToGodot<Pass = ByRef>,
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
    T: ToGodot<Pass = ByValue>,
{
    fn into_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r,
    {
        CowArg::Owned(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Public helper functions (T|&T -> AsArg)

/// Generic abstraction over `T` owned values that should be passed as `AsArg<T>`.
///
/// Useful for generic programming: you have owned values, and want the argument conversion to benefit from moving whenever possible.
/// You don't care if the value can truly be moved efficiently, since you don't need the value at the call site anymore.
///
/// Note that the pattern `owned_into_arg(value.clone())` is inefficient -- instead, use [`ref_to_arg(&value)`][ref_to_arg].
///
/// # Example
/// ```
/// use godot::prelude::*;
/// use godot::meta::{ArrayElement, owned_into_arg};
///
/// // Creates random values, e.g. for fuzzing, property-based testing, etc.
/// // Assume global state for simplicity.
/// trait Generator {
///    fn next() -> Self;
/// }
///
/// fn fill_randomly<T>(arr: &mut Array<T>, count: usize)
/// where
///     T: ArrayElement + ToGodot + Generator,
/// {
///     for _ in 0..count {
///         let value = T::next();
///         arr.push(owned_into_arg(value));
///     }
/// }
/// ```
pub fn owned_into_arg<'r, T>(owned_val: T) -> impl AsArg<T> + 'r
where
    T: ToGodot + 'r,
{
    CowArg::Owned(owned_val)
}

/// Generic abstraction over `&T` references that should be passed as `AsArg<T>`.
///
/// Useful for generic programming: you have references, and want the argument conversion to benefit from borrowing whenever possible.
///
/// If you no longer need the value at the call site, consider using [`owned_into_arg(value)`][owned_into_arg] instead.
///
/// # Example
/// ```
/// use godot::prelude::*;
/// use godot::meta::{ArrayElement, ref_to_arg};
///
/// // Could use `impl AsArg<T>` and forward it, but let's demonstrate `&T` here.
/// fn log_and_push<T>(arr: &mut Array<T>, value: &T)
/// where
///     T: ArrayElement + ToGodot + std::fmt::Debug,
/// {
///     println!("Add value: {value:?}");
///     arr.push(ref_to_arg(value));
/// }
/// ```
pub fn ref_to_arg<'r, T>(ref_val: &'r T) -> impl AsArg<T> + 'r
where
    T: ToGodot + 'r,
{
    CowArg::Borrowed(ref_val)
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Internal helper macros (AsArg -> &T|T)

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
    (infer $arg_variable:ident) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_into_owned();
    };
}

#[macro_export]
macro_rules! declare_arg_method {
    ($ ($docs:tt)+ ) => {
        $( $docs )+
        ///
        /// # Generic bounds
        /// The bounds are implementation-defined and may change at any time. Do not use this function in a generic context requiring `T`
        /// -- use the `From` trait or [`AsArg`][crate::meta::AsArg] in that case.
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
// CowArg

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
// Argument passing (mutually exclusive by-val or by-ref).

/// Determines whether arguments are passed by value or by reference to Godot.
///
/// See [`ToGodot::Pass`].
pub trait ArgPassing: Sealed {
    /// Return type: `T` or `&'r T`.
    type Output<'r, T: 'r>;

    /// FFI argument type: `T::Ffi` or `T::ToFfi<'f>`.
    #[doc(hidden)]
    type FfiOutput<'f, T>: GodotFfiVariant
    where
        T: GodotType + 'f;

    /// Convert to owned `T::Via` (cloning if necessary).
    #[doc(hidden)]
    fn ref_to_owned_via<T>(value: &T) -> T::Via
    where
        T: ToGodot<Pass = Self>,
        T::Via: Clone;

    /// Convert to FFI repr in the most efficient way (move or borrow).
    #[doc(hidden)]
    fn ref_to_ffi<T>(value: &T) -> Self::FfiOutput<'_, T::Via>
    where
        T: ToGodot<Pass = Self>,
        T::Via: GodotType;

    /// Convert to `Variant` in the most efficient way (move or borrow).
    #[doc(hidden)]
    fn ref_to_variant<T>(value: &T) -> Variant
    where
        T: ToGodot<Pass = Self>,
    {
        let ffi_result = Self::ref_to_ffi(value);
        GodotFfiVariant::ffi_to_variant(&ffi_result)
    }
}

/// Pass arguments to Godot by value.
///
/// See [`ToGodot::Pass`].
pub enum ByValue {}
impl Sealed for ByValue {}
impl ArgPassing for ByValue {
    type Output<'r, T: 'r> = T;

    type FfiOutput<'a, T>
        = T::Ffi
    where
        T: GodotType + 'a;

    fn ref_to_owned_via<T>(value: &T) -> T::Via
    where
        T: ToGodot<Pass = Self>,
        T::Via: Clone,
    {
        value.to_godot()
    }

    fn ref_to_ffi<T>(value: &T) -> Self::FfiOutput<'_, T::Via>
    where
        T: ToGodot<Pass = Self>,
        T::Via: GodotType,
    {
        // For ByValue: to_godot() returns owned T::Via, move directly to FFI.
        GodotType::into_ffi(value.to_godot())
    }
}

/// Pass arguments to Godot by reference.
///
/// See [`ToGodot::Pass`].
pub enum ByRef {}
impl Sealed for ByRef {}
impl ArgPassing for ByRef {
    type Output<'r, T: 'r> = &'r T;

    type FfiOutput<'f, T>
        = T::ToFfi<'f>
    where
        T: GodotType + 'f;

    fn ref_to_owned_via<T>(value: &T) -> T::Via
    where
        T: ToGodot<Pass = Self>,
        T::Via: Clone,
    {
        // For ByRef types, clone the reference to get owned value.
        value.to_godot().clone()
    }

    fn ref_to_ffi<T>(value: &T) -> <T::Via as GodotType>::ToFfi<'_>
    where
        T: ToGodot<Pass = Self>,
        T::Via: GodotType,
    {
        // Use by-ref conversion if possible, avoiding unnecessary clones when passing to FFI.
        value.to_godot().to_ffi()
    }
}

#[doc(hidden)] // Easier for internal use.
pub type ToArg<'r, Via, Pass> = <Pass as ArgPassing>::Output<'r, Via>;
