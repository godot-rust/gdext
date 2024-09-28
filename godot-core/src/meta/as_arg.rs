/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::builtin::{GString, NodePath, StringName};
use crate::meta::ToGodot;
use std::ffi::CStr;

#[diagnostic::on_unimplemented(
    message = "The provided argument of type `{Self}` cannot be implicitly converted to a `{T}` parameter",
    note = "GString/StringName aren't implicitly convertible for performance reasons; use their dedicated `to_*` conversion methods."
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
