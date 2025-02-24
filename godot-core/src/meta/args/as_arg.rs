/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, NodePath, StringName};
use crate::meta::{sealed, CowArg};
use std::ffi::CStr;

/// Implicit conversions for arguments passed to Godot APIs.
///
/// An `impl AsArg<T>` parameter allows values to be passed which can be represented in the target type `T`. Note that unlike `From<T>`,
/// this trait is implemented more conservatively.
///
/// As a result, `AsArg<T>` is currently only implemented for certain argument types:
/// - `T` for by-value builtins (typically `Copy`): `i32`, `bool`, `Vector3`, `Transform2D`, ...
/// - `&T` for by-ref builtins: `GString`, `Array`, `Dictionary`, `Packed*Array`, `Variant`...
/// - `&str`, `&String` additionally for string types `GString`, `StringName`, `NodePath`.
///
/// See also the [`AsObjectArg`][crate::meta::AsObjectArg] trait which is specialized for object arguments. It may be merged with `AsArg`
/// in the future.
///
/// # Pass by value
/// Implicitly converting from `T` for by-ref builtins is explicitly not supported. This emphasizes that there is no need to consume the object,
/// thus discourages unnecessary cloning.
///
/// If you need to pass owned values in generic code, you can use [`ParamType::owned_to_arg()`].
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
/// Furthermore, there is currently no benefit in implementing `AsArg` for your own types, as it's only used by Godot APIs which don't accept
/// custom types. Classes are already supported through upcasting and [`AsObjectArg`][crate::meta::AsObjectArg].
#[diagnostic::on_unimplemented(
    message = "Argument of type `{Self}` cannot be passed to an `impl AsArg<{T}>` parameter",
    note = "If you pass by value, consider borrowing instead.",
    note = "GString/StringName/NodePath aren't implicitly convertible for performance reasons; use their `arg()` method.",
    note = "See also `AsArg` docs: https://godot-rust.github.io/docs/gdext/master/godot/meta/trait.AsArg.html"
)]
pub trait AsArg<T: ParamType>
where
    Self: Sized,
{
    #[doc(hidden)]
    fn into_arg<'r>(self) -> <T as ParamType>::Arg<'r>
    where
        Self: 'r;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls

/// Converts `impl AsArg<T>` into a locally valid `&T`.
///
/// This cannot be done via function, since an intermediate variable (the Cow) is needed, which would go out of scope
/// once the reference is returned. Could use more fancy syntax like `arg_into_ref! { let path = ref; }` or `let path = arg_into_ref!(path)`,
/// but still isn't obvious enough to avoid doc lookup and might give a wrong idea about the scope. So being more exotic is a feature.
#[macro_export]
macro_rules! arg_into_ref {
    ($arg_variable:ident) => {
        // Non-generic version allows type inference. Only applicable for CowArg types.
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_as_ref();
    };
    ($arg_variable:ident: $T:ty) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable: &$T = $crate::meta::ParamType::arg_to_ref(&$arg_variable);
    };
}

/// Converts `impl AsArg<T>` into a locally valid `T`.
///
/// A macro for consistency with [`arg_into_ref`][crate::arg_into_ref].
#[macro_export]
macro_rules! arg_into_owned {
    ($arg_variable:ident) => {
        let $arg_variable = $arg_variable.into_arg();
        let $arg_variable = $arg_variable.cow_into_owned();
        // cow_into_owned() is not yet used generically; could be abstracted in ParamType::arg_to_owned() as well.
    };
}

#[macro_export]
macro_rules! impl_asarg_by_value {
    ($T:ty) => {
        impl $crate::meta::AsArg<$T> for $T {
            fn into_arg<'r>(self) -> <$T as $crate::meta::ParamType>::Arg<'r> {
                // Moves value (but typically a Copy type).
                self
            }
        }

        impl $crate::meta::ParamType for $T {
            type Arg<'v> = $T;

            fn owned_to_arg<'v>(self) -> Self::Arg<'v> {
                self
            }

            fn arg_to_ref<'r>(arg: &'r Self::Arg<'_>) -> &'r Self {
                arg
            }
        }
    };
}

#[macro_export]
macro_rules! impl_asarg_by_ref {
    ($T:ty) => {
        impl<'r> $crate::meta::AsArg<$T> for &'r $T {
            // 1 rustfmt + 1 rustc problems (bugs?) here:
            // - formatting doesn't converge; `where` keeps being further indented on each run.
            // - a #[rustfmt::skip] annotation over the macro causes a compile error when mentioning `crate::impl_asarg_by_ref`.
            //   "macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths"
            // Thus, keep `where` on same line.
            // type ArgType<'v> = &'v $T where Self: 'v;

            fn into_arg<'cow>(self) -> <$T as $crate::meta::ParamType>::Arg<'cow>
            where
                'r: 'cow, // Original reference must be valid for at least as long as the returned cow.
            {
                $crate::meta::CowArg::Borrowed(self)
            }
        }

        impl $crate::meta::ParamType for $T {
            type Arg<'v> = $crate::meta::CowArg<'v, $T>;

            fn owned_to_arg<'v>(self) -> Self::Arg<'v> {
                $crate::meta::CowArg::Owned(self)
            }

            fn arg_to_ref<'r>(arg: &'r Self::Arg<'_>) -> &'r Self {
                arg.cow_as_ref()
            }
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
                + $crate::meta::ParamType<Arg<'a> = $crate::meta::CowArg<'a, T>>
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
    for<'r> T: ParamType<Arg<'r> = CowArg<'r, T>> + 'r,
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

#[cfg(since_api = "4.2")] #[cfg_attr(published_docs, doc(cfg(since_api = "4.2")))]
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
// ParamType used to be a subtrait of GodotType, but this can be too restrictive. For example, DynGd is not a "Godot canonical type"
// (GodotType), however it's still useful to store it in arrays -- which requires AsArg and subsequently ParamType.
pub trait ParamType: sealed::Sealed + Sized + 'static
// GodotType bound not required right now, but conceptually should always be the case.
{
    /// Canonical argument passing type, either `T` or an internally-used CoW type.
    ///
    /// The general rule is that `Copy` types are passed by value, while the rest is passed by reference.
    ///
    /// This associated type is closely related to [`ToGodot::ToVia<'v>`][crate::meta::ToGodot::ToVia] and may be reorganized in the future.
    #[doc(hidden)]
    type Arg<'v>: AsArg<Self>
    where
        Self: 'v;

    /// Converts an owned value to the canonical argument type, which can be passed to [`impl AsArg<T>`][AsArg].
    ///
    /// Useful in generic contexts where only a value is available, and one doesn't want to dispatch between value/reference.
    ///
    /// You should not rely on the exact return type, as it may change in future versions; treat it like `impl AsArg<Self>`.
    fn owned_to_arg<'v>(self) -> Self::Arg<'v>;

    /// Converts an argument to a shared reference.
    ///
    /// Useful in generic contexts where you need to extract a reference of an argument, independently of how it is passed.
    #[doc(hidden)] // for now, users are encouraged to use only call-site of impl AsArg; declaration-site may still develop.
    fn arg_to_ref<'r>(arg: &'r Self::Arg<'_>) -> &'r Self;
}
