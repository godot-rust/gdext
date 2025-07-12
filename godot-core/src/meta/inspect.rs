/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Introspection metadata for Godot engine types.

/// Metadata for a single enum constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumConstant<T: Copy + 'static> {
    rust_name: &'static str,
    godot_name: &'static str,
    ord: i32,
    value: T,
}

impl<T: Copy + 'static> EnumConstant<T> {
    /// Creates a new enum constant metadata entry.
    pub(crate) const fn new(
        rust_name: &'static str,
        godot_name: &'static str,
        ord: i32,
        value: T,
    ) -> Self {
        Self {
            rust_name,
            godot_name,
            ord,
            value,
        }
    }

    /// Rust name of the enum variant, usually without Prefix (e.g. `"ESCAPE"` for `Key::ESCAPE`).
    ///
    /// This is returned by [`EngineEnum::as_str()`](crate::obj::EngineEnum::as_str()).
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Godot constant name (e.g. `"KEY_ESCAPE"` for `Key::ESCAPE`).
    pub const fn godot_name(&self) -> &'static str {
        self.godot_name
    }

    /// Ordinal value of this enum variant.
    pub const fn ord(&self) -> i32 {
        self.ord
    }

    /// The enum value itself.
    pub const fn value(&self) -> T {
        self.value
    }
}
