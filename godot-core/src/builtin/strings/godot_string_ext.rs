/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::borrow::Cow;

use godot_ffi as sys;
use sys::GodotFfi;

use crate::builtin::{GString, NodePath, StringName};

/// Extension trait for converting to Godot string types.
///
/// Provides types like `&str` and `String` with "extension functions" to convert to one of the Godot string types.
///
/// An explicitly named `val.to_gstring()` method has several advantages over the `From` trait (which may be phased out over time):
/// - Compared to `GString::from(&val)`, it is shorter and can be used in fluent expressions such as `val.replace(...).to_gstring()`.
/// - Compared to `val.into()`, it makes the conversion more explicit by naming the type, and never runs into type-inference problems
///   (the `into()` function requires an explicit type declaration somewhere).
pub trait GodotStringExt {
    /// Convert to a `GString`.
    fn to_gstring(&self) -> GString;

    /// Convert to a `StringName`.
    fn to_string_name(&self) -> StringName;

    /// Convert to a `NodePath`.
    ///
    /// Since `NodePath` doesn't allow many direct conversions, a default implementation converts via `GString`.
    /// Override if you have a more performant implementation.
    fn to_node_path(&self) -> NodePath {
        self.to_gstring().to_node_path()
    }
}

/// Macro to delegate a builtin conversion to the FFI layer.
///
/// Use: `let dest = unsafe_from_builtin!(dest_from_source -> Dest, source);`
///
/// # Safety
/// The `Dest` type must match what the FFI function `dest_from_source` returns.
macro_rules! unsafe_builtin_convert {
    ($ctor:ident -> $Type:ty, $arg:expr) => {{
        let __arg = $arg; // Must outlive the sys() call.

        // SAFETY: Matching $Type and $ctor is caller responsibility. The rest adheres to Godot FFI for conversion constructors.
        unsafe {
            <$Type>::new_with_uninit(|self_ptr| {
                let ctor = sys::builtin_fn!($ctor);
                let args = [__arg.sys()];
                ctor(self_ptr, args.as_ptr());
            })
        }
    }};
}

impl GodotStringExt for GString {
    fn to_gstring(&self) -> GString {
        self.clone()
    }

    fn to_string_name(&self) -> StringName {
        unsafe_builtin_convert!(string_name_from_string -> StringName, self)
    }

    fn to_node_path(&self) -> NodePath {
        unsafe_builtin_convert!(node_path_from_string -> NodePath, self)
    }
}

impl GodotStringExt for StringName {
    fn to_gstring(&self) -> GString {
        unsafe_builtin_convert!(string_from_string_name -> GString, self)
    }

    fn to_string_name(&self) -> StringName {
        self.clone()
    }
}

impl GodotStringExt for NodePath {
    fn to_gstring(&self) -> GString {
        unsafe_builtin_convert!(string_from_node_path -> GString, self)
    }

    fn to_string_name(&self) -> StringName {
        // Godot FFI provides no direct NodePath -> StringName conversion.
        self.to_gstring().to_string_name()
    }

    fn to_node_path(&self) -> NodePath {
        self.clone()
    }
}

impl GodotStringExt for &str {
    fn to_gstring(&self) -> GString {
        let utf8_bytes = self.as_bytes();

        // SAFETY: string is UTF-8 encoded and len-bounded. Godot takes byte length, not character count.
        unsafe {
            GString::new_with_string_uninit(|string_ptr| {
                #[cfg(before_api = "4.3")]
                let ctor = sys::interface_fn!(string_new_with_utf8_chars_and_len);
                #[cfg(since_api = "4.3")]
                let ctor = sys::interface_fn!(string_new_with_utf8_chars_and_len2);

                ctor(
                    string_ptr,
                    utf8_bytes.as_ptr().cast::<std::ffi::c_char>(),
                    utf8_bytes.len() as i64,
                );
            })
        }
    }

    fn to_string_name(&self) -> StringName {
        let utf8_bytes = self.as_bytes();

        // SAFETY: string is UTF-8 encoded and len-bounded. Godot takes byte length, not character count.
        unsafe {
            StringName::new_with_string_uninit(|string_ptr| {
                sys::interface_fn!(string_name_new_with_utf8_chars_and_len)(
                    string_ptr,
                    utf8_bytes.as_ptr().cast::<std::ffi::c_char>(),
                    utf8_bytes.len() as i64,
                );
            })
        }
    }
}

impl GodotStringExt for &[char] {
    fn to_gstring(&self) -> GString {
        let chars = *self;

        // SAFETY: A `char` value is by definition a valid Unicode code point. Bounded by len().
        unsafe {
            GString::new_with_string_uninit(|string_ptr| {
                sys::interface_fn!(string_new_with_utf32_chars_and_len)(
                    string_ptr,
                    chars.as_ptr().cast::<sys::char32_t>(),
                    chars.len() as i64,
                );
            })
        }
    }

    fn to_string_name(&self) -> StringName {
        // There is no FFI function `string_name_new_with_utf32_chars_and_len` -> go via GString.
        self.to_gstring().to_string_name()
    }
}

impl GodotStringExt for String {
    fn to_gstring(&self) -> GString {
        self.as_str().to_gstring()
    }

    fn to_string_name(&self) -> StringName {
        self.as_str().to_string_name()
    }
}

impl GodotStringExt for Cow<'_, str> {
    fn to_gstring(&self) -> GString {
        self.as_ref().to_gstring()
    }

    fn to_string_name(&self) -> StringName {
        self.as_ref().to_string_name()
    }
}
