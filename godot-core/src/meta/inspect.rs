/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Introspection metadata for Godot engine types.

/// Metadata for a single enum or bitfield constant.
///
/// Returned by [`EngineEnum::all_constants()`][crate::obj::EngineEnum::all_constants] and
/// [`EngineBitfield::all_constants()`][crate::obj::EngineBitfield::all_constants].
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct EnumConstant<T: Copy + 'static> {
    rust_name: &'static str,
    godot_name: &'static str,
    value: T,
}

impl<T> EnumConstant<T>
where
    T: Copy + Eq + PartialEq + 'static,
{
    /// Creates a new enum constant metadata entry.
    pub(crate) const fn new(rust_name: &'static str, godot_name: &'static str, value: T) -> Self {
        Self {
            rust_name,
            godot_name,
            value,
        }
    }

    /// Rust name of the constant, usually without prefix (e.g. `"ESCAPE"` for `Key::ESCAPE`).
    ///
    /// For enums, this is the value returned by [`EngineEnum::as_str()`](crate::obj::EngineEnum::as_str()) **if the value is unique.**
    /// If multiple enum values share the same ordinal, then this function will return each one separately, while `as_str()` will return the
    /// first one.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Godot constant name (e.g. `"KEY_ESCAPE"` for `Key::ESCAPE`).
    pub const fn godot_name(&self) -> &'static str {
        self.godot_name
    }

    /// The Rust value itself.
    ///
    /// Use `value().ord()` to get the ordinal value.
    pub const fn value(&self) -> T {
        self.value
    }
}
