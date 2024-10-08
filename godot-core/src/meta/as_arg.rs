/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, NodePath, StringName};
use crate::meta::{CowArg, ToGodot};
use std::ffi::CStr;

/// Shorthand to determine how a type is passed as an argument to Godot APIs.
pub type Arg<'r, T> = <T as AsArg<T>>::ArgType<'r>;

/// Implicit conversions for arguments passed to Godot APIs.
///
/// An `impl AsArg<T>` parameter allows values to be passed which can be represented in the target type `T`. Note that unlike `From<T>`,
/// this trait is implemented more conservatively.
///
/// # Performance for strings
/// Godot has three string types: [`GString`], [`StringName`] and [`NodePath`]. Conversions between those three, as well as between `String` and
/// them, is generally expensive because of allocations, re-encoding, validations, hashing, etc. While this doesn't matter for a few strings
/// passed to engine APIs, it can become a problematic when passing long strings in a hot loop.
///
/// As a result, `AsArg<T>` is currently only implemented for certain conversions:
/// - `&T` and `&mut T`, since those require no conversion or copy.
/// - String literals like `"string"` and `c"string"`. While these _do_ need conversions, those are quite explicit, and
///   `&'static CStr -> StringName` in particular is cheap.
#[diagnostic::on_unimplemented(
    message = "The provided argument of type `{Self}` cannot be implicitly converted to a `{T}` parameter",
    note = "GString/StringName aren't implicitly convertible for performance reasons; use their dedicated `to_*` conversion methods.",
    note = "See also `AsArg` docs: https://godot-rust.github.io/docs/gdext/master/godot/meta/trait.AsArg.html"
)]
pub trait AsArg<T: ToGodot>
where
    Self: Sized,
{
    /// Target type, either `T` or `&T`.
    ///
    /// The general rule is that `Copy` types are passed by value, while the rest is passed by reference. The type alias [`Arg<T>`] is a
    /// shorthand for `<T as AsArg<T>>::Type`.
    ///
    /// This associated may be merged with [`ToGodot::ToVia<'v>`] in the future.
    type ArgType<'v>
    //: GodotType
    where
        Self: 'v;

    #[doc(hidden)]
    fn as_arg(&self) -> Self::ArgType<'_>;

    #[doc(hidden)]
    fn consume_arg<'r>(self) -> CowArg<'r, T>
    where
        Self: 'r,
    {
        panic!("Direct call by user is an error; this is a private function. Overridden where necessary.")
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls

#[macro_export]
macro_rules! arg_into_ref {
    ($arg_variable:ident) => {
        let $arg_variable = $arg_variable.consume_arg();
        let $arg_variable = $arg_variable.as_ref();
    };
}

#[macro_export]
macro_rules! impl_asarg_by_value {
    ($T:ty) => {
        impl AsArg<$T> for $T {
            type ArgType<'a> = Self;

            fn as_arg(&self) -> Self::ArgType<'_> {
                // Require Copy.
                *self
            }
        }
    };
}

#[macro_export]
macro_rules! impl_asarg_by_ref {
    ($T:ty) => {
        impl<'a> AsArg<$T> for &'a $T {
            type ArgType<'v> = &'v $T
                where Self: 'v;

            fn as_arg(&self) -> Self::ArgType<'_> {
                self
            }
        }
    };
}

// impl_asarg_for_references!(GString);
// impl_asarg_for_references!(NodePath);
// impl_asarg_for_references!(StringName);

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GString

impl AsArg<GString> for &str {
    type ArgType<'v> = GString
    where Self: 'v;

    fn as_arg(&self) -> GString {
        GString::from(*self)
    }
}

impl AsArg<GString> for GString {
    type ArgType<'v> = GString;

    fn as_arg(&self) -> GString {
        self.clone()
    }

    fn consume_arg<'r>(self) -> CowArg<'r, GString>
    where
        Self: 'r,
    {
        CowArg::Owned(self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// StringName

impl AsArg<StringName> for &str {
    type ArgType<'v> = StringName
        where Self: 'v;

    fn as_arg(&self) -> StringName {
        StringName::from(*self)
    }
}

impl AsArg<StringName> for &'static CStr {
    type ArgType<'v> = StringName;

    fn as_arg(&self) -> StringName {
        StringName::from(*self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// NodePath

impl AsArg<NodePath> for &str {
    type ArgType<'v> = NodePath
        where Self: 'v;

    fn as_arg(&self) -> NodePath {
        NodePath::from(*self)
    }
}
