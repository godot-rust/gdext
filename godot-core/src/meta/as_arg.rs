/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, NodePath, StringName};
use crate::meta::ToGodot;
use std::ffi::CStr;

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
pub trait AsArg<T: ToGodot> {
    fn as_arg(&self) -> T::ToVia<'_>;
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Blanket impls

impl<'a, T: ToGodot> AsArg<T> for &'a T {
    fn as_arg(&self) -> T::ToVia<'_> {
        self.to_godot()
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// GString

impl AsArg<GString> for &str {
    fn as_arg(&self) -> GString {
        GString::from(*self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// StringName

impl AsArg<StringName> for &str {
    fn as_arg(&self) -> StringName {
        StringName::from(*self)
    }
}

impl AsArg<StringName> for &'static CStr {
    fn as_arg(&self) -> StringName {
        StringName::from(*self)
    }
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// NodePath

impl AsArg<NodePath> for &str {
    fn as_arg(&self) -> NodePath {
        NodePath::from(*self)
    }
}
